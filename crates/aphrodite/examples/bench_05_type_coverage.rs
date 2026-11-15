//! bench_05_type_coverage - per content-type classification + ratio coverage.
//!
//! Spawns one token-mode proxy, pushes one representative sample per
//! content type through /ccr/create, and reports the per-type compression
//! ratio plus an in-process classifier verdict for each sample (headroom
//! `DetectType` + the Aphrodite semantic override `DetectSemanticType`
//! - informational only: the proxy's fine-grained classifier is private
//!   and not exposed over HTTP). Exits non-zero on any coverage violation:
//!   a sample that fails to compress, or a compressed sample whose retrieve
//!   round-trip misses.
//!
//! Content types covered: build, diff, git, gitlog, ls, test, grep,
//! code_rust, code_python, code_go, code_js, json, error, log, linter,
//! text.
//!
//! cargo run --example bench_05_type_coverage
//!
//! NOTE on detection: the proxy's own classifier lives in proxy.rs
//! (`ProxyDetectContentType`) and is not exposed over HTTP, so this
//! bench verifies coverage against the same underlying engine pieces the
//! hook/FFI path uses - `Preview::detect_type` (headroom classifier) plus
//! `Preview::detect_semantic_type` (Aphrodite's tool-output override).

use std::{
	process::{Command, Stdio},
	time::{Duration, Instant},
};

/// Path to the `Aphrodite` binary (see bench_01_corpus for rationale).
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
const TokenPort:u16 = 34798;

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
	let DatabasePath = std::env::temp_dir().join(format!("aphrodite_bench_05_{}.db", TokenPort));
	// SQLite persists the CCR store across process restarts - remove any
	// leftover file so stale entries can't poison this run's ratio numbers
	// (same isolation trick as bench_01/02/04).
	let _ = std::fs::remove_file(&DatabasePath);
	let Child = Command::new(BinPath())
		.args([
			"--mode",
			"token",
			"--listen",
			&Listen,
			"--api-url",
			"http://127.0.0.1:1",
			"--api-key",
			"bench",
			"--ccr-db-path",
		])
		.arg(&DatabasePath)
		// A repo-root aphrodite.toml would override --mode/--listen/etc.
		.env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap_or_else(|_| panic!("spawn failed - run `cargo build --release` first"));
	let Dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", TokenPort)).is_ok() {
			break;
		}
		assert!(Instant::now() < Dl, "proxy :{} did not start in time", TokenPort);
		std::thread::sleep(Duration::from_millis(50));
	}
	eprintln!("[bench_05] token proxy up on :{}", TokenPort);
	Proxy { Child, Port:TokenPort }
}

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

// ── corpus: one authored sample per content type ───────────────────

struct Sample {
	Label:&'static str,
	Content:String,
}

fn Corpus() -> Vec<Sample> {
	vec![
		Sample {
			Label: "build",
			Content: "   Compiling aphrodite v1.4.3\n   Compiling headroom-core v0.1.2\n   Compiling reqwest v0.13.4\n".repeat(45) + "    Finished `release` profile [optimized] target(s) in 13.41s\n",
		}, // ~3.2 KB
		Sample {
			Label: "diff",
			Content: format!(
				"diff --git a/src/proxy.rs b/src/proxy.rs\nindex abc1234..def5678 100644\n--- a/src/proxy.rs\n+++ b/src/proxy.rs\n{}",
				(0..80)
					.map(|i| format!("-    let old = {};\n+    let new = {};\n", i, i + 1))
					.collect::<String>()
			),
		}, // ~2.8 KB
		Sample {
			Label: "git",
			Content: (0..60)
				.map(|i| {
					let Code = ["M ", "A ", "D ", "R ", "??", "UU"][i % 6];
					format!("{} src/module_{:03}.rs\n", Code, i)
				})
				.collect::<String>(),
		}, // ~1.4 KB git status porcelain
		Sample {
			Label: "gitlog",
			Content: (0..15)
				.map(|i| {
					format!(
						"commit 9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e\nAuthor: Dev <dev@example.com>\nDate:   Mon Sep {:02} 12:00:00 2026 +0000\n\n    fix: handle {} in proxy path\n",
						15 + i, i
					)
				})
				.collect::<String>(),
		}, // ~2.4 KB git log
		Sample {
			Label: "ls",
			Content: "total 128\ndrwxr-xr-x  11 user  staff   352 Sep 15 04:10 .\ndrwxr-xr-x   7 user  staff   224 Sep 15 04:11 ..\n".to_string()
						+ (0..70)
							.map(|i| format!("-rw-r--r--   1 user  staff  {:>5} Sep 15 04:12 file_{:03}.rs", 100 + i * 25, i))
							.collect::<Vec<_>>()
							.join("\n")
							.as_str(),
		}, // ~4.6 KB ls -l
		Sample {
			Label: "test",
			Content: "running 330 tests\n".to_string()
						+ (0..40)
							.map(|i| format!("test module_{}::test::test_case_{} ... ok\n", i % 8, i))
							.collect::<String>()
							.as_str()
						+ "test result: ok. 329 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s\n",
		}, // ~2.9 KB cargo test
		Sample {
			Label: "grep",
			Content: (0..90)
				.map(|i| format!("src/proxy.rs:{}:    let handler_{} = route;\n", i * 13 + 5, i))
				.collect::<String>(),
		}, // ~2.6 KB ripgrep hits
		Sample {
			Label: "code_rust",
			Content: include_str!("bench_payloads/sample.rs").to_string(),
		}, // ~3.5 KB
		Sample {
			Label: "code_python",
			Content: "#!/usr/bin/env python3\nimport sys\nimport json\n\ndef main():\n    data = json.loads(sys.stdin.read())\n    result = process(data)\n    print(json.dumps(result))\n\ndef process(data):\n    out = []\n    for item in data:\n        out.append({\"id\": item[\"id\"], \"name\": item[\"name\"].upper()})\n    return out\n\nclass DataHandler:\n    def __init__(self, limit=100):\n        self.limit = limit\n        self.items = []\n    def add(self, item):\n        if len(self.items) < self.limit:\n            self.items.append(item)\n    def flush(self):\n        result = self.items.copy()\n        self.items.clear()\n        return result\n\nif __name__ == \"__main__\":\n    main()\n".repeat(8),
		}, // ~3 KB
		Sample {
			Label: "code_go",
			Content: "package main\n\nimport (\n    \"fmt\"\n    \"net/http\"\n    \"sync\"\n)\n\ntype Cache struct {\n    mu    sync.RWMutex\n    items map[string]string\n}\n\nfunc NewCache() *Cache {\n    return &Cache{items: make(map[string]string)}\n}\n\nfunc (c *Cache) Get(key string) (string, bool) {\n    c.mu.RLock()\n    defer c.mu.RUnlock()\n    v, ok := c.items[key]\n    return v, ok\n}\n\nfunc main() {\n    cache := NewCache()\n    cache.Set(\"hello\", \"world\")\n    http.HandleFunc(\"/health\", func(w http.ResponseWriter, r *http.Request) {\n        w.WriteHeader(200)\n    })\n}\n".repeat(8),
		}, // ~3 KB
		Sample {
			Label: "code_js",
			Content: "import { useState, useEffect } from 'react';\n\nfunction useDebounce(value, delay = 300) {\n    const [debounced, setDebounced] = useState(value);\n    useEffect(() => {\n        const timer = setTimeout(() => setDebounced(value), delay);\n        return () => clearTimeout(timer);\n    }, [value, delay]);\n    return debounced;\n}\n\nconst API_BASE = process.env.API_URL || 'http://localhost:3000';\n\nasync function fetchData(endpoint) {\n    const res = await fetch(`${API_BASE}/${endpoint}`);\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json();\n}\n\nexport { useDebounce, fetchData };\n".repeat(12),
		}, // ~3 KB
		Sample {
			Label: "json",
			Content: serde_json::to_string_pretty(&serde_json::json!({
				"tool": "bench_05",
				"version": "1.4.3",
				"rows": (0..60).map(|i| serde_json::json!({"id": i, "label": format!("row_{}", i), "ratio": 1.0 + i as f64})).collect::<Vec<_>>(),
				"summary": {"compressed": 60, "total": 60}
			}))
			.unwrap(),
		}, // ~5 KB
		Sample {
			Label: "error",
			Content: format!(
				"thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 7', src/main.rs:42:17\nstack backtrace:\n{}",
				(0..30)
					.map(|i| format!("  {}: some::module::fn_{} at src/lib.rs:{}\n", i, i, i * 4 + 1))
					.collect::<String>()
			),
		}, // ~2.1 KB
		Sample {
			Label: "log",
			Content: (0..70)
				.map(|i| {
					format!(
						"2026-09-15T{:02}:{:02}:{:02}Z [INFO] request id=req-{:04} elapsed={}ms status=200\n",
						i / 3600, (i / 60) % 60, i % 60, i, i * 7 + 1
					)
				})
				.collect::<String>(),
		}, // ~4.4 KB
		Sample {
			Label: "linter",
			Content: (0..45)
				.map(|i| {
					format!(
						"error[E0308]: mismatched types\n  --> src/lib.rs:{}:5\n   |\n{}|   expected `u64`, found `&str`\n",
						i * 3 + 1,
						" ".repeat(3)
					)
				})
				.collect::<String>(),
		}, // ~3.6 KB
		Sample {
			Label: "text",
			Content: "The Aphrodite compression engine provides context-aware CCR. ".repeat(240),
		}, // ~9 KB prose
	]
}

// ── run ────────────────────────────────────────────────────────────

#[derive(Default)]
struct Row {
	Expected:&'static str,
	Detected:String,
	Semantic:Option<&'static str>,
	Orig:usize,
	Marker:usize,
	Ratio:f64,
	Compressed:bool,
	RetrieveOk:bool,
	LatencyMs:u128,
}

fn Classify(Content:&str) -> (String, Option<&'static str>) {
	// `DetectType` = headroom classifier; `DetectSemanticType` = the
	// Aphrodite tool-output override that takes precedence in preview.rs.
	(
		aphrodite::Preview::DetectType(Content),
		aphrodite::Preview::DetectSemanticType(Content),
	)
}

fn Run(Proxy:&Proxy, Samples:&[Sample]) -> Vec<Row> {
	let mut Rows = Vec::new();
	for s in Samples {
		let (Detected, Semantic) = Classify(&s.Content);
		let T0 = Instant::now();
		let Res = CCRCreate(Proxy.Port, &s.Content);
		let Latency = T0.elapsed().as_millis();
		let mut Row = Row {
			Expected:s.Label,
			Detected,
			Semantic,
			Orig:s.Content.len(),
			LatencyMs:Latency,
			..Default::default()
		};
		if let Some(v) = Res {
			let Ratio = v.get("token_savings_ratio").and_then(|r| r.as_f64()).unwrap_or(1.0);
			Row.Marker = v.get("compressed_size").and_then(|c| c.as_u64()).unwrap_or(Row.Orig as u64) as usize;
			Row.Compressed = Ratio > 1.05;
			Row.Ratio = if Row.Compressed { Row.Orig as f64 / Row.Marker.max(1) as f64 } else { 1.0 };
			if Row.Compressed {
				if let Some(hash) = v.get("hash").and_then(|h| h.as_str()) {
					Row.RetrieveOk = CCRRetrieve(Proxy.Port, hash);
					if !Row.RetrieveOk {
						eprintln!("  MISS  [{}] hash={}", s.Label, &hash[..8.min(hash.len())]);
					}
				}
			}
		} else {
			eprintln!("  ERROR [{}] ccr/create failed", s.Label);
		}
		Rows.push(Row);
	}
	Rows
}

fn EffectiveType(Row:&Row) -> String {
	// Precedence mirrors preview.rs: semantic override wins over headroom.
	Row.Semantic.unwrap_or(&Row.Detected).to_string()
}

fn PrintReport(Rows:&[Row]) {
	eprintln!("\n{}", "─".repeat(88));
	eprintln!("  content-type coverage: {} samples", Rows.len());
	eprintln!(
		"{:<12} {:<14} {:<8} {:>8} {:>8} {:>8} {:>8} {:>8}",
		"type", "classifier", "note", "orig_B", "mark_B", "ratio", "retrieve", "lat_ms"
	);
	eprintln!("{}", "─".repeat(88));
	let mut Coarse = 0usize;
	for r in Rows {
		let Eff = EffectiveType(r);
		// The classifier column shows the in-process verdict (headroom
		// `DetectType` + semantic override). The proxy's fine-grained
		// classifier (`ProxyDetectContentType`) is private and not
		// exposed over HTTP, so a coarse verdict here (e.g. `Text` for a
		// JSON blob) is expected, not a failure - the bench's gate is
		// compression + round-trip coverage per type, below.
		let Note = if Eff == r.Expected
			|| (r.Expected == "json" && Eff == "json_array")
			|| (r.Expected == "build" && Eff == "build")
			|| (r.Expected == "grep" && Eff == "grep")
		{
			"exact"
		} else {
			Coarse += 1;
			"coarse"
		};
		eprintln!(
			"{:<12} {:<14} {:<8} {:>8} {:>8} {:>8} {:>8} {:>8}",
			r.Expected,
			Eff,
			Note,
			r.Orig,
			if r.Compressed { r.Marker } else { 0 },
			if r.Compressed { format!("{:.2}x", r.Ratio) } else { "-".into() },
			if r.Compressed { if r.RetrieveOk { "OK" } else { "MISS" } } else { "skip" },
			r.LatencyMs
		);
	}
	let Compr:Vec<&Row> = Rows.iter().filter(|r| r.Compressed).collect();
	let TotalOrig:usize = Compr.iter().map(|r| r.Orig).sum();
	let TotalMark:usize = Compr.iter().map(|r| r.Marker).sum();
	eprintln!("{}", "─".repeat(88));
	eprintln!(
		"  compressed={} passthrough={}  retrieve hits={} misses={}  coarse-classifier={}",
		Compr.len(),
		Rows.len() - Compr.len(),
		Compr.iter().filter(|r| r.RetrieveOk).count(),
		Compr.iter().filter(|r| !r.RetrieveOk).count(),
		Coarse
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
		"[bench_05] corpus: {} samples, {} total bytes",
		Samples.len(),
		Samples.iter().map(|s| s.Content.len()).sum::<usize>()
	);

	let Proxy = SpawnProxy();
	let Rows = Run(&Proxy, &Samples);
	PrintReport(&Rows);

	// Coverage gate: every sample must compress (ratio > 1.05) AND
	// round-trip. Coarse in-process classification is informational only
	// (see print_report) - the proxy's own fine-grained classifier is not
	// exposed over HTTP, so it cannot be asserted from here.
	let Misses:usize = Rows.iter().filter(|r| r.Compressed && !r.RetrieveOk).count();
	let Failures:usize = Rows.iter().filter(|r| !r.Compressed).count();
	drop(Proxy);
	if Misses > 0 || Failures > 0 {
		eprintln!("\n[bench_05] FAILED - uncompressed={} retrieve miss(es)={}", Failures, Misses);
		std::process::exit(1);
	}
	eprintln!("\n[bench_05] OK");
}
