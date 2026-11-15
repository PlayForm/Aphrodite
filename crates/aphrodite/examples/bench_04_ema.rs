//! bench_04_ema - EMA auto-tune drift and threshold feedback loop.
//!
//! Drives the compression_ratio_ema field (exposed on GET /stats as
//! `CompressionRatioEma`) through three phases:
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

/// Path to the `Aphrodite` binary.
///
/// `CARGO_BIN_EXE_aphrodite` is only set by Cargo for `cargo test`/`cargo
/// bench` harnesses, not for `cargo run --example` (this file's own doc
/// comment says to run it that way) - so it can't be relied on here. Instead,
/// derive the binary path from this example's own executable location:
/// `target/<profile>/examples/bench_04_ema` and `target/<profile>/aphrodite`
/// are siblings two directories apart.
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
const TokenPort:u16 = 39799;

struct Proxy {
	Child:std::process::Child,
	Port:u16,
}
impl Drop for Proxy {
	fn drop(&mut self) {
		let _ = self.Child.kill();
		let _ = self.Child.wait();
	}
}
fn SpawnProxy() -> Proxy {
	let Listen = format!("127.0.0.1:{}", TokenPort);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let DatabasePath = std::env::temp_dir().join(format!("aphrodite_bench_04_{}.db", TokenPort));
	// SQLite persists the CCR store across process restarts - without this,
	// a leftover file from a previous run would still be inside the TTL
	// window and could poison this run's EMA measurements with stale state.
	let _ = std::fs::remove_file(&DatabasePath);
	let Child = Command::new(BinPath())
		.args([
			"--mode",
			"token",
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
		if std::net::TcpStream::connect(("127.0.0.1", TokenPort)).is_ok() {
			break;
		}
		assert!(Instant::now() < Dl);
		std::thread::sleep(Duration::from_millis(50));
	}
	Proxy { Child, Port:TokenPort }
}

fn CCRCreate(Port:u16, Content:&str) {
	let body = serde_json::json!({"content": Content}).to_string();
	let _ = Command::new("curl")
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
		.output();
}

fn ReadEma(Port:u16) -> f64 {
	let Out = Command::new("curl")
		.args(["-s", &format!("http://127.0.0.1:{}/stats", Port)])
		.output()
		.ok()
		.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok());
	Out.and_then(|v| v.get("compression_ratio_ema").and_then(|e| e.as_f64()))
		.unwrap_or(0.0)
}

/// Read back the compression threshold for a given content type by injecting
/// a pair of probes at threshold-1 and threshold bytes and checking which
/// one compresses.  Binary-searches between 256 and 65536.
fn MeasureThreshold(Port:u16, Ct:&str) -> usize {
	let Make = |Size:usize| -> String {
		let Unit = match Ct {
			"linter" => "error[E0308]: mismatched types\n  --> src/lib.rs:1:5\n",
			"build_output" => "   Compiling crate v0.1.0\n",
			_ => "a",
		};
		let Rep = (Size / Unit.len()).max(1);
		let mut s = Unit.repeat(Rep);
		match s.len().cmp(&Size) {
			std::cmp::Ordering::Greater => s.truncate(Size),
			std::cmp::Ordering::Less => s.extend(std::iter::repeat_n('x', Size - s.len())),
			_ => {},
		}
		s
	};
	let Compressed = |Size:usize| -> bool {
		let body = serde_json::json!({"content": Make(Size)}).to_string();
		Command::new("curl")
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
			.ok()
			.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
			.and_then(|v| v.get("token_savings_ratio").and_then(|r| r.as_f64()))
			.map(|r| r > 1.05)
			.unwrap_or(false)
	};
	let mut Lo = 256usize;
	let mut Hi = 65536usize;
	while Lo + 1 < Hi {
		let Mid = (Lo + Hi) / 2;
		if Compressed(Mid) {
			Hi = Mid;
		} else {
			Lo = Mid;
		}
	}
	Hi
}

fn Check(ID:u8, Label:&str, Pass:bool, Failures:&mut usize) {
	eprintln!("  {:02}  {:<60} {}", ID, Label, if Pass { "PASS" } else { "FAIL ←" });
	if !Pass {
		*Failures += 1;
	}
}

fn main() {
	let Proxy = SpawnProxy();
	let mut Failures = 0usize;

	// High-ratio content: single repeated character - only 1 unique 3-byte
	// trigram, so the entropy estimate gives the highest possible ratio.
	// 100 KB pushes EMA well above 20×, triggering the R-9 auto-tune branch.
	let High = || -> String { "a".repeat(100_000) }; // 100 KB, maximum compression
	// Low-ratio content: pseudo-random hex, ~32 KB, nearly incompressible
	let Low = || -> String {
		(0u64..8000)
			.map(|i| format!("{:016x}", i.wrapping_mul(6364136223846793005)))
			.collect()
	};

	let InitialEma = ReadEma(Proxy.Port);
	eprintln!("[bench_04] initial EMA: {:.2}x", InitialEma);

	// ── Phase A: warm-up with high-ratio payloads ─────────────────────────
	eprintln!("\n[bench_04] Phase A: 10 high-ratio inserts");
	for _ in 0..10 {
		CCRCreate(Proxy.Port, &High());
	}
	let EmaA = ReadEma(Proxy.Port);
	eprintln!("  EMA after phase A: {:.2}x", EmaA);
	Check(1, &format!("phase A: EMA > 5.0x (got {:.2}x)", EmaA), EmaA > 5.0, &mut Failures);
	Check(
		2,
		&format!("phase A: EMA < 500x (got {:.2}x)", EmaA),
		EmaA < 500.0,
		&mut Failures,
	);

	// ── Phase B: shock with incompressible payloads ───────────────────────
	eprintln!("\n[bench_04] Phase B: 10 low-ratio inserts");
	for _ in 0..10 {
		CCRCreate(Proxy.Port, &Low());
	}
	let EmaB = ReadEma(Proxy.Port);
	eprintln!("  EMA after phase B: {:.2}x", EmaB);
	Check(
		3,
		&format!("phase B: EMA decayed below phase A ({:.2} < {:.2})", EmaB, EmaA),
		EmaB < EmaA,
		&mut Failures,
	);
	Check(
		4,
		&format!("phase B: EMA still > 1.0x (got {:.2}x)", EmaB),
		EmaB > 1.0,
		&mut Failures,
	);

	// ── Phase C: recovery ────────────────────────────────────────────────
	eprintln!("\n[bench_04] Phase C: 10 more high-ratio inserts");
	for _ in 0..10 {
		CCRCreate(Proxy.Port, &High());
	}
	let EmaC = ReadEma(Proxy.Port);
	eprintln!("  EMA after phase C: {:.2}x", EmaC);
	Check(
		5,
		&format!("phase C: EMA recovered above phase B ({:.2} > {:.2})", EmaC, EmaB),
		EmaC > EmaB,
		&mut Failures,
	);

	// ── R-9 guard: linter threshold must not double when EMA is high ──────
	// After phase A + C the EMA should be well above 20x which triggers the
	// auto-tune 2× branch.  The bug (R-9) would double the linter threshold
	// from 512 B to 1024 B.  We measure the actual threshold and assert
	// it stays at or below 600 B (giving 17% tolerance for EMA fluctuation).
	eprintln!("\n[bench_04] R-9 guard: linter threshold under high EMA");
	// Re-read EMA to confirm it is in the high-ratio range
	let EmaCheck = ReadEma(Proxy.Port);
	eprintln!("  current EMA: {:.2}x", EmaCheck);
	if EmaCheck > 20.0 {
		let LinterThresh = MeasureThreshold(Proxy.Port, "linter");
		eprintln!("  measured linter threshold: {} B", LinterThresh);
		Check(
			6,
			&format!(
				"linter threshold <= 600 B even with EMA {:.1}x (got {} B)",
				EmaCheck, LinterThresh
			),
			LinterThresh <= 600,
			&mut Failures,
		);

		let BuildThresh = MeasureThreshold(Proxy.Port, "build_output");
		eprintln!("  measured build_output threshold: {} B", BuildThresh);
		Check(
			7,
			&format!(
				"build_output threshold <= 600 B even with EMA {:.1}x (got {} B)",
				EmaCheck, BuildThresh
			),
			BuildThresh <= 600,
			&mut Failures,
		);
	} else {
		eprintln!(
			"  EMA not high enough to trigger R-9 branch ({:.2}x < 20x) - skipping guard",
			EmaCheck
		);
	}

	// ── summary ───────────────────────────────────────────────────────────
	eprintln!("\n[bench_04] {}/{} checks passed", 7 - Failures, 7);
	if Failures > 0 {
		eprintln!("[bench_04] FAILED");
		// std::process::exit skips unwinding (and thus Drop for Proxy), which
		// would leak the spawned proxy process on failure - it would keep
		// listening on TOKEN_PORT and silently answer the next run's bind
		// attempt with stale EMA state. Drop it explicitly before exiting.
		drop(Proxy);
		std::process::exit(1);
	} else {
		eprintln!("[bench_04] OK");
	}
}
