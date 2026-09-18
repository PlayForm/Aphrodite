//! bench_05_type_coverage - per content-type classification + ratio coverage.
//!
//! Spawns one token-mode proxy, pushes one representative sample per
//! content type through /ccr/create, and reports the per-type compression
//! ratio plus an in-process classifier verdict for each sample (headroom
//! `detect_type` + the Aphrodite semantic override `detect_semantic_type`
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
//! (`proxy_detect_content_type`) and is not exposed over HTTP, so this
//! bench verifies coverage against the same underlying engine pieces the
//! hook/FFI path uses - `preview::detect_type` (headroom classifier) plus
//! `preview::detect_semantic_type` (Aphrodite's tool-output override).

use std::{
	process::{Command, Stdio},
	time::{Duration, Instant},
};

/// Path to the `aphrodite` binary (see bench_01_corpus for rationale).
fn bin_path() -> std::path::PathBuf {
	if let Ok(p) = std::env::var("CARGO_BIN_EXE_aphrodite") {
		return p.into();
	}
	let exe = std::env::current_exe().expect("current_exe");
	let bin_name = if cfg!(windows) { "aphrodite.exe" } else { "aphrodite" };
	exe.parent() // target/<profile>/examples/
		.and_then(|p| p.parent()) // target/<profile>/
		.map(|p| p.join(bin_name))
		.unwrap_or_else(|| bin_name.into())
}
const TOKEN_PORT:u16 = 34798;

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
fn spawn_proxy() -> Proxy {
	let listen = format!("127.0.0.1:{}", TOKEN_PORT);
	let db_path = std::env::temp_dir().join(format!("aphrodite_bench_05_{}.db", TOKEN_PORT));
	// SQLite persists the CCR store across process restarts - remove any
	// leftover file so stale entries can't poison this run's ratio numbers
	// (same isolation trick as bench_01/02/04).
	let _ = std::fs::remove_file(&db_path);
	let child = Command::new(bin_path())
		.args([
			"--mode",
			"token",
			"--listen",
			&listen,
			"--api-url",
			"http://127.0.0.1:1",
			"--api-key",
			"bench",
			"--ccr-db-path",
		])
		.arg(&db_path)
		// A repo-root aphrodite.toml would override --mode/--listen/etc.
		.env("APHRODITE_CONFIG_PATH", "/nonexistent/aphrodite-bench.toml")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap_or_else(|_| panic!("spawn failed - run `cargo build --release` first"));
	let dl = Instant::now() + Duration::from_secs(5);
	loop {
		if std::net::TcpStream::connect(("127.0.0.1", TOKEN_PORT)).is_ok() {
			break;
		}
		assert!(Instant::now() < dl, "proxy :{} did not start in time", TOKEN_PORT);
		std::thread::sleep(Duration::from_millis(50));
	}
	eprintln!("[bench_05] token proxy up on :{}", TOKEN_PORT);
	Proxy { child, port:TOKEN_PORT }
}

fn ccr_create(port:u16, content:&str) -> Option<serde_json::Value> {
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
	serde_json::from_slice(&out.stdout).ok()
}

fn ccr_retrieve(port:u16, hash:&str) -> bool {
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
		.ok()
		.and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok());
	out.and_then(|v| v.get("found").and_then(|f| f.as_bool())).unwrap_or(false)
}

// ── corpus: one authored sample per content type ───────────────────

struct Sample {
	label:&'static str,
	content:String,
}

fn corpus() -> Vec<Sample> {
	vec![
		Sample {
			label: "build",
			content: "   Compiling aphrodite v1.4.3\n   Compiling headroom-core v0.1.2\n   Compiling reqwest v0.13.4\n".repeat(45) + "    Finished `release` profile [optimized] target(s) in 13.41s\n",
		}, // ~3.2 KB
		Sample {
			label: "diff",
			content: format!(
				"diff --git a/src/proxy.rs b/src/proxy.rs\nindex abc1234..def5678 100644\n--- a/src/proxy.rs\n+++ b/src/proxy.rs\n{}",
				(0..80)
					.map(|i| format!("-    let old = {};\n+    let new = {};\n", i, i + 1))
					.collect::<String>()
			),
		}, // ~2.8 KB
		Sample {
			label: "git",
			content: (0..60)
				.map(|i| {
					let code = ["M ", "A ", "D ", "R ", "??", "UU"][i % 6];
					format!("{} src/module_{:03}.rs\n", code, i)
				})
				.collect::<String>(),
		}, // ~1.4 KB git status porcelain
		Sample {
			label: "gitlog",
			content: (0..15)
				.map(|i| {
					format!(
						"commit 9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e\nAuthor: Dev <dev@example.com>\nDate:   Mon Sep {:02} 12:00:00 2026 +0000\n\n    fix: handle {} in proxy path\n",
						15 + i, i
					)
				})
				.collect::<String>(),
		}, // ~2.4 KB git log
		Sample {
			label: "ls",
			content: "total 128\ndrwxr-xr-x  11 user  staff   352 Sep 15 04:10 .\ndrwxr-xr-x   7 user  staff   224 Sep 15 04:11 ..\n".to_string()
				+ &(0..70)
					.map(|i| format!("-rw-r--r--   1 user  staff  {:>5} Sep 15 04:12 file_{:03}.rs", 100 + i * 25, i))
					.collect::<Vec<_>>()
					.join("\n"),
		}, // ~4.6 KB ls -l
		Sample {
			label: "test",
			content: "running 330 tests\n".to_string()
				+ &(0..40)
					.map(|i| format!("test module_{}::tests::test_case_{} ... ok\n", i % 8, i))
					.collect::<String>()
				+ "test result: ok. 329 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s\n",
		}, // ~2.9 KB cargo test
		Sample {
			label: "grep",
			content: (0..90)
				.map(|i| format!("src/proxy.rs:{}:    let handler_{} = route;\n", i * 13 + 5, i))
				.collect::<String>(),
		}, // ~2.6 KB ripgrep hits
		Sample {
			label: "code_rust",
			content: include_str!("bench_payloads/sample.rs").to_string(),
		}, // ~3.5 KB
		Sample {
			label: "code_python",
			content: "#!/usr/bin/env python3\nimport sys\nimport json\n\ndef main():\n    data = json.loads(sys.stdin.read())\n    result = process(data)\n    print(json.dumps(result))\n\ndef process(data):\n    out = []\n    for item in data:\n        out.append({\"id\": item[\"id\"], \"name\": item[\"name\"].upper()})\n    return out\n\nclass DataHandler:\n    def __init__(self, limit=100):\n        self.limit = limit\n        self.items = []\n    def add(self, item):\n        if len(self.items) < self.limit:\n            self.items.append(item)\n    def flush(self):\n        result = self.items.copy()\n        self.items.clear()\n        return result\n\nif __name__ == \"__main__\":\n    main()\n".repeat(8),
		}, // ~3 KB
		Sample {
			label: "code_go",
			content: "package main\n\nimport (\n    \"fmt\"\n    \"net/http\"\n    \"sync\"\n)\n\ntype Cache struct {\n    mu    sync.RWMutex\n    items map[string]string\n}\n\nfunc NewCache() *Cache {\n    return &Cache{items: make(map[string]string)}\n}\n\nfunc (c *Cache) Get(key string) (string, bool) {\n    c.mu.RLock()\n    defer c.mu.RUnlock()\n    v, ok := c.items[key]\n    return v, ok\n}\n\nfunc main() {\n    cache := NewCache()\n    cache.Set(\"hello\", \"world\")\n    http.HandleFunc(\"/health\", func(w http.ResponseWriter, r *http.Request) {\n        w.WriteHeader(200)\n    })\n}\n".repeat(8),
		}, // ~3 KB
		Sample {
			label: "code_js",
			content: "import { useState, useEffect } from 'react';\n\nfunction useDebounce(value, delay = 300) {\n    const [debounced, setDebounced] = useState(value);\n    useEffect(() => {\n        const timer = setTimeout(() => setDebounced(value), delay);\n        return () => clearTimeout(timer);\n    }, [value, delay]);\n    return debounced;\n}\n\nconst API_BASE = process.env.API_URL || 'http://localhost:3000';\n\nasync function fetchData(endpoint) {\n    const res = await fetch(`${API_BASE}/${endpoint}`);\n    if (!res.ok) throw new Error(`HTTP ${res.status}`);\n    return res.json();\n}\n\nexport { useDebounce, fetchData };\n".repeat(12),
		}, // ~3 KB
		Sample {
			label: "json",
			content: serde_json::to_string_pretty(&serde_json::json!({
				"tool": "bench_05",
				"version": "1.4.3",
				"rows": (0..60).map(|i| serde_json::json!({"id": i, "label": format!("row_{}", i), "ratio": 1.0 + i as f64})).collect::<Vec<_>>(),
				"summary": {"compressed": 60, "total": 60}
			}))
			.unwrap(),
		}, // ~5 KB
		Sample {
			label: "error",
			content: format!(
				"thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 7', src/main.rs:42:17\nstack backtrace:\n{}",
				(0..30)
					.map(|i| format!("  {}: some::module::fn_{} at src/lib.rs:{}\n", i, i, i * 4 + 1))
					.collect::<String>()
			),
		}, // ~2.1 KB
		Sample {
			label: "log",
			content: (0..70)
				.map(|i| {
					format!(
						"2026-09-15T{:02}:{:02}:{:02}Z [INFO] request id=req-{:04} elapsed={}ms status=200\n",
						i / 3600, (i / 60) % 60, i % 60, i, i * 7 + 1
					)
				})
				.collect::<String>(),
		}, // ~4.4 KB
		Sample {
			label: "linter",
			content: (0..45)
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
			label: "text",
			content: "The Aphrodite compression engine provides context-aware CCR. ".repeat(240),
		}, // ~9 KB prose
	]
}

// ── run ────────────────────────────────────────────────────────────

#[derive(Default)]
struct Row {
	expected:&'static str,
	detected:String,
	semantic:Option<&'static str>,
	orig:usize,
	marker:usize,
	ratio:f64,
	compressed:bool,
	retrieve_ok:bool,
	latency_ms:u128,
}

fn classify(content:&str) -> (String, Option<&'static str>) {
	// `detect_type` = headroom classifier; `detect_semantic_type` = the
	// Aphrodite tool-output override that takes precedence in preview.rs.
	(
		aphrodite::preview::detect_type(content),
		aphrodite::preview::detect_semantic_type(content),
	)
}

fn run(proxy:&Proxy, samples:&[Sample]) -> Vec<Row> {
	let mut rows = Vec::new();
	for s in samples {
		let (detected, semantic) = classify(&s.content);
		let t0 = Instant::now();
		let res = ccr_create(proxy.port, &s.content);
		let latency = t0.elapsed().as_millis();
		let mut row = Row {
			expected:s.label,
			detected,
			semantic,
			orig:s.content.len(),
			latency_ms:latency,
			..Default::default()
		};
		if let Some(v) = res {
			let ratio = v.get("token_savings_ratio").and_then(|r| r.as_f64()).unwrap_or(1.0);
			row.marker = v.get("compressed_size").and_then(|c| c.as_u64()).unwrap_or(row.orig as u64) as usize;
			row.compressed = ratio > 1.05;
			row.ratio = if row.compressed { row.orig as f64 / row.marker.max(1) as f64 } else { 1.0 };
			if row.compressed {
				if let Some(hash) = v.get("hash").and_then(|h| h.as_str()) {
					row.retrieve_ok = ccr_retrieve(proxy.port, hash);
					if !row.retrieve_ok {
						eprintln!("  MISS  [{}] hash={}", s.label, &hash[..8.min(hash.len())]);
					}
				}
			}
		} else {
			eprintln!("  ERROR [{}] ccr/create failed", s.label);
		}
		rows.push(row);
	}
	rows
}

fn effective_type(row:&Row) -> String {
	// Precedence mirrors preview.rs: semantic override wins over headroom.
	row.semantic.unwrap_or(&row.detected).to_string()
}

fn print_report(rows:&[Row]) {
	eprintln!("\n{}", "─".repeat(88));
	eprintln!("  content-type coverage: {} samples", rows.len());
	eprintln!(
		"{:<12} {:<14} {:<8} {:>8} {:>8} {:>8} {:>8} {:>8}",
		"type", "classifier", "note", "orig_B", "mark_B", "ratio", "retrieve", "lat_ms"
	);
	eprintln!("{}", "─".repeat(88));
	let mut coarse = 0usize;
	for r in rows {
		let eff = effective_type(r);
		// The classifier column shows the in-process verdict (headroom
		// `detect_type` + semantic override). The proxy's fine-grained
		// classifier (`proxy_detect_content_type`) is private and not
		// exposed over HTTP, so a coarse verdict here (e.g. `text` for a
		// JSON blob) is expected, not a failure - the bench's gate is
		// compression + round-trip coverage per type, below.
		let note = if eff == r.expected
			|| (r.expected == "json" && eff == "json_array")
			|| (r.expected == "build" && eff == "build")
			|| (r.expected == "grep" && eff == "grep")
		{
			"exact"
		} else {
			coarse += 1;
			"coarse"
		};
		eprintln!(
			"{:<12} {:<14} {:<8} {:>8} {:>8} {:>8} {:>8} {:>8}",
			r.expected,
			eff,
			note,
			r.orig,
			if r.compressed { r.marker } else { 0 },
			if r.compressed { format!("{:.2}x", r.ratio) } else { "-".into() },
			if r.compressed { if r.retrieve_ok { "OK" } else { "MISS" } } else { "skip" },
			r.latency_ms
		);
	}
	let compr:Vec<&Row> = rows.iter().filter(|r| r.compressed).collect();
	let total_orig:usize = compr.iter().map(|r| r.orig).sum();
	let total_mark:usize = compr.iter().map(|r| r.marker).sum();
	eprintln!("{}", "─".repeat(88));
	eprintln!(
		"  compressed={} passthrough={}  retrieve hits={} misses={}  coarse-classifier={}",
		compr.len(),
		rows.len() - compr.len(),
		compr.iter().filter(|r| r.retrieve_ok).count(),
		compr.iter().filter(|r| !r.retrieve_ok).count(),
		coarse
	);
	if total_mark > 0 {
		eprintln!(
			"  overall ratio: {:.2}x  ({} B -> {} B)",
			total_orig as f64 / total_mark as f64,
			total_orig,
			total_mark
		);
	}
}

fn main() {
	let samples = corpus();
	eprintln!(
		"[bench_05] corpus: {} samples, {} total bytes",
		samples.len(),
		samples.iter().map(|s| s.content.len()).sum::<usize>()
	);

	let proxy = spawn_proxy();
	let rows = run(&proxy, &samples);
	print_report(&rows);

	// Coverage gate: every sample must compress (ratio > 1.05) AND
	// round-trip. Coarse in-process classification is informational only
	// (see print_report) - the proxy's own fine-grained classifier is not
	// exposed over HTTP, so it cannot be asserted from here.
	let misses:usize = rows.iter().filter(|r| r.compressed && !r.retrieve_ok).count();
	let failures:usize = rows.iter().filter(|r| !r.compressed).count();
	drop(proxy);
	if misses > 0 || failures > 0 {
		eprintln!("\n[bench_05] FAILED - uncompressed={} retrieve miss(es)={}", failures, misses);
		std::process::exit(1);
	}
	eprintln!("\n[bench_05] OK");
}
