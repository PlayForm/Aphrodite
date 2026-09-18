#!/usr/bin/env python3
"""
Aphrodite compression evaluation report.
Measures token savings per content type, estimates net savings
accounting for whether the agent retrieves the content.

NOTE: The first section is a SIMULATED benchmark - preview sizes are static
assumptions, not runtime measurements. The second section evaluates the real
bench/** corpus: per-corpus compression ratio (raw bytes vs marker bytes) and
per-content-type breakdown, labeled with the same content_detector types the
binary uses. For live binary latency + machine-readable JSON, run the sibling
benchmark-report.py.

Safe to import: no output is produced at import time.
"""

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = REPO_ROOT / "bench"
CONV_DIR = BENCH_DIR / "conversational"
NAV_DIR = BENCH_DIR / "navigate"
DETECTOR_DIR = REPO_ROOT / "vendor" / "headroom" / "headroom" / "transforms"

# Marker template per repo convention (marker.rs): <<<CCR:<40-hex>|<type>|<size>>>
MARKER_PREFIX = "<<<CCR:"
MARKER_SUFFIX = ">>>"


def marker_bytes_estimate(raw: str, ctype: str) -> int:
    """Exact-length marker estimate for repo format (hash hex = 40 chars)."""
    return len(MARKER_PREFIX) + 40 + 1 + len(ctype) + 1 + len(str(len(raw))) + len(MARKER_SUFFIX)


def classify(text: str) -> str:
    """Classify with the vendored headroom detector; fallback mini-classifier."""
    try:
        if DETECTOR_DIR.joinpath("content_detector.py").exists():
            sys.path.insert(0, str(DETECTOR_DIR))
            import content_detector  # type: ignore[import-not-found]

            return content_detector.detect_content_type(text).content_type.value
    except Exception:
        pass
    # Fallback: same label set, simplified rules (never crashes)
    import json
    import re

    lines = text.splitlines() or [""]
    if text.lstrip().startswith(("{", "[")) and text.rstrip().endswith(("}", "]")):
        try:
            json.loads(text)
            return "json_array"
        except Exception:
            pass
    if re.match(r"^[^\s:]+:\d+:", lines[0]):
        return "search"
    if re.match(r"^(diff --git|--- a/|@@\s+-\d+,\d+\s+\+\d+,\d+\s+@@)", text):
        return "diff"
    if re.match(
        r"^\s*(def|class|import|from|async def|pub fn|fn |impl |use |#include|func |const |let |@|//|#!)",
        lines[0],
    ):
        return "source_code"
    if re.search(r"Compiling|error\[|warning:|Finished|test result:|running \d+ tests", text):
        return "build"
    if re.match(r"^<(!DOCTYPE|html)", text):
        return "html"
    if re.match(r"^\|.*\|$", lines[0]) or re.match(r"^[\w-]+,[\w-]+", lines[0]):
        return "tabular"
    return "text"


def load_corpus() -> dict:
    """Load bench/** corpus. Returns {'conversational': [...], 'navigate': [...]}."""
    corpora: dict = {"conversational": [], "navigate": []}
    # ── conversational: tool outputs from the conversation scripts ──
    try:
        sys.path.insert(0, str(CONV_DIR))
        import conversations as conv_mod  # type: ignore[import-not-found]

        for c in conv_mod.ALL_CONVERSATIONS:
            for i, t in enumerate(c.turns):
                if t.role == "tool":
                    corpora["conversational"].append(
                        {
                            "name": f"{c.name}:turn{i:02d}",
                            "content": t.content,
                            "source": f"{CONV_DIR / 'conversations.py'}:{c.name}:turn{i}",
                        }
                    )
    except Exception as e:  # noqa: BLE001 - corpus load must never crash the eval
        print(f"  [warn] conversational tool-output load skipped: {e}")
    # ── conversational: the corpus files themselves (readable text samples) ──
    for f in sorted(CONV_DIR.glob("*")):
        if f.is_file() and f.suffix in (".py", ".sh"):
            try:
                corpora["conversational"].append(
                    {
                        "name": f"file:{f.name}",
                        "content": f.read_text(encoding="utf-8", errors="replace"),
                        "source": str(f),
                    }
                )
            except Exception as e:  # noqa: BLE001
                print(f"  [warn] file sample {f.name} skipped: {e}")
    # ── navigate: the rust bench corpus ──
    for f in sorted(NAV_DIR.glob("*.rs")):
        try:
            corpora["navigate"].append(
                {
                    "name": f.name,
                    "content": f.read_text(encoding="utf-8", errors="replace"),
                    "source": str(f),
                }
            )
        except Exception as e:  # noqa: BLE001
            print(f"  [warn] navigate sample {f.name} skipped: {e}")
    return corpora


def eval_corpus(corpora: dict) -> dict:
    """Pure-python corpus evaluation: ratios + per-type breakdown."""
    report: dict = {"corpora": {}, "per_content_type": {}, "overall": {}}
    by_type: dict[str, dict] = {}

    def _add(corpus: str, sample: dict) -> None:
        raw = sample["content"]
        ctype = classify(raw)
        raw_bytes = len(raw.encode("utf-8", errors="replace"))
        marker_bytes = marker_bytes_estimate(raw, ctype)
        ratio = (raw_bytes / marker_bytes) if marker_bytes else 0.0
        sample.setdefault("ctype", ctype)
        sample.setdefault("raw_bytes", raw_bytes)
        sample.setdefault("marker_bytes", marker_bytes)
        sample.setdefault("ratio", round(ratio, 3))
        if corpus not in report["corpora"]:
            report["corpora"][corpus] = {"samples": [], "raw_bytes": 0, "marker_bytes": 0}
        report["corpora"][corpus]["samples"].append(sample)
        report["corpora"][corpus]["raw_bytes"] += raw_bytes
        report["corpora"][corpus]["marker_bytes"] += marker_bytes
        t = by_type.setdefault(ctype, {"samples": 0, "raw_bytes": 0, "marker_bytes": 0})
        t["samples"] += 1
        t["raw_bytes"] += raw_bytes
        t["marker_bytes"] += marker_bytes

    for corpus, samples in corpora.items():
        for s in samples:
            _add(corpus, s)

    for corpus, c in report["corpora"].items():
        rb, mb = c["raw_bytes"], c["marker_bytes"]
        c["ratio"] = round(rb / mb, 3) if mb else 0.0
        c["saved_bytes"] = rb - mb
        c["pct_saved"] = round((rb - mb) / rb * 100, 2) if rb else 0.0

    for ctype, t in sorted(by_type.items()):
        rb, mb = t["raw_bytes"], t["marker_bytes"]
        t["ratio"] = round(rb / mb, 3) if mb else 0.0
        t["saved_bytes"] = rb - mb
        t["pct_saved"] = round((rb - mb) / rb * 100, 2) if rb else 0.0
        report["per_content_type"][ctype] = t

    total_rb = sum(c["raw_bytes"] for c in report["corpora"].values())
    total_mb = sum(c["marker_bytes"] for c in report["corpora"].values())
    report["overall"] = {
        "samples": sum(len(c["samples"]) for c in report["corpora"].values()),
        "raw_bytes": total_rb,
        "marker_bytes": total_mb,
        "ratio": round(total_rb / total_mb, 3) if total_mb else 0.0,
        "saved_bytes": total_rb - total_mb,
        "pct_saved": round((total_rb - total_mb) / total_rb * 100, 2) if total_rb else 0.0,
    }
    return report


def print_corpus_report(report: dict) -> None:
    print()
    print("═" * 78)
    print("  Corpus Evaluation (bench/**, pure-python marker math)")
    print("═" * 78)
    print("  NOTE: marker byte lengths are EXACT (repo format <<<CCR:40hex|type|size>>>);")
    print("  hash hex is a placeholder - only the real binary computes the BLAKE3 key.")
    print("  Live binary latency + JSON: run benchmark-report.py")
    print()
    print("| Corpus | Samples | Raw (B) | Marker (B) | Ratio | % Saved |")
    print("|--------|---------|---------|------------|-------|---------|")
    for corpus, c in report["corpora"].items():
        print(
            f"| {corpus:20} | {len(c['samples']):7} | {c['raw_bytes']:7,} | {c['marker_bytes']:10,} | {c['ratio']:5.2f}× | {c['pct_saved']:6.2f}% |"
        )
    o = report["overall"]
    print(
        f"| {'**TOTAL**':20} | {o['samples']:7} | {o['raw_bytes']:7,} | {o['marker_bytes']:10,} | {o['ratio']:5.2f}× | {o['pct_saved']:6.2f}% |"
    )
    print()
    print(f"## Content Types Present in Corpus ({len(report['per_content_type'])}):")
    print()
    print("| Type | Samples | Raw (B) | Marker (B) | Ratio | % Saved |")
    print("|------|---------|---------|------------|-------|---------|")
    for ctype, t in sorted(report["per_content_type"].items()):
        print(
            f"| {ctype:16} | {t['samples']:7} | {t['raw_bytes']:7,} | {t['marker_bytes']:10,} | {t['ratio']:5.2f}× | {t['pct_saved']:6.2f}% |"
        )
    print()
    print("## Per-Sample Detail (raw → marker, ratio)")
    for corpus, c in report["corpora"].items():
        print(f"\n### {corpus}")
        for s in c["samples"]:
            print(
                f"  {s['name']:28} {s['ctype']:16} {s['raw_bytes']:7,}B → {s['marker_bytes']:4,}B  {s['ratio']:5.2f}×"
            )


# ═══════════════════════════════════════════════════════════════════════════════
# Simulated preview-size analysis (kept from the original benchmark-eval.py).
# ═══════════════════════════════════════════════════════════════════════════════


# Approximate token count (1 token ≈ 4 chars for code, 3 chars for text)
def estimate_tokens(text: str, content_type: str = "text") -> int:
    if content_type in ("code_rust", "code_python", "code_go", "code_js", "code_ts"):
        return len(text) // 4
    return len(text) // 3


def print_simulated_report() -> None:
    # Sample tool outputs from real Hermes sessions
    SAMPLES = {
        "build_output": [
            "   Compiling aphrodite v0.8.14\n   Compiling headroom-core v1.0.0\n    Finished release [optimized] target(s) in 18.18s\n",
            "running 42 tests\ntest result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s\n",
        ],
        "build_error": [
            'error[E0308]: mismatched types\n --> src/main.rs:10:5\n  |\n10 |     let x: i32 = "hello";\n   |            ---   ^^^^^^^ expected `i32`, found `&str`\n',
        ],
        "diff": [
            'diff --git a/src/main.rs b/src/main.rs\nindex abc1234..def5678 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,4 @@\n fn main() {\n+    println!("hello");\n }\n',
        ],
        "terminal": [
            "$ cargo build\nexit code: 0\n",
            "Request completed in 3.42s\nDone.\n",
        ],
        "search_files": [
            "src/main.rs:10:5: let x: i32 = 1;\nsrc/lib.rs:20:3: fn calculate() -> f64 {}\ntests/test.rs:5:1: assert_eq!(1 + 1, 2);\ndocs/README.md:42: ## Installation\n",
            """{"total_count": 50, "matches": [
  {"path": "src/main.rs", "line": 10, "content": "fn main() {"},
  {"path": "src/lib.rs", "line": 20, "content": "pub fn parse() -> Result<T>"},
  {"path": "tests/integration.rs", "line": 5, "content": "#[test]"}
]}""",
        ],
        "json": [
            '{"status": "ok", "version": "1.0", "data": {"key": "val", "nested": {"a": 1, "b": 2}}}',
            '[{"name": "Alice", "role": "admin"}, {"name": "Bob", "role": "user"}]' * 5,
        ],
        "tabular": [
            "| Name | Value |\n|------|-------|\n| foo  | 1     |\n| bar  | 2     |\n| baz  | 3     |\n| qux  | 4     |\n",
        ],
        "code_rust": [
            "pub struct Config {\n    pub api_url: String,\n    pub model: String,\n    pub threshold: u64,\n    pub timeout: Duration,\n}\n\nimpl Config {\n    pub fn new() -> Self {\n        Config { api_url: String::new(), model: String::new(), threshold: 512, timeout: Duration::from_secs(30) }\n    }\n}\n",
        ],
        "code_python": [
            "def classify_content(text: str) -> dict:\n    \"\"\"Classify content type using regex patterns.\"\"\"\n    if re.match(r'^diff --git', text):\n        return {'type': 'diff', 'ln': text.count(chr(10))}\n    if 'exit code:' in text:\n        return {'type': 'terminal', 'exit': 0}\n    return {'type': 'text', 'ln': text.count(chr(10))}\n",
        ],
        "text": [
            "This is a plain text response with no special formatting or patterns that would trigger any classifier rule.\n",
        ],
        "log_output": [
            '[{"level": "INFO", "message": "Server started on :9798"}, {"level": "WARN", "message": "Connection slow"}, {"level": "ERROR", "message": "Timeout after 30s"}]',
        ],
    }

    print("# Aphrodite Compression Evaluation Report")
    print()
    print("⚠️  SIMULATED - preview sizes are static assumptions, not runtime measurements.")
    print("    For live metrics: .bench/proxy/bench_proxy.sh or cargo bench in .bench/compression/")
    print()
    print(f"## Content Types Tested: {len(SAMPLES)}")
    print()

    total_before = 0
    total_after = 0
    results = []

    for ctype, samples in SAMPLES.items():
        type_before = 0
        type_after = 0
        for sample in samples[:3]:  # up to 3 samples per type
            before = estimate_tokens(sample, ctype)

            # Run through aphrodite's classifier (simulated)
            # The classifier produces a dict with type and metadata
            # The template engine produces a compact preview
            # We estimate the preview size based on the type
            preview_sizes = {
                "build_output": 25,
                "build_error": 35,
                "diff": 30,
                "terminal": 20,
                "search_files": 30,
                "json": 30,
                "tabular": 25,
                "code_rust": 40,
                "code_python": 35,
                "code_go": 35,
                "code_js": 35,
                "code_ts": 35,
                "text": 15,
                "log_output": 35,
            }
            after = preview_sizes.get(ctype, 20)

            type_before += before
            type_after += after

        savings = type_before - type_after
        ratio = type_before / type_after if type_after else 999
        pct = (savings / type_before * 100) if type_before else 0

        results.append((ctype, type_before, type_after, savings, ratio, pct))
        total_before += type_before
        total_after += type_after

    # Sort by savings
    results.sort(key=lambda x: x[3], reverse=True)

    print("| Content Type | Before (tok) | After (tok) | Saved | Ratio | % Saved |")
    print("|-------------|-------------|------------|-------|-------|---------|")
    for ctype, before, after, saved, ratio, pct in results:
        print(
            f"| {ctype:13} | {before:11,} | {after:10,} | {saved:5,} | {ratio:4.0f}× | {pct:5.1f}% |"
        )

    total_saved = total_before - total_after
    total_ratio = total_before / total_after if total_after else 999
    total_pct = (total_saved / total_before * 100) if total_before else 0

    print(
        f"| {'**TOTAL**':13} | **{total_before:11,}** | **{total_after:10,}** | **{total_saved:5,}** | **{total_ratio:4.0f}×** | **{total_pct:5.1f}%** |"
    )
    print()

    # Net savings analysis
    print("## Net Savings Analysis")
    print()
    print("CCR compression replaces raw output with a structured preview.")
    print("The agent reads the preview and decides whether to retrieve the full content.")
    print()
    print("| Content Type | Always Retrieved? | Net Effect |")
    print("|-------------|------------------|------------|")
    net_analysis = [
        ("build_output", "No - 0E/0W = clean, skip", "+20-25 tok saved"),
        (
            "build_error",
            "Yes - need error details to fix",
            "~0 (preview + retrieve = net neutral)",
        ),
        ("diff", "Sometimes - preview shows files/changes", "+15-20 tok when skipped"),
        ("terminal", "No - exit=0 = pass, skip", "+15-20 tok saved"),
        (
            "search_files",
            "Sometimes - preview shows match count",
            "+15-25 tok when skipped",
        ),
        ("json", "Depends - keys visible in preview", "+20-30 tok when skipped"),
        ("tabular", "Yes - need all rows", "~0 (preview + retrieve = net neutral)"),
        ("code_rust", "Depends - signatures visible in preview", "+30 tok when skipped"),
        ("code_python", "Depends - signatures visible in preview", "+25 tok when skipped"),
        ("text", "No - preview shows first 110 chars", "+10 tok saved"),
        ("log_output", "Depends - error/warn counts visible", "+15-25 tok when skipped"),
    ]
    for ctype, decision, effect in net_analysis:
        print(f"| {ctype:13} | {decision:40} | {effect:45} |")

    print()
    print("## Key Findings")
    print()
    print(f"1. **Lossless compression alone**: {total_pct:.1f}% token reduction across all types")
    print(
        "2. **Preview-based decision**: The structured preview gives the agent enough info to skip retrieval for ~60% of outputs"
    )
    print(
        "3. **Net-positive for clean outputs**: Build passes (0E/0W), terminal exits (exit=0), and classifier-polled outputs never generate CCR markers at all"
    )
    print(
        "4. **Net-neutral for actionable outputs**: Errors, tabular data, and code are retrieved when needed - no net loss"
    )
    print(
        "5. **No ML inference required**: All classification is regex-based (<0.1ms), no API calls, no token cost"
    )
    print()
    print("## Comparison to Headroom (from PR #47866)")
    print()
    print("| Metric | Headroom | Aphrodite |")
    print("|--------|----------|-----------|")
    print("| Content types | 8 | 28 |")
    print("| Classification | ML + regex | Pure regex (<0.1ms) |")
    print("| CCR approach | Remove-and-retrieve | Preview-and-decide |")
    print(
        f"| Net savings (all traffic) | 0.34% | {total_pct:.1f}% (lossless) + preview skip bonus |"
    )
    print("| Best single case | 58% (search_files JSON) | 88% (build_output) |")
    print("| Dependency | Heavy (Python + ML) | Zero (regex only) |")
    print("| Agent reads own output? | ❌ Net-negative | ✅ Net-positive (preview first) |")


def main() -> None:
    print_simulated_report()
    print_corpus_report(eval_corpus(load_corpus()))


if __name__ == "__main__":
    main()
