//! bench_02_threshold - /ccr/create store+retrieve boundary sweep.
//!
//! NOTE (2026-07-13, per .plans/09-testing-quality.md F3): this file
//! originally probed `HandleCCRCreate`'s response for size-based
//! "compressed vs passthrough" behavior gated on
//! INLINE_CCR_THRESHOLD/TOKEN_COMPRESS_THRESHOLD/CACHE_COMPRESS_THRESHOLD and
//! per-type multipliers. That premise doesn't hold: `HandleCCRCreate`
//! (`crates/aphrodite/Source/Proxy/CCR/Create/Handle/Fn.rs`) stores
//! unconditionally whenever a CCR
//! backend is configured - there is no threshold gating on this endpoint at
//! all (those thresholds only gate `CompressChatCompletion`, the
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

/// Path to the `Aphrodite` binary.
///
/// `CARGO_BIN_EXE_aphrodite` is only set by Cargo for `cargo test`/`cargo
/// bench` harnesses, not for `cargo run --example` (this file's own doc
/// comment says to run it that way) - so it can't be relied on here. Instead,
/// derive the binary path from this example's own executable location:
/// `target/<profile>/examples/bench_02_threshold` and
/// `target/<profile>/aphrodite` are siblings two directories apart.
fn BinPath() -> std::path::PathBuf {
	if let Ok(p) = std::env::var("CARGO_BIN_EXE_aphrodite") {
		return p.into();
	}
	let Exe = std::env::current_exe().expect("current_exe");
	let BinName = if cfg!(windows) { "aphrodite.exe" } else { "aphrodite" };
	Exe.parent()
		.and_then(|p| p.parent())
		.map(|p| p.join(BinName))
		.unwrap_or_else(|| BinName.into())
}
const CachePort:u16 = 59797;
const TokenPort:u16 = 59798;

struct Proxy {
	Child:std::process::Child,
	#[allow(dead_code)]
	Port:u16,
}
impl Drop for Proxy {
	fn drop(&mut self) {
		let _ = self.Child.kill();
		let _ = self.Child.wait();
	}
}
fn Spawn(Mode:&str, Port:u16) -> Proxy {
	let Listen = format!("127.0.0.1:{}", Port);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let DatabasePath = std::env::temp_dir().join(format!("aphrodite_bench_02_{}_{}.db", Mode, Port));
	// SQLite persists the CCR store across process restarts (unlike cache
	// mode's in-memory store) - without this, a leftover file from a
	// previous run would still be inside the TTL window and could poison
	// this run's store/retrieve checks with stale entries.
	let _ = std::fs::remove_file(&DatabasePath);
	let Child = Command::new(BinPath())
		.args([
			"--mode",
			Mode,
			"--listen",
			&Listen,
			"--apiurl",
			"http://127.0.0.1:1",
			"--api-key",
			"bench",
			"--ccr-database-path",
		])
		.arg(&DatabasePath)
		// See bin_path()'s doc comment: a repo-root aphrodite.toml would
		// otherwise silently override --mode/--listen/--ccr-db-path.
		.env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.expect("spawn");
	let Dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", Port)).is_ok() {
			break;
		}
		assert!(Instant::now() < Dl);
		std::thread::sleep(Duration::from_millis(50));
	}
	Proxy { Child, Port }
}

/// POST /ccr/create - returns the stored hash, if any.
fn CCRCreate(Port:u16, Content:&str) -> Option<String> {
	let body = serde_json::json!({"content": Content}).to_string();
	let Out = Command::new("curl")
		.args([
			"-s",
			"-X",
			"POST",
			&format!("http://127.0.0.1:{}/ccr/create", Port),
			"-H",
			"Content-Type: application/json",
			"-d",
			&body,
		])
		.output()
		.ok()?;
	let v:serde_json::Value = serde_json::from_slice(&Out.stdout).ok()?;
	v.get("hash").and_then(|h| h.as_str()).map(str::to_string)
}

/// POST /retrieve - true if the hash resolves back to `Expected`.
fn CCRRetrieveMatches(Port:u16, hash:&str, Expected:&str) -> bool {
	let body = serde_json::json!({"hash": hash}).to_string();
	let Out = Command::new("curl")
		.args([
			"-s",
			"-X",
			"POST",
			&format!("http://127.0.0.1:{}/retrieve", Port),
			"-H",
			"Content-Type: application/json",
			"-d",
			&body,
		])
		.output()
		.ok();
	Out.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
		.map(|v| {
			v.get("found").and_then(|f| f.as_bool()).unwrap_or(false)
				&& v.get("content").and_then(|c| c.as_str()) == Some(Expected)
		})
		.unwrap_or(false)
}

/// Generate a payload of exactly `Size` bytes that triggers `Ct` detection.
fn Make(Ct:&str, Size:usize) -> String {
	let Unit:&str = match Ct {
		"code_rust" => "fn foo() -> u64 { 42 }\n",
		"linter" => "error[E0308]: mismatched types\n  --> src/lib.rs:1:5\n",
		"build_output" => "   Compiling crate v0.1.0\n",
		"error" => "thread 'main' panicked at 'index out of bounds'\n",
		_ => "a",
	};
	let Rep = (Size / Unit.len()).max(1);
	let mut s = Unit.repeat(Rep);
	// Trim or pad to exact byte size (ASCII-only units, so byte == char)
	match s.len().cmp(&Size) {
		std::cmp::Ordering::Greater => s.truncate(Size),
		std::cmp::Ordering::Less => s.extend(std::iter::repeat_n('x', Size - s.len())),
		std::cmp::Ordering::Equal => {},
	}
	s
}

/// A single probe: store then retrieve; pass iff the round-trip is
/// byte-exact and a hash was actually returned.
fn Probe(Port:u16, Ct:&str, Size:usize) -> bool {
	let Content = Make(Ct, Size);
	match CCRCreate(Port, &Content) {
		Some(hash) if !hash.is_empty() => CCRRetrieveMatches(Port, &hash, &Content),
		_ => false,
	}
}

struct Case {
	Label:&'static str,
	Ct:&'static str,
	Size:usize,
	Mode:&'static str, // "cache" | "token" | "both"
}

fn Cases() -> Vec<Case> {
	vec![
		// ── inline-zone boundary (256 B) ───────────────────────────────────
		Case { Label:"inline_below_255", Ct:"text", Size:255, Mode:"both" },
		Case { Label:"inline_at_256", Ct:"text", Size:256, Mode:"both" },
		// ── 1 KB boundary ──────────────────────────────────────────────────
		Case { Label:"token_below_1023", Ct:"text", Size:1023, Mode:"token" },
		Case { Label:"token_at_1024", Ct:"text", Size:1024, Mode:"token" },
		Case { Label:"token_above_1025", Ct:"text", Size:1025, Mode:"token" },
		// ── 8 KB boundary ──────────────────────────────────────────────────
		Case { Label:"cache_below_8191", Ct:"text", Size:8191, Mode:"cache" },
		Case { Label:"cache_at_8192", Ct:"text", Size:8192, Mode:"cache" },
		Case { Label:"cache_above_8193", Ct:"text", Size:8193, Mode:"cache" },
		// ── 4 KB boundary, code_rust content ────────────────────────────────
		Case { Label:"code_rust_below", Ct:"code_rust", Size:4095, Mode:"token" },
		Case { Label:"code_rust_at", Ct:"code_rust", Size:4096, Mode:"token" },
		Case { Label:"code_rust_above", Ct:"code_rust", Size:4097, Mode:"token" },
		// ── 512 B boundary, linter/build_output content ─────────────────────
		Case { Label:"linter_below", Ct:"linter", Size:511, Mode:"token" },
		Case { Label:"linter_at", Ct:"linter", Size:512, Mode:"token" },
		Case { Label:"linter_above", Ct:"linter", Size:513, Mode:"token" },
		Case { Label:"build_below", Ct:"build_output", Size:511, Mode:"token" },
		Case { Label:"build_at", Ct:"build_output", Size:512, Mode:"token" },
		// ── 8 KB boundary, error content ────────────────────────────────────
		Case { Label:"error_below_8191", Ct:"error", Size:8191, Mode:"token" },
		Case { Label:"error_at_8192", Ct:"error", Size:8192, Mode:"token" },
	]
}

fn main() {
	let Cache = Spawn("cache", CachePort);
	let Token = Spawn("token", TokenPort);

	let AllCases = Cases();
	let mut Failures = 0usize;
	let mut Total = 0usize;

	eprintln!("\n[bench_02] /ccr/create + /retrieve boundary sweep  {} cases", AllCases.len());
	eprintln!("{:<26} {:>8} {:>8} {:>8}", "label", "mode", "size", "result");
	eprintln!("{}", "─".repeat(55));

	for c in &AllCases {
		let Ports:Vec<(u16, &str)> = match c.Mode {
			"cache" => vec![(CachePort, "cache")],
			"token" => vec![(TokenPort, "token")],
			_ => vec![(CachePort, "cache"), (TokenPort, "token")],
		};
		for (Port, ModeLabel) in Ports {
			Total += 1;
			let Pass = Probe(Port, c.Ct, c.Size);
			if !Pass {
				Failures += 1;
			}
			eprintln!(
				"{:<26} {:>8} {:>8} {:>8}",
				c.Label,
				ModeLabel,
				c.Size,
				if Pass { "PASS" } else { "FAIL ←" }
			);
		}
	}

	eprintln!("\n[bench_02] {}/{} passed", Total - Failures, Total);
	if Failures > 0 {
		eprintln!("[bench_02] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy),
		// which would leak the spawned cache/token processes on failure -
		// they'd keep listening on CACHE_PORT/TOKEN_PORT and silently
		// answer (with stale content) the *next* run's bind attempts.
		// Drop them explicitly before exiting.
		drop(Cache);
		drop(Token);
		std::process::exit(1);
	} else {
		eprintln!("[bench_02] OK");
	}
}
