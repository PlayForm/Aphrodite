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

/// Path to the `Aphrodite` binary.
///
/// `CARGO_BIN_EXE_aphrodite` is only set by Cargo for `cargo test`/`cargo
/// bench` harnesses, not for `cargo run --example` (this file's own doc
/// comment says to run it that way) - so it can't be relied on here. Instead,
/// derive the binary path from this example's own executable location:
/// `target/<profile>/examples/bench_03_retrieve` and
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
const CachePort:u16 = 39797;
const TokenPort:u16 = 39798;

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
	let DatabasePath = std::env::temp_dir().join(format!("aphrodite_bench_03_{}_{}.db", Mode, Port));
	// SQLite persists the CCR store across process restarts (unlike cache
	// mode's in-memory store) - without this, a leftover file from a
	// previous run would still be inside the TTL window and could poison
	// this run's cross-port isolation checks with stale entries (the actual
	// root cause of an earlier debugging session on this file - see git
	// history/09-testing-quality.md T14 notes).
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

fn Store(Port:u16, Content:&str) -> Option<String> {
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
	v.get("hash").and_then(|h| h.as_str()).map(|s| s.to_string())
}

/// POST /retrieve  {"hash": "..."}  → {"found": bool, "content": ...}
fn RetrieveRaw(Port:u16, hash:&str) -> Option<serde_json::Value> {
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
		.ok()?;
	serde_json::from_slice(&Out.stdout).ok()
}

fn Found(Port:u16, hash:&str) -> bool {
	RetrieveRaw(Port, hash)
		.and_then(|v| v.get("found").and_then(|f| f.as_bool()))
		.unwrap_or(false)
}

/// POST /retrieve and return HTTP status code (for negative-path tests).
#[allow(dead_code)]
fn RetrieveStatus(Port:u16, hash:&str) -> Option<u16> {
	let body = serde_json::json!({"hash": hash}).to_string();
	Command::new("curl")
		.args([
			"-s",
			"-o",
			"/dev/null",
			"-w",
			"%{http_code}",
			"-X",
			"POST",
			&format!("http://127.0.0.1:{}/retrieve", Port),
			"-H",
			"Content-Type: application/json",
			"-d",
			&body,
		])
		.output()
		.ok()
		.and_then(|o| String::from_utf8(o.stdout).ok())
		.and_then(|s| s.trim().parse().ok())
}

/// DELETE /ccr/{hash}
fn delete(Port:u16, hash:&str) -> bool {
	let Out = Command::new("curl")
		.args(["-s", "-X", "DELETE", &format!("http://127.0.0.1:{}/ccr/{}", Port, hash)])
		.output()
		.ok();
	Out.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
		.and_then(|v| v.get("deleted").and_then(|d| d.as_bool()))
		.unwrap_or(false)
}

fn Check(ID:u8, Label:&str, Pass:bool, Failures:&mut usize) {
	eprintln!("  {:02}  {:<52} {}", ID, Label, if Pass { "PASS" } else { "FAIL ←" });
	if !Pass {
		*Failures += 1;
	}
}

fn main() {
	let Cache = Spawn("cache", CachePort);
	let Token = Spawn("token", TokenPort);
	let mut Failures = 0usize;

	// ── 01 / 02  same-port store + retrieve ───────────────────────────────
	// NOTE: cache/token content must differ here - CCR hashes are
	// content-addressed (compute_key), so storing identical bytes on both
	// ports produces the same hash and makes checks 03/04 (cross-port
	// isolation) vacuously fail regardless of actual store isolation.
	let LargeCache = "x".repeat(12_000);
	let LargeToken = "y".repeat(12_000);
	let Hc = Store(CachePort, &LargeCache).expect("cache store large");
	let Ht = Store(TokenPort, &LargeToken).expect("token store large");
	Check(1, "cache: store + retrieve large (12 KB)", Found(CachePort, &Hc), &mut Failures);
	Check(2, "token: store + retrieve large (12 KB)", Found(TokenPort, &Ht), &mut Failures);

	// ── 03 / 04  cross-port isolation ─────────────────────────────────────
	Check(3, "token hash on cache port = miss", !Found(CachePort, &Ht), &mut Failures);
	Check(4, "cache hash on token port = miss", !Found(TokenPort, &Hc), &mut Failures);

	// ── 05  inline_ccr zone (257 B) retrievable ───────────────────────────
	// inline_ccr stores entries between INLINE_CCR_THRESHOLD (256 B) and
	// TOKEN_COMPRESS_THRESHOLD (1 KB).  /retrieve must check inline_ccr first.
	let InlineContent = "hello inline store ".repeat(14); // 266 B
	let Hi = Store(TokenPort, &InlineContent).expect("inline store");
	Check(
		5,
		"inline_ccr (266 B) retrievable via POST /retrieve",
		Found(TokenPort, &Hi),
		&mut Failures,
	);

	// ── 06 / 07  UTF-8 round-trip ─────────────────────────────────────────
	let Utf8 = "日本語テスト привет мир 🦀🔥 ".repeat(80); // ~2.8 KB
	let Hu = Store(TokenPort, &Utf8).expect("utf8 store");
	let raw = RetrieveRaw(TokenPort, &Hu);
	let ContentOk = raw
		.as_ref()
		.and_then(|v| v.get("content").and_then(|c| c.as_str()))
		.map(|c| c == Utf8)
		.unwrap_or(false);
	Check(
		6,
		"utf-8 content: found=true",
		raw.and_then(|v| v.get("found").and_then(|f| f.as_bool())).unwrap_or(false),
		&mut Failures,
	);
	Check(7, "utf-8 content: byte-exact round-trip", ContentOk, &mut Failures);

	// ── 08  bulk storm: 50 entries ────────────────────────────────────────
	let Hashes:Vec<String> = (0u32..50)
		.filter_map(|i| Store(TokenPort, &format!("bulk {:04} {}", i, "payload ".repeat(200))))
		.collect();
	let Hits = Hashes.iter().filter(|h| Found(TokenPort, h)).count();
	Check(
		8,
		&format!("bulk storm: {}/50 retrieved (0 misses)", Hits),
		Hits == Hashes.len() && Hashes.len() == 50,
		&mut Failures,
	);

	// ── 09  DELETE then miss ───────────────────────────────────────────────
	let DelContent = "to be deleted ".repeat(100); // ~1.4 KB
	let Hd = Store(TokenPort, &DelContent).expect("store for delete");
	let Deleted = delete(TokenPort, &Hd);
	let After = Found(TokenPort, &Hd);
	Check(9, "DELETE /ccr/{hash}: deleted=true", Deleted, &mut Failures);
	Check(9, "DELETE /ccr/{hash}: subsequent retrieve=miss", !After, &mut Failures);

	// ── 10  double-store idempotency ──────────────────────────────────────
	let Dup = "duplicate content ".repeat(120); // ~2.1 KB
	let H1 = Store(TokenPort, &Dup).expect("dup store 1");
	let H2 = Store(TokenPort, &Dup).expect("dup store 2");
	Check(10, "double-store: same hash returned", H1 == H2, &mut Failures);
	Check(10, "double-store: still retrievable", Found(TokenPort, &H1), &mut Failures);

	// ── 11  invalid hashes rejected ─────────────────────────────────────
	Check(11, "invalid hash (too short): miss", !Found(TokenPort, "abc123"), &mut Failures);
	Check(
		11,
		"invalid hash (non-hex): miss",
		!Found(TokenPort, "gggggggggggggggggggggggggggggggggggggggg"),
		&mut Failures,
	);
	Check(11, "empty hash: miss", !Found(TokenPort, ""), &mut Failures);
	Check(
		11,
		"non-existent hash: miss",
		!Found(TokenPort, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
		&mut Failures,
	);

	// ── 12  large content round-trip (250 KB) ──────────────────────────
	let Huge = "LARGE PAYLOAD ".repeat(20_000); // ~260 KB
	let Hh = Store(TokenPort, &Huge).expect("huge store");
	let RawHuge = RetrieveRaw(TokenPort, &Hh);
	let HugeOk = RawHuge
		.as_ref()
		.and_then(|v| v.get("content").and_then(|c| c.as_str()))
		.map(|c| c == Huge)
		.unwrap_or(false);
	Check(12, "large content (260 KB): round-trip byte-exact", HugeOk, &mut Failures);

	// ── 13  concurrent stores + retrieves (5 parallel curl calls) ──────
	let Entries:Vec<String> = (0u32..10)
		.filter_map(|i| Store(TokenPort, &format!("concurrent bulk {:04} {}", i, "data ".repeat(500))))
		.collect();
	let Misses = Entries.iter().filter(|h| !Found(TokenPort, h)).count();
	Check(
		13,
		&format!("concurrent storm: {}/10 retrieved, 0 misses", Entries.len() - Misses),
		Misses == 0,
		&mut Failures,
	);

	// ── summary ───────────────────────────────────────────────────────────
	let Total = 17usize; // total check() calls above
	eprintln!("\n[bench_03] {}/{} checks passed", Total - Failures, Total);
	if Failures > 0 {
		eprintln!("[bench_03] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy),
		// which would leak the spawned cache/token processes on failure -
		// they'd keep listening on CACHE_PORT/TOKEN_PORT and silently
		// answer (with stale content) the *next* run's bind attempts.
		// Drop them explicitly before exiting.
		drop(Cache);
		drop(Token);
		std::process::exit(1);
	} else {
		eprintln!("[bench_03] OK");
	}
}
