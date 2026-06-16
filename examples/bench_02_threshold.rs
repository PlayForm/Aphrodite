//! bench_02_threshold — boundary sweep for every compression threshold.
//!
//! Probes payload sizes at threshold-1, threshold, threshold+1 for:
//!   INLINE_CCR_THRESHOLD  256 B  (stored inline, not CCR-marked)
//!   TOKEN_COMPRESS_THRESHOLD  1 KB  (token mode base)
//!   CACHE_COMPRESS_THRESHOLD  8 KB  (cache mode base)
//!   code_rust multiplier 4×  → 4 KB in token mode
//!   linter/build_output multiplier 0.5× → 512 B in token mode
//!   error multiplier 8× → 8 KB in token mode
//!
//! Exits non-zero on any boundary violation.
//!
//! cargo run --example bench_02_threshold

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_aphrodite");
const CACHE_PORT: u16 = 59797;
const TOKEN_PORT: u16 = 59798;

struct Proxy { child: std::process::Child, port: u16 }
impl Drop for Proxy {
    fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); }
}
fn spawn(mode: &str, port: u16) -> Proxy {
    let listen = format!("127.0.0.1:{}", port);
    let child = Command::new(BIN)
        .args(["--mode", mode, "--listen", &listen,
               "--api-url", "http://127.0.0.1:1", "--api-key", "bench"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().expect("spawn");
    let dl = Instant::now() + Duration::from_secs(5);
    loop {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() { break; }
        assert!(Instant::now() < dl);
        std::thread::sleep(Duration::from_millis(50));
    }
    Proxy { child, port }
}

fn ccr_create(port: u16, content: &str) -> Option<f64> {
    let body = serde_json::json!({"content": content}).to_string();
    let out = Command::new("curl")
        .args(["-s", "-X", "POST",
               &format!("http://127.0.0.1:{}/ccr/create", port),
               "-H", "Content-Type: application/json", "-d", &body])
        .output().ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v.get("compression_ratio").and_then(|r| r.as_f64())
}

/// Generate a payload of exactly `size` bytes that triggers `ct` detection.
fn make(ct: &str, size: usize) -> String {
    let unit: &str = match ct {
        "code_rust"    => "fn foo() -> u64 { 42 }\n",
        "linter"       => "error[E0308]: mismatched types\n  --> src/lib.rs:1:5\n",
        "build_output" => "   Compiling crate v0.1.0\n",
        "error"        => "thread 'main' panicked at 'index out of bounds'\n",
        _              => "a",
    };
    let rep = (size / unit.len()).max(1);
    let mut s = unit.repeat(rep);
    // Trim or pad to exact byte size (ASCII-only units, so byte == char)
    match s.len().cmp(&size) {
        std::cmp::Ordering::Greater => s.truncate(size),
        std::cmp::Ordering::Less   => s.extend(std::iter::repeat('x').take(size - s.len())),
        std::cmp::Ordering::Equal  => {}
    }
    s
}

/// A single probe: returns (pass, ratio).
fn probe(port: u16, ct: &str, size: usize, expect_compressed: bool) -> (bool, f64) {
    let content = make(ct, size);
    match ccr_create(port, &content) {
        Some(ratio) => {
            let compressed = ratio > 1.05;
            let pass = compressed == expect_compressed;
            (pass, ratio)
        }
        None => (false, 0.0),
    }
}

struct Case {
    label:             &'static str,
    ct:                &'static str,
    size:              usize,
    mode:              &'static str,  // "cache" | "token" | "both"
    expect_compressed: bool,
}

fn cases() -> Vec<Case> {
    vec![
        // ── INLINE threshold  (256 B) ──────────────────────────────────────
        // Below inline: tiny, no CCR at all
        Case { label: "inline_below_255",   ct: "text", size: 255,  mode: "both",  expect_compressed: false },
        // At inline: stored in inline_ccr map, compression_ratio == 1.0 from /ccr/create
        Case { label: "inline_at_256",      ct: "text", size: 256,  mode: "both",  expect_compressed: false },
        // ── TOKEN threshold  (1 KB) ────────────────────────────────────────
        Case { label: "token_below_1023",   ct: "text", size: 1023, mode: "token", expect_compressed: false },
        Case { label: "token_at_1024",      ct: "text", size: 1024, mode: "token", expect_compressed: true  },
        Case { label: "token_above_1025",   ct: "text", size: 1025, mode: "token", expect_compressed: true  },
        // ── CACHE threshold  (8 KB) ────────────────────────────────────────
        Case { label: "cache_below_8191",   ct: "text", size: 8191, mode: "cache", expect_compressed: false },
        Case { label: "cache_at_8192",      ct: "text", size: 8192, mode: "cache", expect_compressed: true  },
        Case { label: "cache_above_8193",   ct: "text", size: 8193, mode: "cache", expect_compressed: true  },
        // ── code_rust  4× multiplier  → 4 KB token threshold ──────────────
        Case { label: "code_rust_below",    ct: "code_rust", size: 4095, mode: "token", expect_compressed: false },
        Case { label: "code_rust_at",       ct: "code_rust", size: 4096, mode: "token", expect_compressed: true  },
        Case { label: "code_rust_above",    ct: "code_rust", size: 4097, mode: "token", expect_compressed: true  },
        // ── linter  0.5× multiplier  → 512 B token threshold ──────────────
        Case { label: "linter_below",       ct: "linter",   size: 511,  mode: "token", expect_compressed: false },
        Case { label: "linter_at",          ct: "linter",   size: 512,  mode: "token", expect_compressed: true  },
        Case { label: "linter_above",       ct: "linter",   size: 513,  mode: "token", expect_compressed: true  },
        // ── build_output  0.5× multiplier  → 512 B token threshold ────────
        Case { label: "build_below",        ct: "build_output", size: 511, mode: "token", expect_compressed: false },
        Case { label: "build_at",           ct: "build_output", size: 512, mode: "token", expect_compressed: true  },
        // ── error  8× multiplier  → 8 KB token threshold ──────────────────
        Case { label: "error_below_8191",   ct: "error",   size: 8191, mode: "token", expect_compressed: false },
        Case { label: "error_at_8192",      ct: "error",   size: 8192, mode: "token", expect_compressed: true  },
    ]
}

fn main() {
    let cache = spawn("cache", CACHE_PORT);
    let token = spawn("token", TOKEN_PORT);

    let all_cases = cases();
    let mut failures = 0usize;
    let mut total = 0usize;

    eprintln!("\n[bench_02] threshold boundary sweep  {} cases", all_cases.len());
    eprintln!("{:<26} {:>8} {:>8} {:>10} {:>10} {:>8}",
        "label", "mode", "size", "ratio", "expect", "result");
    eprintln!("{}", "─".repeat(75));

    for c in &all_cases {
        let ports: Vec<(u16, &str)> = match c.mode {
            "cache" => vec![(CACHE_PORT, "cache")],
            "token" => vec![(TOKEN_PORT, "token")],
            _       => vec![(CACHE_PORT, "cache"), (TOKEN_PORT, "token")],
        };
        for (port, mode_label) in ports {
            total += 1;
            let (pass, ratio) = probe(port, c.ct, c.size, c.expect_compressed);
            if !pass { failures += 1; }
            eprintln!("{:<26} {:>8} {:>8} {:>10.3}x {:>10} {:>8}",
                c.label, mode_label, c.size, ratio,
                if c.expect_compressed { "compressed" } else { "passthrough" },
                if pass { "PASS" } else { "FAIL ←" });
        }
    }

    eprintln!("\n[bench_02] {}/{} passed", total - failures, total);
    if failures > 0 { eprintln!("[bench_02] FAILED"); std::process::exit(1); }
    else { eprintln!("[bench_02] OK"); }
}
