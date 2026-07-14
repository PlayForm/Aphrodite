//! bench_02_threshold - /ccr/create store+retrieve boundary sweep.
//!
//! NOTE (2026-07-13, per .plans/09-testing-quality.md F3): this file
//! originally probed `handle_ccr_create`'s response for size-based
//! "compressed vs passthrough" behavior gated on
//! INLINE_CCR_THRESHOLD/TOKEN_COMPRESS_THRESHOLD/CACHE_COMPRESS_THRESHOLD and
//! per-type multipliers. That premise doesn't hold: `handle_ccr_create`
//! (`crates/aphrodite/src/proxy.rs`) stores unconditionally whenever a CCR
//! backend is configured - there is no threshold gating on this endpoint at
//! all (those thresholds only gate `compress_chat_completion`, the
//! `/v1/chat/completions` response-body compression path, which this bench
//! does not exercise). Every case here was therefore either vacuously true
//! or silently wrong depending on hash length vs content length.
//!
//! Rewritten to test what `/ccr/create` + `/retrieve` actually promise at
//! each size boundary: content is always stored, a hash is always returned,
//! and the stored content is retrievable byte-for-byte - across the same
//! threshold-adjacent sizes the original file swept (256B inline zone, 1KB,
//! 4KB, 8KB boundaries), in both cache and token mode.
//!
//! Exits non-zero on any store/retrieve boundary violation.
//!
//! cargo run --example bench_02_threshold

use std::{
	process::{Command, Stdio},
	time::{Duration, Instant},
};

/// Path to the `aphrodite` binary.
///
/// `CARGO_BIN_EXE_aphrodite` is only set by Cargo for `cargo test`/`cargo
/// bench` harnesses, not for `cargo run --example` (this file's own doc
/// comment says to run it that way) - so it can't be relied on here. Instead,
/// derive the binary path from this example's own executable location:
/// `target/<profile>/examples/bench_02_threshold` and
/// `target/<profile>/aphrodite` are siblings two directories apart.
fn bin_path() -> std::path::PathBuf {
	if let Ok(p) = std::env::var("CARGO_BIN_EXE_aphrodite") {
		return p.into();
	}
	let exe = std::env::current_exe().expect("current_exe");
	let bin_name = if cfg!(windows) { "aphrodite.exe" } else { "aphrodite" };
	exe.parent()
		.and_then(|p| p.parent())
		.map(|p| p.join(bin_name))
		.unwrap_or_else(|| bin_name.into())
}
const CACHE_PORT:u16 = 59797;
const TOKEN_PORT:u16 = 59798;

struct Proxy {
	child:std::process::Child,
	port:u16,
}
impl Drop for Proxy {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}
fn spawn(mode:&str, port:u16) -> Proxy {
	let listen = format!("127.0.0.1:{}", port);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let db_path = std::env::temp_dir().join(format!("aphrodite_bench_02_{}_{}.db", mode, port));
	// SQLite persists the CCR store across process restarts (unlike cache
	// mode's in-memory store) - without this, a leftover file from a
	// previous run would still be inside the TTL window and could poison
	// this run's store/retrieve checks with stale entries.
	let _ = std::fs::remove_file(&db_path);
	let child = Command::new(bin_path())
        .args([
            "--mode",
            mode,
            "--listen",
            &listen,
            "--api-url",
            "http://127.0.0.1:1",
            "--api-key",
            "bench",
            "--ccr-db-path",
        ])
        .arg(&db_path)
        // See bin_path()'s doc comment: a repo-root aphrodite.toml would
        // otherwise silently override --mode/--listen/--ccr-db-path.
        .env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
	let dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
			break;
		}
		assert!(Instant::now() < dl);
		std::thread::sleep(Duration::from_millis(50));
	}
	Proxy { child, port }
}

/// POST /ccr/create - returns the stored hash, if any.
fn ccr_create(port:u16, content:&str) -> Option<String> {
	let body = serde_json::json!({"content": content}).to_string();
	let out = Command::new("curl")
		.args([
			"-s",
			"-X",
			"POST",
			&format!("http://127.0.0.1:{}/ccr/create", port),
			"-H",
			"Content-Type: application/json",
			"-d",
			&body,
		])
		.output()
		.ok()?;
	let v:serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
	v.get("hash").and_then(|h| h.as_str()).map(str::to_string)
}

/// POST /retrieve - true if the hash resolves back to `expected`.
fn ccr_retrieve_matches(port:u16, hash:&str, expected:&str) -> bool {
	let body = serde_json::json!({"hash": hash}).to_string();
	let out = Command::new("curl")
		.args([
			"-s",
			"-X",
			"POST",
			&format!("http://127.0.0.1:{}/retrieve", port),
			"-H",
			"Content-Type: application/json",
			"-d",
			&body,
		])
		.output()
		.ok();
	out.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
		.map(|v| {
			v.get("found").and_then(|f| f.as_bool()).unwrap_or(false)
				&& v.get("content").and_then(|c| c.as_str()) == Some(expected)
		})
		.unwrap_or(false)
}

/// Generate a payload of exactly `size` bytes that triggers `ct` detection.
fn make(ct:&str, size:usize) -> String {
	let unit:&str = match ct {
		"code_rust" => "fn foo() -> u64 { 42 }\n",
		"linter" => "error[E0308]: mismatched types\n  --> src/lib.rs:1:5\n",
		"build_output" => "   Compiling crate v0.1.0\n",
		"error" => "thread 'main' panicked at 'index out of bounds'\n",
		_ => "a",
	};
	let rep = (size / unit.len()).max(1);
	let mut s = unit.repeat(rep);
	// Trim or pad to exact byte size (ASCII-only units, so byte == char)
	match s.len().cmp(&size) {
		std::cmp::Ordering::Greater => s.truncate(size),
		std::cmp::Ordering::Less => s.extend(std::iter::repeat('x').take(size - s.len())),
		std::cmp::Ordering::Equal => {},
	}
	s
}

/// A single probe: store then retrieve; pass iff the round-trip is
/// byte-exact and a hash was actually returned.
fn probe(port:u16, ct:&str, size:usize) -> bool {
	let content = make(ct, size);
	match ccr_create(port, &content) {
		Some(hash) if !hash.is_empty() => ccr_retrieve_matches(port, &hash, &content),
		_ => false,
	}
}

struct Case {
	label:&'static str,
	ct:&'static str,
	size:usize,
	mode:&'static str, // "cache" | "token" | "both"
}

fn cases() -> Vec<Case> {
	vec![
		// ── inline-zone boundary (256 B) ───────────────────────────────────
		Case { label:"inline_below_255", ct:"text", size:255, mode:"both" },
		Case { label:"inline_at_256", ct:"text", size:256, mode:"both" },
		// ── 1 KB boundary ──────────────────────────────────────────────────
		Case { label:"token_below_1023", ct:"text", size:1023, mode:"token" },
		Case { label:"token_at_1024", ct:"text", size:1024, mode:"token" },
		Case { label:"token_above_1025", ct:"text", size:1025, mode:"token" },
		// ── 8 KB boundary ──────────────────────────────────────────────────
		Case { label:"cache_below_8191", ct:"text", size:8191, mode:"cache" },
		Case { label:"cache_at_8192", ct:"text", size:8192, mode:"cache" },
		Case { label:"cache_above_8193", ct:"text", size:8193, mode:"cache" },
		// ── 4 KB boundary, code_rust content ────────────────────────────────
		Case { label:"code_rust_below", ct:"code_rust", size:4095, mode:"token" },
		Case { label:"code_rust_at", ct:"code_rust", size:4096, mode:"token" },
		Case { label:"code_rust_above", ct:"code_rust", size:4097, mode:"token" },
		// ── 512 B boundary, linter/build_output content ─────────────────────
		Case { label:"linter_below", ct:"linter", size:511, mode:"token" },
		Case { label:"linter_at", ct:"linter", size:512, mode:"token" },
		Case { label:"linter_above", ct:"linter", size:513, mode:"token" },
		Case { label:"build_below", ct:"build_output", size:511, mode:"token" },
		Case { label:"build_at", ct:"build_output", size:512, mode:"token" },
		// ── 8 KB boundary, error content ────────────────────────────────────
		Case { label:"error_below_8191", ct:"error", size:8191, mode:"token" },
		Case { label:"error_at_8192", ct:"error", size:8192, mode:"token" },
	]
}

fn main() {
	let cache = spawn("cache", CACHE_PORT);
	let token = spawn("token", TOKEN_PORT);

	let all_cases = cases();
	let mut failures = 0usize;
	let mut total = 0usize;

	eprintln!("\n[bench_02] /ccr/create + /retrieve boundary sweep  {} cases", all_cases.len());
	eprintln!("{:<26} {:>8} {:>8} {:>8}", "label", "mode", "size", "result");
	eprintln!("{}", "─".repeat(55));

	for c in &all_cases {
		let ports:Vec<(u16, &str)> = match c.mode {
			"cache" => vec![(CACHE_PORT, "cache")],
			"token" => vec![(TOKEN_PORT, "token")],
			_ => vec![(CACHE_PORT, "cache"), (TOKEN_PORT, "token")],
		};
		for (port, mode_label) in ports {
			total += 1;
			let pass = probe(port, c.ct, c.size);
			if !pass {
				failures += 1;
			}
			eprintln!(
				"{:<26} {:>8} {:>8} {:>8}",
				c.label,
				mode_label,
				c.size,
				if pass { "PASS" } else { "FAIL ←" }
			);
		}
	}

	eprintln!("\n[bench_02] {}/{} passed", total - failures, total);
	if failures > 0 {
		eprintln!("[bench_02] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy),
		// which would leak the spawned cache/token processes on failure -
		// they'd keep listening on CACHE_PORT/TOKEN_PORT and silently
		// answer (with stale content) the *next* run's bind attempts.
		// Drop them explicitly before exiting.
		drop(cache);
		drop(token);
		std::process::exit(1);
	} else {
		eprintln!("[bench_02] OK");
	}
}
