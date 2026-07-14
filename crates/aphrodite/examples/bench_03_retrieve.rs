//! bench_03_retrieve - retrieval correctness.
//!
//! Each test is atomic and labeled. Tests:
//!   01  large content: store + same-port retrieve (cache)
//!   02  large content: store + same-port retrieve (token)
//!   03  cross-port isolation: token hash must miss on cache port
//!   04  cross-port isolation: cache hash must miss on token port
//!   05  inline_ccr zone (257-999 B): must be retrievable via POST /retrieve
//!       - exercises R-10 fix (inline_ccr not checked in retrieve handler)
//!   06  UTF-8 content: found=true (exercises R-5 fix: byte-boundary panic)
//!   07  UTF-8 content: retrieved content matches original byte-for-byte
//!   08  bulk storm: 50 inserts then 50 retrieves, 0 misses
//!   09  DELETE /ccr/{hash}: entry removed, subsequent retrieve = miss
//!   10  double-store same content: ratio unchanged, still retrievable
//!
//! cargo run --example bench_03_retrieve

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
/// `target/<profile>/examples/bench_03_retrieve` and
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
const CACHE_PORT: u16 = 39797;
const TOKEN_PORT: u16 = 39798;

struct Proxy {
	child: std::process::Child,
	port: u16,
}
impl Drop for Proxy {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}
fn spawn(mode: &str, port: u16) -> Proxy {
	let listen = format!("127.0.0.1:{}", port);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let db_path = std::env::temp_dir().join(format!("aphrodite_bench_03_{}_{}.db", mode, port));
	// SQLite persists the CCR store across process restarts (unlike cache
	// mode's in-memory store) - without this, a leftover file from a
	// previous run would still be inside the TTL window and could poison
	// this run's cross-port isolation checks with stale entries (the actual
	// root cause of an earlier debugging session on this file - see git
	// history/09-testing-quality.md T14 notes).
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

fn store(port: u16, content: &str) -> Option<String> {
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
	let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
	v.get("hash").and_then(|h| h.as_str()).map(|s| s.to_string())
}

/// POST /retrieve  {"hash": "..."}  → {"found": bool, "content": ...}
fn retrieve_raw(port: u16, hash: &str) -> Option<serde_json::Value> {
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
		.ok()?;
	serde_json::from_slice(&out.stdout).ok()
}

fn found(port: u16, hash: &str) -> bool {
	retrieve_raw(port, hash)
		.and_then(|v| v.get("found").and_then(|f| f.as_bool()))
		.unwrap_or(false)
}

/// DELETE /ccr/{hash}
fn delete(port: u16, hash: &str) -> bool {
	let out = Command::new("curl")
		.args(["-s", "-X", "DELETE", &format!("http://127.0.0.1:{}/ccr/{}", port, hash)])
		.output()
		.ok();
	out.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
		.and_then(|v| v.get("deleted").and_then(|d| d.as_bool()))
		.unwrap_or(false)
}

fn check(id: u8, label: &str, pass: bool, failures: &mut usize) {
	eprintln!("  {:02}  {:<52} {}", id, label, if pass { "PASS" } else { "FAIL ←" });
	if !pass {
		*failures += 1;
	}
}

fn main() {
	let cache = spawn("cache", CACHE_PORT);
	let token = spawn("token", TOKEN_PORT);
	let mut failures = 0usize;

	// ── 01 / 02  same-port store + retrieve ───────────────────────────────
	// NOTE: cache/token content must differ here - CCR hashes are
	// content-addressed (compute_key), so storing identical bytes on both
	// ports produces the same hash and makes checks 03/04 (cross-port
	// isolation) vacuously fail regardless of actual store isolation.
	let large_cache = "x".repeat(12_000);
	let large_token = "y".repeat(12_000);
	let hc = store(CACHE_PORT, &large_cache).expect("cache store large");
	let ht = store(TOKEN_PORT, &large_token).expect("token store large");
	check(
		1,
		"cache: store + retrieve large (12 KB)",
		found(CACHE_PORT, &hc),
		&mut failures,
	);
	check(
		2,
		"token: store + retrieve large (12 KB)",
		found(TOKEN_PORT, &ht),
		&mut failures,
	);

	// ── 03 / 04  cross-port isolation ─────────────────────────────────────
	check(3, "token hash on cache port = miss", !found(CACHE_PORT, &ht), &mut failures);
	check(4, "cache hash on token port = miss", !found(TOKEN_PORT, &hc), &mut failures);

	// ── 05  inline_ccr zone (257 B) retrievable ───────────────────────────
	// inline_ccr stores entries between INLINE_CCR_THRESHOLD (256 B) and
	// TOKEN_COMPRESS_THRESHOLD (1 KB).  /retrieve must check inline_ccr first.
	let inline_content = "hello inline store ".repeat(14); // 266 B
	let hi = store(TOKEN_PORT, &inline_content).expect("inline store");
	check(
		5,
		"inline_ccr (266 B) retrievable via POST /retrieve",
		found(TOKEN_PORT, &hi),
		&mut failures,
	);

	// ── 06 / 07  UTF-8 round-trip ─────────────────────────────────────────
	let utf8 = "日本語テスト привет мир 🦀🔥 ".repeat(80); // ~2.8 KB
	let hu = store(TOKEN_PORT, &utf8).expect("utf8 store");
	let raw = retrieve_raw(TOKEN_PORT, &hu);
	let content_ok = raw
		.as_ref()
		.and_then(|v| v.get("content").and_then(|c| c.as_str()))
		.map(|c| c == utf8)
		.unwrap_or(false);
	check(
		6,
		"utf-8 content: found=true",
		raw.and_then(|v| v.get("found").and_then(|f| f.as_bool())).unwrap_or(false),
		&mut failures,
	);
	check(7, "utf-8 content: byte-exact round-trip", content_ok, &mut failures);

	// ── 08  bulk storm: 50 entries ────────────────────────────────────────
	let hashes: Vec<String> = (0u32..50)
		.filter_map(|i| store(TOKEN_PORT, &format!("bulk {:04} {}", i, "payload ".repeat(200))))
		.collect();
	let hits = hashes.iter().filter(|h| found(TOKEN_PORT, h)).count();
	check(
		8,
		&format!("bulk storm: {}/50 retrieved (0 misses)", hits),
		hits == hashes.len() && hashes.len() == 50,
		&mut failures,
	);

	// ── 09  DELETE then miss ───────────────────────────────────────────────
	let del_content = "to be deleted ".repeat(100); // ~1.4 KB
	let hd = store(TOKEN_PORT, &del_content).expect("store for delete");
	let deleted = delete(TOKEN_PORT, &hd);
	let after = found(TOKEN_PORT, &hd);
	check(9, "DELETE /ccr/{hash}: deleted=true", deleted, &mut failures);
	check(9, "DELETE /ccr/{hash}: subsequent retrieve=miss", !after, &mut failures);

	// ── 10  double-store idempotency ──────────────────────────────────────
	let dup = "duplicate content ".repeat(120); // ~2.1 KB
	let h1 = store(TOKEN_PORT, &dup).expect("dup store 1");
	let h2 = store(TOKEN_PORT, &dup).expect("dup store 2");
	check(10, "double-store: same hash returned", h1 == h2, &mut failures);
	check(10, "double-store: still retrievable", found(TOKEN_PORT, &h1), &mut failures);

	// ── summary ───────────────────────────────────────────────────────────
	let total = 12usize; // total check() calls above
	eprintln!("\n[bench_03] {}/{} checks passed", total - failures, total);
	if failures > 0 {
		eprintln!("[bench_03] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy),
		// which would leak the spawned cache/token processes on failure -
		// they'd keep listening on CACHE_PORT/TOKEN_PORT and silently
		// answer (with stale content) the *next* run's bind attempts.
		// Drop them explicitly before exiting.
		drop(cache);
		drop(token);
		std::process::exit(1);
	} else {
		eprintln!("[bench_03] OK");
	}
}
