//! bench_01_corpus - full content-type corpus run.
//!
//! Spawns cache (:49797) and token (:49798) proxies, pushes one sample
//! per content-type through /ccr/create, then retrieves each via POST
//! /retrieve. Prints a per-label table and exits non-zero on any retrieve miss.
//!
//! cargo run --example bench_01_corpus

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
/// `target/<profile>/examples/bench_01_corpus` and `target/<profile>/aphrodite`
/// are siblings two directories apart.
fn BinPath() -> std::path::PathBuf {
	if let Ok(p) = std::env::var("CARGO_BIN_EXE_aphrodite") {
		return p.into();
	}
	let Exe = std::env::current_exe().expect("current_exe");
	let BinName = if cfg!(windows) { "aphrodite.exe" } else { "aphrodite" };
	Exe.parent() // target/<profile>/examples/
		.and_then(|p| p.parent()) // target/<profile>/
		.map(|p| p.join(BinName))
		.unwrap_or_else(|| BinName.into())
}
const CachePort:u16 = 49797;
const TokenPort:u16 = 49798;

// ── proxy lifecycle ────────────────────────────────────────────────

struct Proxy {
	Child:std::process::Child,
	Port:u16,
	Mode:&'static str,
}
impl Drop for Proxy {
	fn drop(&mut self) {
		let _ = self.Child.kill();
		let _ = self.Child.wait();
	}
}

fn Spawn(Mode:&'static str, Port:u16) -> Proxy {
	let Listen = format!("127.0.0.1:{}", Port);
	// Isolate CCR storage per bench run so this never touches the operator's
	// real ~/.hermes/aphrodite/ccr.db (token mode only opens SQLite there).
	let DatabasePath = std::env::temp_dir().join(format!("aphrodite_bench_01_{}_{}.db", Mode, Port));
	// SQLite persists the CCR store across process restarts (unlike cache
	// mode's in-memory store) - without this, a leftover file from a
	// previous run would still be inside the TTL window and could poison
	// this run's retrieve checks with stale entries.
	let _ = std::fs::remove_file(&DatabasePath);
	let Child = Command::new(BinPath())
		.args([
			"--mode",
			Mode,
			"--listen",
			&Listen,
			"--api-url",
			"http://127.0.0.1:1",
			"--api-key",
			"bench",
			"--ccr-db-path",
		])
		.arg(&DatabasePath)
		// The binary prefers a repo-root `aphrodite.toml` over these CLI
		// flags whenever one exists relative to its cwd (main.rs `run()`),
		// which would silently ignore --mode/--listen/--ccr-db-path here.
		// Point it at a path that never exists so the single-proxy CLI path
		// is always used, regardless of the directory this bench is run from.
		.env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap_or_else(|_| panic!("spawn failed - run `cargo build --release` first"));
	let Dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", Port)).is_ok() {
			break;
		}
		assert!(Instant::now() < Dl, "proxy :{} did not start in time", Port);
		std::thread::sleep(Duration::from_millis(50));
	}
	eprintln!("[bench_01] {} proxy up on :{}", Mode, Port);
	Proxy { Child, Port, Mode }
}

// ── HTTP via curl (no extra deps) ──────────────────────────────────

fn CCRCreate(Port:u16, Content:&str) -> Option<serde_json::Value> {
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
	serde_json::from_slice(&Out.stdout).ok()
}

/// POST /retrieve  body: {"hash": "..."}
fn CCRRetrieve(Port:u16, hash:&str) -> bool {
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
		.ok()
		.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok());
	Out.and_then(|v| v.get("found").and_then(|f| f.as_bool())).unwrap_or(false)
}

// ── corpus ─────────────────────────────────────────────────────────

struct Sample {
	Label:&'static str,
	Content:String,
}

fn Corpus() -> Vec<Sample> {
	vec![
		Sample { Label: "tiny_text", Content: "ok ".repeat(40) }, // 120 B  - below all thresholds
		Sample { Label: "small_prose", Content: "The quick brown fox jumps. ".repeat(18) }, // ~468 B - inline zone
		Sample {
			Label: "medium_prose",
			Content: "Lorem ipsum dolor sit amet, consectetur adipiscing. ".repeat(30),
		}, // ~1.5 KB
		Sample { Label: "large_prose", Content: "Lorem ipsum dolor. ".repeat(600) }, // ~11 KB
		Sample {
			Label: "rust_code",
			Content: include_str!("bench_payloads/sample.rs").to_string(),
		}, // ~3.5 KB
		Sample {
			Label: "build_output",
			Content: "   Compiling aphrodite v0.5.0\n".repeat(70) + "    Finished release in 14.2s\n",
		},
		Sample {
			Label: "linter_output",
			Content: (0..50)
				.map(|i| {
					format!(
						"error[E0308]: mismatched types\n  --> src/lib.rs:{}:5\n   |\n{}|   expected `u64`\n",
						i * 3 + 1,
						" ".repeat(3)
					)
				})
				.collect::<String>(),
		},
		Sample {
			Label: "diff",
			Content: format!(
				"diff --git a/src/proxy.rs b/src/proxy.rs\nindex abc..def 100644\n--- a/src/proxy.rs\n+++ \
				 b/src/proxy.rs\n{}",
				(0..100)
					.map(|i| format!("-old line {}\n+new line {}\n", i, i))
					.collect::<String>()
			),
		},
		Sample {
			Label: "json_tool",
			Content: serde_json::to_string_pretty(&serde_json::json!({
				"exit_code": 0,
				"stdout": "ok\n".repeat(150),
				"stderr": "",
				"status": "success"
			}))
			.unwrap(),
		},
		Sample {
			Label: "error_output",
			Content: format!(
				"thread 'main' panicked at 'index out of bounds'\nstack backtrace:\n{}",
				(0..35)
					.map(|i| format!("  {}: some::module::fn_{} at src/lib.rs:{}\n", i, i, i * 4 + 1))
					.collect::<String>()
			),
		},
		Sample {
			Label: "log_output",
			Content: (0..80)
				.map(|i| {
					format!(
						"2026-06-16T09:{:02}:{:02}Z [INFO] request id=req-{:04} elapsed={}ms\n",
						i / 60,
						i % 60,
						i,
						i * 7 + 1
					)
				})
				.collect::<String>(),
		},
		Sample {
			Label:"unicode_cjk", Content:"日本語テスト привет мир 🦀🔥 ".repeat(140)
		}, // ~4 KB mixed UTF-8
		Sample {
			Label:"code_python",
			Content:"#!/usr/bin/env python3\nimport sys\nimport json\n\ndef main():\n    data = json.loads(sys.stdin.read())\n    result = process(data)\n    print(json.dumps(result))\n\ndef process(data):\n    out = []\n    for item in data:\n        out.append({\"id\": item[\"id\"], \"name\": item[\"name\"].upper()})\n    return out\n\nclass DataHandler:\n    def __init__(self, limit=100):\n        self.limit = limit\n        self.items = []\n    def add(self, item):\n        if len(self.items) < self.limit:\n            self.items.append(item)\n    def flush(self):\n        result = self.items.copy()\n        self.items.clear()\n        return result\n\nif __name__ == \"__main__\":\n    main()\n".repeat(8),
		}, // ~3 KB Python
		Sample {
			Label:"code_js",
			Content:"import { useState, useEffect } from 'react';\n\nfunction useDebounce(value, delay = 300) {\n    const [debounced, setDebounced] = useState(value);\n    useEffect(() => {\n        const timer = setTimeout(() => setDebounced(value), delay);\n        return () => clearTimeout(timer);\n    }, [value, delay]);\n    return debounced;\n}\n\nconst API_BASE = process.env.API_URL || 'http://localhost:3000';\n\nasync function fetchData(endpoint) {\n    const res = await fetch(`${API_BASE}/${endpoint}`);\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json();\n}\n\nexport { useDebounce, fetchData };\n".repeat(12),
		}, // ~3 KB JavaScript
		Sample {
			Label:"code_go",
			Content:"package main\n\nimport (\n    \"fmt\"\n    \"net/http\"\n    \"sync\"\n)\n\ntype Cache struct {\n    mu    sync.RWMutex\n    items map[string]string\n}\n\nfunc NewCache() *Cache {\n    return &Cache{items: make(map[string]string)}\n}\n\nfunc (c *Cache) Get(key string) (string, bool) {\n    c.mu.RLock()\n    defer c.mu.RUnlock()\n    v, ok := c.items[key]\n    return v, ok\n}\n\nfunc (c *Cache) Set(key, value string) {\n    c.mu.Lock()\n    defer c.mu.Unlock()\n    c.items[key] = value\n}\n\nfunc main() {\n    cache := NewCache()\n    cache.Set(\"hello\", \"world\")\n    if v, ok := cache.Get(\"hello\"); ok {\n        fmt.Println(v)\n    }\n    http.HandleFunc(\"/health\", func(w http.ResponseWriter, r *http.Request) {\n        w.WriteHeader(200)\n    })\n}\n".repeat(8),
		}, // ~3 KB Go
		Sample {
			Label:"search_results",
			Content:"./src/proxy.rs:470:1\n./src/proxy.rs:503:1\n./src/proxy.rs:540:1\n./src/proxy.rs:577:1\n./src/proxy.rs:614:1\n./src/proxy.rs:651:1\n./src/proxy.rs:688:1\n./src/proxy.rs:725:1\n./src/proxy.rs:762:1\n./src/proxy.rs:799:1\n./src/proxy.rs:836:1\n./src/proxy.rs:873:1\n".repeat(40),
		}, // ~2 KB search output
		Sample {
			Label:"terminal_exit",
			Content:format!(
				"$ cargo build --release\n   Compiling aphrodite v1.3.3\n{}\nerror: could not compile `aphrodite` (lib) due to 3 previous errors\nexit code: 101\n",
				(0..60).map(|i| format!("error[E{:04}]: some error message at line {}\n", i, i * 10 + 1)).collect::<String>()
			),
		}, // ~4 KB terminal with errors
		Sample {
			Label:"build_error",
			Content:format!(
				"{}\n{}\nerror: could not compile `mycrate` (lib) due to 12 previous errors\n\nSome errors have detailed explanations: E0308, E0502.\nFor more information about an error, try `rustc --explain E0308`.\n",
				(0..40).map(|i| format!("error[E{:04}]: type mismatch\n  --> src/module{}.rs:{}:{}\n   |\n{}|     let x: u64 = \"hello\";\n   |                  ^^^^^^^ expected `u64`, found `&str`\n", 300 + i, i, i * 5 + 2, i * 3 + 1, " ".repeat(5))).collect::<String>(),
				(0..10).map(|i| format!("warning[W{:04}]: unused variable `x`\n  --> src/module{}.rs:{}:{}\n", 100 + i, i, i * 7 + 5, i * 4 + 3)).collect::<String>()
			),
		}, // ~6 KB build errors
		Sample {
			Label:"huge_prose",
			Content:"The Aphrodite compression engine provides context-aware CCR. ".repeat(400),
		}, // ~24 KB large prose
		Sample {
			Label:"shell_output",
			Content:format!(
				"total 120\ndrwxr-xr-x  11 user  staff   352 Jul 14 04:10 .\ndrwxr-xr-x   7 user  staff   224 Jul 14 04:11 ..\n{}\n-rw-r--r--   1 user  staff  2222 Jul 14 04:12 Cargo.lock\n-rw-r--r--   1 user  staff   555 Jul 14 04:12 Cargo.toml\n",
				(0..80).map(|i| format!("-rw-r--r--   1 user  staff  {:>5} Jul {:>2} 04:12 file_{:03}.rs", 100 + i * 30, (i % 28) + 1, i)).collect::<Vec<_>>().join("\n")
			),
		}, // ~5 KB shell listing
	]
}

// ── run ────────────────────────────────────────────────────────────

#[derive(Default)]
struct Row {
	Orig:usize,
	Marker:usize,
	Compressed:bool,
	RetrieveOk:bool,
	RetrieveAttempted:bool,
	LatencyMs:u128,
}

fn Run(Proxy:&Proxy, Samples:&[Sample]) -> Vec<(&'static str, Row)> {
	let mut Rows = Vec::new();
	for s in Samples {
		let T0 = Instant::now();
		let Res = CCRCreate(Proxy.Port, &s.Content);
		let Latency = T0.elapsed().as_millis();
		let mut Row = Row { Orig:s.Content.len(), LatencyMs:Latency, ..Default::default() };
		if let Some(v) = Res {
			// CCRCreateResponse serializes `TokenSavingsRatio`, never
			// `compression_ratio` (proxy.rs's CCRCreateResponse struct).
			let Ratio = v.get("token_savings_ratio").and_then(|r| r.as_f64()).unwrap_or(1.0);
			Row.Marker = v.get("compressed_size").and_then(|c| c.as_u64()).unwrap_or(Row.Orig as u64) as usize;
			Row.Compressed = Ratio > 1.05;
			if Row.Compressed {
				if let Some(hash) = v.get("hash").and_then(|h| h.as_str()) {
					Row.RetrieveAttempted = true;
					Row.RetrieveOk = CCRRetrieve(Proxy.Port, hash);
					if !Row.RetrieveOk {
						eprintln!("  MISS  [{}:{}] hash={}", Proxy.Mode, s.Label, &hash[..8.min(hash.len())]);
					}
				}
			}
		} else {
			eprintln!("  ERROR [{}:{}] ccr/create failed", Proxy.Mode, s.Label);
		}
		let RatioStr = if Row.Compressed {
			format!("{:.2}x", Row.Orig as f64 / Row.Marker.max(1) as f64)
		} else {
			"  -   ".into()
		};
		eprintln!(
			"  [{Mode}] {Label:<20} {Orig:>7}B {Status:<12} {Ratio:<8} {Lat}ms",
			Mode = Proxy.Mode,
			Label = s.Label,
			Orig = Row.Orig,
			Status = if Row.Compressed { "COMPRESSED" } else { "passthrough" },
			Ratio = RatioStr,
			Lat = Latency
		);
		Rows.push((s.Label, Row));
	}
	Rows
}

fn PrintReport(Mode:&str, Rows:&[(&'static str, Row)]) {
	eprintln!("\n{}", "─".repeat(80));
	eprintln!("  mode={}  {} samples", Mode, Rows.len());
	eprintln!(
		"{:<22} {:>8} {:>8} {:>7} {:>8} {:>8}",
		"label", "orig_B", "mark_B", "ratio", "retrieve", "lat_ms"
	);
	eprintln!("{}", "─".repeat(80));
	for (Label, r) in Rows {
		eprintln!(
			"{:<22} {:>8} {:>8} {:>7} {:>8} {:>8}",
			Label,
			r.Orig,
			if r.Compressed { r.Marker } else { 0 },
			if r.Compressed {
				format!("{:.2}x", r.Orig as f64 / r.Marker.max(1) as f64)
			} else {
				"-".into()
			},
			if r.RetrieveAttempted {
				if r.RetrieveOk { "OK" } else { "MISS" }
			} else {
				"skip"
			},
			r.LatencyMs
		);
	}
	let Misses:usize = Rows.iter().filter(|(_, r)| r.RetrieveAttempted && !r.RetrieveOk).count();
	let Hits:usize = Rows.iter().filter(|(_, r)| r.RetrieveOk).count();
	let Compr:usize = Rows.iter().filter(|(_, r)| r.Compressed).count();
	let TotalOrig:usize = Rows.iter().filter(|(_, r)| r.Compressed).map(|(_, r)| r.Orig).sum();
	let TotalMark:usize = Rows.iter().filter(|(_, r)| r.Compressed).map(|(_, r)| r.Marker).sum();
	eprintln!("{}", "─".repeat(80));
	eprintln!(
		"  compressed={} passthrough={}  retrieve hits={} misses={}",
		Compr,
		Rows.len() - Compr,
		Hits,
		Misses
	);
	if TotalMark > 0 {
		eprintln!(
			"  overall ratio: {:.2}x  ({} B -> {} B)",
			TotalOrig as f64 / TotalMark as f64,
			TotalOrig,
			TotalMark
		);
	}
}

fn main() {
	let Samples = Corpus();
	eprintln!(
		"[bench_01] corpus: {} samples, {} total bytes",
		Samples.len(),
		Samples.iter().map(|s| s.Content.len()).sum::<usize>()
	);

	let Cache = Spawn("cache", CachePort);
	let Token = Spawn("token", TokenPort);

	eprintln!("\n[bench_01] ── cache mode ──");
	let CacheRows = Run(&Cache, &Samples);
	eprintln!("\n[bench_01] ── token mode ──");
	let TokenRows = Run(&Token, &Samples);

	PrintReport("cache", &CacheRows);
	PrintReport("token", &TokenRows);

	let Misses:usize = CacheRows
		.iter()
		.chain(TokenRows.iter())
		.filter(|(_, r)| r.RetrieveAttempted && !r.RetrieveOk)
		.count();
	if Misses > 0 {
		eprintln!("\n[bench_01] FAILED - {} retrieve miss(es)", Misses);
		// std::process::exit skips unwinding (and thus Drop for Proxy),
		// which would leak the spawned cache/token processes on failure -
		// they'd keep listening on CACHE_PORT/TOKEN_PORT and silently
		// answer (with stale content) the *next* run's bind attempts.
		// Drop them explicitly before exiting.
		drop(Cache);
		drop(Token);
		std::process::exit(1);
	}
	eprintln!("\n[bench_01] OK");
}
