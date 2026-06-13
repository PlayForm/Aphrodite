#!/usr/bin/env python3
"""
Content-level compression comparison — inline plugin vs proxy vs CLI.

Tests all three compression modes with real tool outputs,
showing PRE-COMPRESS vs POST-COMPRESS content side-by-side.

Usage:
    python3 tests/bench_compare_content.py
    python3 tests/bench_compare_content.py --mode proxy    # proxy only
    python3 tests/bench_compare_content.py --mode inline   # inline only
    python3 tests/bench_compare_content.py --verbose       # full content dumps
"""

import json, os, subprocess, sys, time, urllib.request
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parent.parent
HEADROOM_BIN = REPO_DIR / ".venv" / "bin" / "headroom"
HEADROOM_CLI = REPO_DIR / ".venv" / "bin" / "hermes-compress"
DEEPSEEK_KEY = os.getenv("HEADROOM_DEEPSEEK_KEY", "")
DEEPSEEK_URL = "https://api.deepseek.com"
PROXY_PORT = 8787

VERBOSE = "--verbose" in sys.argv
MODE_FILTER = next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--mode=")), None)

# ── Test payloads (simulating real tool outputs) ─────────────────────

SHORT_TEXT = "File not found: /path/to/missing.py"

LONG_TERMINAL = json.dumps({
    "output": "\n".join(
        f"  file_{i:04d}.py    12345 bytes  Jun {10 + i % 20} 2026  "
        f"owner:nobody  group:staff  mode:0644"
        for i in range(80)
    ),
    "exit_code": 0,
})

LONG_CODE = "\n".join(
    f"{i:4d}|    def test_case_{i}(self):\n"
    f"{i:4d}|        \"\"\"Test case {i} — validates compression pipeline.\"\"\"\n"
    f"{i:4d}|        _result = self.compressor.compress(messages)\n"
    f"{i:4d}|        assert _result.tokens_saved > 0\n"
    for i in range(100, 200)
)

WEB_SEARCH_JSON = json.dumps({
    "results": [
        {
            "title": f"Result {i} — How to compress LLM context with headroom",
            "url": f"https://example.com/article/{i}",
            "description": "Learn about the latest techniques in context compression using "
                           "ONNX-based Kompress models. This approach reduces token usage "
                           "by 40-60% while preserving semantic meaning across diverse workloads.",
        }
        for i in range(8)
    ]
})

TEST_CASES = [
    {"name": "short_text", "tool": "read_file", "content": SHORT_TEXT, "label": "Short (skip)"},
    {"name": "terminal_long", "tool": "terminal", "content": LONG_TERMINAL, "label": "Terminal 80 files"},
    {"name": "code_long", "tool": "read_file", "content": LONG_CODE, "label": "Code 100 functions"},
    {"name": "web_search_json", "tool": "web_search", "content": WEB_SEARCH_JSON, "label": "Web search 8 results"},
]

# ── Inline compression ──────────────────────────────────────────────

def test_inline():
    """Compress via Compress class (in-process, library mode)."""
    from hermes_compress._compress import Compress, CompressOption

    option = CompressOption(
        Enabled=True,
        Mode="inline",
        ProtectRecent=1,
        MinTokensToCompress=100,
    )
    compressor = Compress(option=option, model="deepseek-v4-pro")

    results = []
    for tc in TEST_CASES:
        msg = {
            "role": "tool",
            "content": tc["content"],
            "tool_call_id": f"tc_{tc['name']}",
            "name": tc["tool"],
        }
        messages = [{"role": "user", "content": "test"}, msg]

        try:
            result = compressor.compress(messages)
            post_content = ""
            for m in result.messages:
                if m.get("role") == "tool":
                    post_content = m.get("content", "")
            results.append({
                "case": tc["name"],
                "label": tc["label"],
                "pre_len": len(tc["content"]),
                "post_len": len(post_content),
                "pre_content": tc["content"],
                "post_content": post_content,
                "mode": "inline",
                "error": result.error,
                "tokens_saved": result.tokens_saved,
            })
        except Exception as e:
            results.append({
                "case": tc["name"], "label": tc["label"],
                "pre_len": len(tc["content"]), "post_len": 0,
                "mode": "inline", "error": str(e),
            })
    return results


# ── CLI compression ─────────────────────────────────────────────────

def test_cli():
    """Compress via hermes-compress CLI (pipes text to stdin)."""
    if not HEADROOM_CLI.exists():
        return [{"case": tc["name"], "mode": "cli", "error": "CLI not found"} for tc in TEST_CASES]

    results = []
    for tc in TEST_CASES:
        try:
            p = subprocess.run(
                [str(HEADROOM_CLI), "compress", "--model", "deepseek-v4-pro", "--json"],
                input=tc["content"], capture_output=True, text=True, timeout=60,
            )
            if p.returncode == 0:
                out = json.loads(p.stdout)
                results.append({
                    "case": tc["name"], "label": tc["label"],
                    "pre_len": len(tc["content"]),
                    "post_len": out.get("tokens_after", 0),
                    "pre_content": tc["content"],
                    "post_content": f"[{out.get('tokens_after', 0)} tokens]",
                    "mode": "cli",
                    "error": None,
                    "tokens_saved": out.get("tokens_saved", 0),
                })
            else:
                results.append({
                    "case": tc["name"], "label": tc["label"],
                    "pre_len": len(tc["content"]), "post_len": 0,
                    "mode": "cli", "error": p.stderr[:200],
                })
        except Exception as e:
            results.append({
                "case": tc["name"], "mode": "cli", "error": str(e),
            })
    return results


# ── Proxy compression ───────────────────────────────────────────────

def proxy_health():
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PROXY_PORT}/health", timeout=3) as r:
            return json.loads(r.read()).get("ready", False)
    except Exception:
        return False


def test_proxy():
    """Send API call through proxy, capture pre/post stats + LLM response."""
    if not proxy_health():
        return [{"case": tc["name"], "mode": "proxy", "error": "Proxy not running"} for tc in TEST_CASES]

    results = []
    for tc in TEST_CASES:
        # Get stats BEFORE
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{PROXY_PORT}/stats", timeout=3) as r:
                pre_stats = json.loads(r.read())
        except Exception:
            pre_stats = {}

        pre_removed = pre_stats.get("summary", {}).get("compression", {}).get("total_tokens_removed", 0)

        # Build payload with the test content embedded
        payload = {
            "model": "deepseek-v4-flash",
            "messages": [
                {"role": "system", "content": f"You are testing compression. "
                 f"Echo the tool output length you see in the user message."},
                {"role": "user", "content": f"Tool output ({tc['tool']}):\n{tc['content']}\n\n"
                 f"How many characters was that tool output?"},
            ],
            "max_tokens": 50,
        }

        try:
            data = json.dumps(payload).encode()
            req = urllib.request.Request(
                f"http://127.0.0.1:{PROXY_PORT}/v1/chat/completions",
                data=data, method="POST",
                headers={
                    "Content-Type": "application/json",
                    "Authorization": f"Bearer {DEEPSEEK_KEY}",
                },
            )
            with urllib.request.urlopen(req, timeout=60) as r:
                body = json.loads(r.read())

            # Get stats AFTER
            with urllib.request.urlopen(f"http://127.0.0.1:{PROXY_PORT}/stats", timeout=3) as r:
                post_stats = json.loads(r.read())

            post_removed = post_stats.get("summary", {}).get("compression", {}).get("total_tokens_removed", 0)
            compressed = post_stats.get("summary", {}).get("compression", {}).get("requests_compressed", 0)

            llm_response = body.get("choices", [{}])[0].get("message", {}).get("content", "")
            prompt_tokens = body.get("usage", {}).get("prompt_tokens", 0)
            completion_tokens = body.get("usage", {}).get("completion_tokens", 0)

            results.append({
                "case": tc["name"],
                "label": tc["label"],
                "pre_len": len(tc["content"]),
                "post_len": prompt_tokens,  # tokens sent to LLM (proxy compressed)
                "pre_content": tc["content"][:500],
                "post_content": f"[{prompt_tokens} prompt tokens sent to LLM]",
                "mode": "proxy",
                "error": body.get("error", {}).get("message"),
                "headroom_removed": post_removed - pre_removed,
                "requests_compressed": compressed,
                "llm_response": llm_response[:200],
                "completion_tokens": completion_tokens,
            })
        except Exception as e:
            results.append({
                "case": tc["name"], "label": tc["label"],
                "pre_len": len(tc["content"]), "post_len": 0,
                "mode": "proxy", "error": str(e)[:200],
            })

    return results


# ── Display ──────────────────────────────────────────────────────────

def format_row(case, label, pre_len, post_len, pct, mode, extra=""):
    bar = "█" * min(int(pct / 5), 20) if pct > 0 else ""
    return f"│ {case:<18} │ {label:<22} │ {pre_len:>7,d} │ {post_len:>7,d} │ {pct:>5.0f}% {bar:<20} │ {mode:<7} │ {extra}"


def print_table(results, mode_label):
    """Print comparison table for one mode."""
    print(f"\n{'='*120}")
    print(f"  MODE: {mode_label}")
    print(f"{'='*120}")
    print(f"│ {'Case':<18} │ {'Label':<22} │ {'Pre':>7} │ {'Post':>7} │ {'Saved':>5}  {'':20} │ {'Mode':<7} │ Details")
    print(f"│{'─'*18}─┼─{'─'*22}─┼─{'─'*7}─┼─{'─'*7}─┼─{'─'*25}─┼─{'─'*7}─┼─{'─'*40}")

    for r in results:
        pre = r.get("pre_len", 0)
        post = r.get("post_len", 0)
        pct = round((1 - post / pre) * 100, 1) if pre > 0 else 0
        extra = ""
        if r.get("error"):
            extra = f"ERROR: {r['error'][:40]}"
        elif r.get("headroom_removed") is not None:
            extra = f"headroom removed {r['headroom_removed']}t, LLM: {r.get('llm_response', '')[:50]}"
        elif r.get("tokens_saved", 0) > 0:
            saved = r["tokens_saved"]
            extra = f"compressed {saved}t then reverted by guard" if post == pre else f"tokens_saved={saved}"
        print(format_row(r["case"], r.get("label", ""), pre, post, pct, r["mode"], extra))

    if results and all(not r.get("error") for r in results):
        total_pre = sum(r.get("pre_len", 0) for r in results)
        total_post = sum(r.get("post_len", 0) for r in results)
        total_pct = round((1 - total_post / total_pre) * 100, 1) if total_pre > 0 else 0
        print(f"│{'─'*18}─┼─{'─'*22}─┼─{'─'*7}─┼─{'─'*7}─┼─{'─'*25}─┼─{'─'*7}─┤")
        print(f"│ {'TOTAL':<18} │ {'':<22} │ {total_pre:>7,d} │ {total_post:>7,d} │ {total_pct:>5.0f}% {'':20} │         │")
    print(f"└{'─'*18}─┴─{'─'*22}─┴─{'─'*7}─┴─{'─'*7}─┴─{'─'*25}─┴─{'─'*7}─┘")


def print_content_compare(results_inline, results_proxy):
    """Side-by-side content comparison for verbose mode."""
    if not VERBOSE:
        return

    print(f"\n{'='*120}")
    print("  CONTENT COMPARISON (pre vs post)")
    print(f"{'='*120}")

    for tc in TEST_CASES:
        inline = next((r for r in results_inline if r["case"] == tc["name"]), {})
        proxy = next((r for r in results_proxy if r["case"] == tc["name"]), {})

        pre = tc["content"]
        post_inline = inline.get("post_content", "") if not inline.get("error") else f"ERROR: {inline.get('error')}"

        print(f"\n── {tc['name']} ({tc['label']}) ──")
        print(f"  PRE  ({len(pre):,d} chars): {pre[:200]}{'...' if len(pre) > 200 else ''}")
        print(f"  INLINE POST ({len(post_inline):,d} chars): {str(post_inline)[:200]}{'...' if len(str(post_inline)) > 200 else ''}")
        if proxy:
            print(f"  PROXY LLM response: {proxy.get('llm_response', 'N/A')[:200]}")


# ── Main ────────────────────────────────────────────────────────────

def main():
    print("HermesCompress — Content-Level Compression Comparison")
    print(f"Proxy: {'healthy' if proxy_health() else 'NOT RUNNING'} on port {PROXY_PORT}")

    all_results = {}

    if not MODE_FILTER or MODE_FILTER == "inline":
        print("\n[inline] Compressing via Compress class (library mode)...")
        results_inline = test_inline()
        all_results["inline"] = results_inline
        print_table(results_inline, "INLINE (Compress library in-process)")

    if not MODE_FILTER or MODE_FILTER == "cli":
        print("\n[cli] Compressing via hermes-compress CLI...")
        results_cli = test_cli()
        all_results["cli"] = results_cli
        print_table(results_cli, "CLI (hermes-compress command)")

    if not MODE_FILTER or MODE_FILTER == "proxy":
        print("\n[proxy] Testing through headroom proxy...")
        results_proxy = test_proxy()
        all_results["proxy"] = results_proxy
        print_table(results_proxy, "PROXY (headroom proxy → DeepSeek)")

    # Cross-mode content comparison
    if VERBOSE and "inline" in all_results and "proxy" in all_results:
        print_content_compare(all_results["inline"], all_results["proxy"])

    # Save results
    ts = time.strftime("%Y%m%d_%H%M%S")
    out_file = REPO_DIR / ".hermes" / "tests" / f"compare_content_{ts}.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text(json.dumps(all_results, indent=2, default=str))
    print(f"\nResults saved: {out_file}")


if __name__ == "__main__":
    main()
