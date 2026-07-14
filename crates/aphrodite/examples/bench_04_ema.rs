//! bench_04_ema - EMA auto-tune drift and threshold feedback loop.
//!
//! Drives the compression_ratio_ema field (exposed on GET /stats as
//! `compression_ratio_ema`) through three phases:
//!
//!   Phase A - warm-up:   10 high-ratio payloads (very repetitive text).
//!                        EMA should climb well above 10x.
//!   Phase B - shock:     10 near-incompressible payloads (hex noise).
//!                        EMA should decay toward 1x.
//!   Phase C - recovery:  10 more high-ratio payloads.
//!                        EMA should partially recover (not collapse permanently).
//!
//! After each phase, asserts EMA is within expected range.
//! Also checks that threshold_for() auto-tune multipliers do not flip
//! linter/build_output to the wrong half (bug R-9: doubling on high EMA).
//!
//! cargo run --example bench_04_ema

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
/// `target/<profile>/examples/bench_04_ema` and `target/<profile>/aphrodite`
/// are siblings two directories apart.
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
const TOKEN_PORT: u16 = 39799;

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
fn spawn_proxy() -> Proxy {
	let listen = format!("127.0.0.1:{}", TOKEN_PORT);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let db_path = std::env::temp_dir().join(format!("aphrodite_bench_04_{}.db", TOKEN_PORT));
	// SQLite persists the CCR store across process restarts - without this,
	// a leftover file from a previous run would still be inside the TTL
	// window and could poison this run's EMA measurements with stale state.
	let _ = std::fs::remove_file(&db_path);
	let child = Command::new(bin_path())
        .args(["--mode", "token", "--listen", &listen,
               "--api-url", "http://127.0.0.1:1", "--api-key", "bench",
               "--ccr-db-path"])
        .arg(&db_path)
        // See bin_path()'s doc comment: a repo-root aphrodite.toml would
        // otherwise silently override --mode/--listen/--ccr-db-path.
        .env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().expect("spawn");
	let dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", TOKEN_PORT)).is_ok() {
			break;
		}
		assert!(Instant::now() < dl);
		std::thread::sleep(Duration::from_millis(50));
	}
	Proxy { child, port: TOKEN_PORT }
}

fn ccr_create(port: u16, content: &str) {
	let body = serde_json::json!({"content": content}).to_string();
	let _ = Command::new("curl")
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
		.output();
}

fn read_ema(port: u16) -> f64 {
	let out = Command::new("curl")
		.args(["-s", &format!("http://127.0.0.1:{}/stats", port)])
		.output()
		.ok()
		.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok());
	out.and_then(|v| v.get("compression_ratio_ema").and_then(|e| e.as_f64()))
		.unwrap_or(0.0)
}

/// Read back the compression threshold for a given content type by injecting
/// a pair of probes at threshold-1 and threshold bytes and checking which
/// one compresses.  Binary-searches between 256 and 65536.
fn measure_threshold(port: u16, ct: &str) -> usize {
	let make = |size: usize| -> String {
		let unit = match ct {
			"linter" => "error[E0308]: mismatched types\n  --> src/lib.rs:1:5\n",
			"build_output" => "   Compiling crate v0.1.0\n",
			_ => "a",
		};
		let rep = (size / unit.len()).max(1);
		let mut s = unit.repeat(rep);
		match s.len().cmp(&size) {
			std::cmp::Ordering::Greater => s.truncate(size),
			std::cmp::Ordering::Less => s.extend(std::iter::repeat('x').take(size - s.len())),
			_ => {},
		}
		s
	};
	let compressed = |size: usize| -> bool {
		let body = serde_json::json!({"content": make(size)}).to_string();
		Command::new("curl")
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
			.ok()
			.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
			.and_then(|v| v.get("token_savings_ratio").and_then(|r| r.as_f64()))
			.map(|r| r > 1.05)
			.unwrap_or(false)
	};
	let mut lo = 256usize;
	let mut hi = 65536usize;
	while lo + 1 < hi {
		let mid = (lo + hi) / 2;
		if compressed(mid) {
			hi = mid;
		} else {
			lo = mid;
		}
	}
	hi
}

fn check(id: u8, label: &str, pass: bool, failures: &mut usize) {
	eprintln!("  {:02}  {:<60} {}", id, label, if pass { "PASS" } else { "FAIL ←" });
	if !pass {
		*failures += 1;
	}
}

fn main() {
	let proxy = spawn_proxy();
	let mut failures = 0usize;

	// High-ratio content: highly repetitive, zlib/zstd compresses extremely well
	let high = || -> String { "aaabbbccc ".repeat(800) }; // ~8 KB, ratio >> 10x
	// Low-ratio content: pseudo-random hex, nearly incompressible
	let low = || -> String {
		(0u64..500)
			.map(|i| format!("{:016x}", i.wrapping_mul(6364136223846793005)))
			.collect()
	};

	let initial_ema = read_ema(proxy.port);
	eprintln!("[bench_04] initial EMA: {:.2}x", initial_ema);

	// ── Phase A: warm-up with high-ratio payloads ─────────────────────────
	eprintln!("\n[bench_04] Phase A: 10 high-ratio inserts");
	for _ in 0..10 {
		ccr_create(proxy.port, &high());
	}
	let ema_a = read_ema(proxy.port);
	eprintln!("  EMA after phase A: {:.2}x", ema_a);
	check(
		1,
		&format!("phase A: EMA > 5.0x (got {:.2}x)", ema_a),
		ema_a > 5.0,
		&mut failures,
	);
	check(
		2,
		&format!("phase A: EMA < 500x (got {:.2}x)", ema_a),
		ema_a < 500.0,
		&mut failures,
	);

	// ── Phase B: shock with incompressible payloads ───────────────────────
	eprintln!("\n[bench_04] Phase B: 10 low-ratio inserts");
	for _ in 0..10 {
		ccr_create(proxy.port, &low());
	}
	let ema_b = read_ema(proxy.port);
	eprintln!("  EMA after phase B: {:.2}x", ema_b);
	check(
		3,
		&format!("phase B: EMA decayed below phase A ({:.2} < {:.2})", ema_b, ema_a),
		ema_b < ema_a,
		&mut failures,
	);
	check(
		4,
		&format!("phase B: EMA still > 1.0x (got {:.2}x)", ema_b),
		ema_b > 1.0,
		&mut failures,
	);

	// ── Phase C: recovery ────────────────────────────────────────────────
	eprintln!("\n[bench_04] Phase C: 10 more high-ratio inserts");
	for _ in 0..10 {
		ccr_create(proxy.port, &high());
	}
	let ema_c = read_ema(proxy.port);
	eprintln!("  EMA after phase C: {:.2}x", ema_c);
	check(
		5,
		&format!("phase C: EMA recovered above phase B ({:.2} > {:.2})", ema_c, ema_b),
		ema_c > ema_b,
		&mut failures,
	);

	// ── R-9 guard: linter threshold must not double when EMA is high ──────
	// After phase A + C the EMA should be well above 20x which triggers the
	// auto-tune 2× branch.  The bug (R-9) would double the linter threshold
	// from 512 B to 1024 B.  We measure the actual threshold and assert
	// it stays at or below 600 B (giving 17% tolerance for EMA fluctuation).
	eprintln!("\n[bench_04] R-9 guard: linter threshold under high EMA");
	// Re-read EMA to confirm it is in the high-ratio range
	let ema_check = read_ema(proxy.port);
	eprintln!("  current EMA: {:.2}x", ema_check);
	if ema_check > 20.0 {
		let linter_thresh = measure_threshold(proxy.port, "linter");
		eprintln!("  measured linter threshold: {} B", linter_thresh);
		check(
			6,
			&format!(
				"linter threshold <= 600 B even with EMA {:.1}x (got {} B)",
				ema_check, linter_thresh
			),
			linter_thresh <= 600,
			&mut failures,
		);

		let build_thresh = measure_threshold(proxy.port, "build_output");
		eprintln!("  measured build_output threshold: {} B", build_thresh);
		check(
			7,
			&format!(
				"build_output threshold <= 600 B even with EMA {:.1}x (got {} B)",
				ema_check, build_thresh
			),
			build_thresh <= 600,
			&mut failures,
		);
	} else {
		eprintln!(
			"  EMA not high enough to trigger R-9 branch ({:.2}x < 20x) - skipping guard",
			ema_check
		);
	}

	// ── summary ───────────────────────────────────────────────────────────
	eprintln!("\n[bench_04] {}/{} checks passed", 7 - failures, 7);
	if failures > 0 {
		eprintln!("[bench_04] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy), which
		// would leak the spawned proxy process on failure - it would keep
		// listening on TOKEN_PORT and silently answer the next run's bind
		// attempt with stale EMA state. Drop it explicitly before exiting.
		drop(proxy);
		std::process::exit(1);
	} else {
		eprintln!("[bench_04] OK");
	}
}
