#!/usr/bin/env python3
"""Regenerate the deterministic benchmark-corpus fixtures.

Run from anywhere: files are written next to this script.

Migration notes (archive corpus -> Development bench/corpus):
  * Fixtures are synthetic SAMPLES of content types, NOT measurements:
    no benchmark numbers are claimed anywhere in the corpus.
  * Content is anonymized: no usernames, emails, tokens, secrets, or
    machine-specific paths. All paths shown are workspace-relative
    (crates/aphrodite/src/...) or generic /home/user/ placeholders.
  * build_log.txt is a modern cargo build log for the aphrodite v1.4.3
    workspace under rustc 1.88; git_diff.patch references current file
    names under crates/aphrodite/src/.
  * code_rust.rs is copied from crates/aphrodite/examples/bench_payloads/
    sample.rs -- the Maintain/examples/bench_payloads/ path referenced by
    the original archive README no longer exists on Development, so the
    example now lives under crates/aphrodite/examples/. If the example is
    absent the embedded SAMPLE_FALLBACK (canonical archive-era copy) is
    used instead, keeping regeneration deterministic.
"""

import json
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
random.seed(0xCC12)

WORDS = (
    "compression proxy marker retrieve latency baseline corpus semantic "
    "reduction pipeline stage classifier preview hash inline store token "
    "cache context engine threshold budget eviction structured extraction "
    "diff build signature nested resolve expand recursive"
).split()


def text_blob(target_bytes: int) -> str:
    lines, size = [], 0
    i = 0
    while size < target_bytes:
        n = 6 + (i % 9)
        line = f"{i:06d} " + " ".join(random.choice(WORDS) for _ in range(n))
        lines.append(line)
        size += len(line) + 1
        i += 1
    return "\n".join(lines)[:target_bytes]


def write(name: str, data, binary=False):
    path = os.path.join(HERE, name)
    if binary:
        with open(path, "wb") as f:
            f.write(data)
    else:
        with open(path, "w", encoding="utf-8") as f:
            f.write(data)
    print(f"{name}: {os.path.getsize(path)} bytes")


# ── code_rust.rs: copied from the current bench-payload example ──────
# The example moved from Maintain/examples/bench_payloads/ (archive-README
# path, now deleted) to crates/aphrodite/examples/bench_payloads/.
SAMPLE_CANDIDATES = (
    os.path.join(REPO_ROOT, "crates", "aphrodite", "examples", "bench_payloads", "sample.rs"),
    os.path.join(REPO_ROOT, "Maintain", "examples", "bench_payloads", "sample.rs"),
)


def rust_sample() -> str:
    for path in SAMPLE_CANDIDATES:
        if os.path.isfile(path):
            with open(path, "r", encoding="utf-8") as fh:
                return fh.read()
    return SAMPLE_FALLBACK


SAMPLE_FALLBACK = r"""//! Rust source payload for bench_01_corpus.
//! ~3.5 KB - above TOKEN_COMPRESS_THRESHOLD (1 KB), below CACHE_COMPRESS_THRESHOLD (8 KB).
//! Expected: compressed in token mode, passthrough in cache mode.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Store {
	data: Mutex<HashMap<String, String>>,
	cap: usize,
	order: Mutex<Vec<String>>,
}

impl Store {
	pub fn new(cap: usize) -> Arc<Self> {
		Arc::new(Self { data: Mutex::new(HashMap::new()), cap, order: Mutex::new(Vec::new()) })
	}
	pub fn get(&self, k: &str) -> Option<String> {
		let d = self.data.lock().unwrap();
		if let Some(v) = d.get(k) {
			let mut o = self.order.lock().unwrap();
			o.retain(|x| x != k);
			o.push(k.to_string());
			Some(v.clone())
		} else {
			None
		}
	}
	pub fn put(&self, k: &str, v: &str) {
		let mut d = self.data.lock().unwrap();
		let mut o = self.order.lock().unwrap();
		if d.contains_key(k) {
			o.retain(|x| x != k);
		} else if d.len() >= self.cap {
			if let Some(evict) = o.first().cloned() {
				o.remove(0);
				d.remove(&evict);
			}
		}
		d.insert(k.to_string(), v.to_string());
		o.push(k.to_string());
	}
	pub fn del(&self, k: &str) -> bool {
		let mut d = self.data.lock().unwrap();
		let mut o = self.order.lock().unwrap();
		o.retain(|x| x != k);
		d.remove(k).is_some()
	}
	pub fn len(&self) -> usize {
		self.data.lock().unwrap().len()
	}
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

pub fn fnv1a(data: &[u8]) -> u64 {
	let mut h: u64 = 14695981039346656037;
	for b in data {
		h ^= *b as u64;
		h = h.wrapping_mul(1099511628211);
	}
	h
}

pub struct BatchProcessor {
	store: Arc<Store>,
	workers: usize,
}
impl BatchProcessor {
	pub fn new(store: Arc<Store>, workers: usize) -> Self {
		Self { store, workers }
	}
	pub fn process_batch(&self, items: &[(String, String)]) -> usize {
		let chunk = (items.len() + self.workers - 1) / self.workers;
		let done = Arc::new(Mutex::new(0usize));
		std::thread::scope(|s| {
			for slice in items.chunks(chunk) {
				let st = self.store.clone();
				let dn = done.clone();
				let sl = slice.to_vec();
				s.spawn(move || {
					for (k, v) in &sl {
						st.put(k, v);
						*dn.lock().unwrap() += 1;
					}
				});
			}
		});
		*done.lock().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn put_get() {
		let s = Store::new(4);
		s.put("a", "1");
		assert_eq!(s.get("a"), Some("1".into()));
	}
	#[test]
	fn eviction() {
		let s = Store::new(2);
		s.put("a", "1");
		s.put("b", "2");
		s.put("c", "3");
		assert_eq!(s.get("a"), None);
		assert_eq!(s.get("b"), Some("2".into()));
	}
	#[test]
	fn fnv_known() {
		assert_eq!(fnv1a(b"hello"), 0xa430d84680aabd0b);
	}
}
"""


# ── plain text at four sizes ─────────────────────────────────────────
write("text_1kb.txt", text_blob(1 * 1024))
write("text_10kb.txt", text_blob(10 * 1024))
write("text_100kb.txt", text_blob(100 * 1024))
write("text_500kb.txt", text_blob(500 * 1024))

# ── deep JSON: 20 nesting levels ─────────────────────────────────────
deep = {"leaf": "bottom", "items": list(range(16))}
for lvl in range(20, 0, -1):
    deep = {f"level_{lvl}": deep, "meta": f"depth {lvl}", "n": lvl}
write("deep_nested_20.json", json.dumps(deep, indent=1))

# ── wide JSON: 5000 top-level keys ───────────────────────────────────
wide = {f"key_{i:05d}": {"v": i, "w": random.choice(WORDS)} for i in range(5000)}
write("wide_5k_keys.json", json.dumps(wide, indent=0))

# ── build log: modern cargo build of the aphrodite workspace ─────────
CRATES = [
    ("aphrodite-types", "1.4.3"),
    ("aphrodite-core", "1.4.3"),
    ("aphrodite-http", "1.4.3"),
    ("aphrodite-cache", "1.4.3"),
    ("aphrodite", "1.4.3"),
    ("headroom-core", "0.3.2"),
    ("headroom-proxy", "0.3.2"),
    ("tokio", "1.45.0"),
    ("serde", "1.0.219"),
    ("serde_json", "1.0.145"),
    ("clap", "4.5.41"),
    ("thiserror", "2.0.12"),
]
SRC_FILES = ["compressor.rs", "marker.rs", "config.rs", "proxy.rs", "lib.rs"]
ERR_CODES = ["E0308", "E0599", "E0433", "E0502"]
ERR_MSGS = {
    "E0308": "mismatched types",
    "E0599": "no method named `compress2` found for struct `Compressor`",
    "E0433": "failed to resolve: use of undeclared crate or module `http`",
    "E0502": "cannot borrow `self.state` as mutable because it is also borrowed as immutable",
}
ERR_SHORT = {
    "E0308": "expected `usize`, found `Option<usize>`",
    "E0599": "help: items from traits can only be used if the trait is in scope",
    "E0433": "help: a similar name exists in the crate root: `http_extra`",
    "E0502": "help: consider cloning the value or using a different borrow scope",
}
blines = [
    "$ rustc --version && cargo build --release",
    "rustc 1.88.0",
    "   Compiling autocfg v1.4.0",
    "   Compiling libc v0.2.177",
    "   Compiling memchr v2.7.4",
    "   Compiling serde_derive v1.0.219",
]
err_count = warn_count = 0
for i in range(320):
    name, ver = CRATES[i % len(CRATES)]
    blines.append(f"   Compiling {name} v{ver}")
    if i % 13 == 0:
        code = ERR_CODES[err_count % len(ERR_CODES)]
        src = SRC_FILES[(err_count * 3) % len(SRC_FILES)]
        ln = 90 + (err_count * 7) % 900
        col = 17 + (err_count % 30)
        blines += [
            f"error[{code}]: {ERR_MSGS[code]}",
            f"  --> crates/aphrodite/src/{src}:{ln}:{col}",
            "   |",
            f"{ln:4d} |     let size: usize = marker.size();",
            f"   |                 ^^^^^^^ {ERR_SHORT[code]}",
            "   |",
            "",
        ]
        err_count += 1
    if i % 7 == 0:
        src = SRC_FILES[(warn_count * 5) % len(SRC_FILES)]
        ln = 40 + (warn_count * 11) % 700
        blines += [
            "warning: unused variable: `pending`",
            f"  --> crates/aphrodite/src/{src}:{ln}:9",
            "   |",
            f"{ln:4d} |     let pending = batch.drain(..window);",
            "   |         ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_pending`",
            "   |",
            "   = note: `#[warn(unused_variables)]` on by default",
            "",
        ]
        warn_count += 1
blines.append(
    f"error: could not compile `aphrodite` (lib) due to {err_count} previous errors; "
    f"{warn_count} warnings emitted"
)
blines += [
    "",
    "$ cargo test --lib",
    "   Compiling aphrodite v1.4.3 (crates/aphrodite)",
    "    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.41s",
    "     Running unittests src/lib.rs (target/debug/deps/aphrodite-9f0c2a41)",
    "running 214 tests",
]
for t in range(12):
    blines.append(f"test compressor::tests::case_{t:02d} ... ok")
blines += [
    "test result: ok. 214 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.84s",
    "",
    "   Doc-tests aphrodite",
    "running 12 tests",
    "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s",
]
write("build_log.txt", "\n".join(blines))

# ── git diff referencing current crates/aphrodite/src/ paths ─────────
DIFF_FILES = [
    ("crates/aphrodite/src/compressor.rs", 198),
    ("crates/aphrodite/src/marker.rs", 64),
    ("crates/aphrodite/src/config.rs", 30),
    ("crates/aphrodite/src/proxy.rs", 141),
    ("crates/aphrodite/src/lib.rs", 17),
]
dlines = []
for f_i, (name, start) in enumerate(DIFF_FILES):
    dlines += [
        f"diff --git a/{name} b/{name}",
        f"index {0x1A2B3C + f_i * 0x11111:07x}..{0x5D6E7F + f_i * 0x22222:07x} 100644",
        f"--- a/{name}",
        f"+++ b/{name}",
        f"@@ -{start},40 +{start},46 @@ impl Compressor {{",
    ]
    for h in range(40):
        if h % 5 == 0:
            dlines.append(f"-        let old_{h} = self.compute_{f_i}({h});")
            dlines.append(f"+        let new_{h} = self.compute_{f_i}_fast({h}, &ctx);")
        else:
            dlines.append(f"         let keep_{h} = self.value({h});")
    dlines.append("")
write("git_diff.patch", "\n".join(dlines) + "\n")

# ── pathological UTF-8: multibyte chars straddling 500-byte marks ────
# 3-byte '€' and 4-byte '🜲' runs offset by a single 'a' so that byte
# offsets 500, 1000, 1500, ... systematically land mid-codepoint.
# (wave-1 slice-panic regression class)
chunk = "a" + "€" * 700 + "b" + "🜲" * 500 + "é" * 300  # é = 2 bytes
write("utf8_boundary.txt", chunk * 4)

# ── literal CCR markers embedded in ordinary text (05-F1 class) ──────
marker_doc = (
    "Documentation about the marker format.\n"
    "A full marker looks like <<<CCR:deadbeefdeadbeefdeadbeefdeadbeefdeadbeef|source_code|4096>>>\n"
    "and the short form is [CCR:cafebabecafebabecafebabe|text].\n"
    "Unicode glyph form: \u2af7CCR:0123456789abcdef0123\u2af8 is also parsed.\n"
    "None of these hashes exist in any store - retrieval/expansion must\n"
    "leave this text byte-identical, never substitute or corrupt it.\n"
    "Edge cases: <<<CCR: (unclosed), <<<CCR:>>> (empty), "
    "<<<CCR:a<<<CCR:bbbbbbbbbbbbbbbbbbbbbbbb|t|1>>> (nested prefix).\n"
) * 20
write("marker_literal.txt", marker_doc)

# ── interior-NUL-bearing JSON (03-F3 class) ──────────────────────────
# Valid JSON whose decoded strings contain NUL () characters, plus
# a raw 0x00 byte appended after the JSON document (still valid UTF-8).
nul_obj = {
    "name": "nul\x00inside",
    "rows": [f"row_{i}\x00tail" for i in range(200)],
    "note": "decoded strings contain interior NUL bytes",
}
write(
    "interior_nul.json", json.dumps(nul_obj).encode() + b"\ntrailing\x00raw\x00bytes\n", binary=True
)

# ── source fixtures for struct_extract (python/go/ts) ────────────────
py = [
    "#!/usr/bin/env python3",
    '"""Fixture module for struct_extract benches."""',
    "from __future__ import annotations",
    "",
    "import os",
    "",
]
for i in range(60):
    py += [
        f"class Handler{i}:",
        f'    """Bench handler {i}."""',
        f"    def __init__(self, name: str, offset: int = {i}) -> None:",
        "        self.name = name",
        "        self.offset = offset",
        "",
        f"    def process_{i}(self, data: bytes, depth: int = 0) -> bytes:",
        f"        return data[self.offset:] + bytes([depth + {i}])",
        "",
        f"def util_{i}(a: int, b: int = {i}, *args: object, **kw: object) -> int:",
        f"    return a + b + {i}",
        "",
    ]
write("code_python.py", "\n".join(py))

go = [
    "// Package fixture provides realistic Go source for struct_extract benches.",
    "package fixture",
    "",
    'import "fmt"',
    "",
]
for i in range(60):
    go += [
        f"type Record{i} struct {{",
        "\tID   int64",
        "\tName string",
        "\tTags []string",
        "}",
        "",
        f"func Process{i}(r *Record{i}, depth int) (string, error) {{",
        f'\treturn fmt.Sprintf("%d-%d", r.ID, depth+{i}), nil',
        "}",
        "",
    ]
write("code_go.go", "\n".join(go))

ts = [
    "// Fixture module for struct_extract benches",
    "import { readFile } from 'node:fs/promises';",
    "",
]
for i in range(60):
    ts += [
        f"export interface Shape{i} {{ id: number; name: string; }}",
        f"export class Widget{i} {{",
        f"  constructor(private id: number = {i}) {{}}",
        f"  render(depth: number): string {{ return `w${{this.id}}:${{depth}}`; }}",
        "}",
        "",
        f"export async function load{i}(path: string): Promise<Shape{i}> {{",
        "  const raw = await readFile(path, 'utf8');",
        f"  return JSON.parse(raw) as Shape{i};",
        "}",
        "",
    ]
write("code_ts.ts", "\n".join(ts))

write("code_rust.rs", rust_sample())
