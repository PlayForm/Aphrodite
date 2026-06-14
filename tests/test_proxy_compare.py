#!/usr/bin/env python3
"""
Dual-proxy side-by-side comparison - tests cache vs token mode.

Sends identical messages to both proxies simultaneously and compares
token counts, compression, latency, and stats.

Usage:
    cd HermesCompress
    .venv/bin/python tests/test_proxy_compare.py
"""

from __future__ import annotations

import json
import os
import time
import urllib.request
import urllib.error
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

PROXIES = {
    "cache": {"url": "http://127.0.0.1:8787/v1/chat/completions",
              "stats": "http://127.0.0.1:8787/stats",
              "retrieve": "http://127.0.0.1:8787/v1/retrieve",
              "desc": "freeze prefixes"},
    "token": {"url": "http://127.0.0.1:8788/v1/chat/completions",
              "stats": "http://127.0.0.1:8788/stats",
              "retrieve": "http://127.0.0.1:8788/v1/retrieve",
              "desc": "compress/rewrite"},
}

MODEL = "deepseek-v4-pro"
MAX_OUTPUT = 384_000

# Per-turn results: [{label, turn, cache: {tokens,latency,...}, token: {...}}]
RESULTS: list[dict] = []


def _load_key() -> str:
    env_file = REPO / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("HEADROOM_DEEPSEEK_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.getenv("HEADROOM_DEEPSEEK_KEY", "")


def _call(proxy: str, messages: list[dict], label: str = "") -> dict | None:
    """Send to one proxy, return {content, usage, elapsed}."""
    url = PROXIES[proxy]["url"]
    payload = {"model": MODEL, "messages": messages, "max_tokens": MAX_OUTPUT}
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {_key}",
        },
    )
    start = time.time()
    try:
        resp = urllib.request.urlopen(req, timeout=120)
        data = json.loads(resp.read())
        usage = data.get("usage", {})
        content = data["choices"][0]["message"]["content"]
        elapsed = time.time() - start
        reasoning = usage.get("completion_tokens_details", {}).get("reasoning_tokens", 0)
        return {
            "content": content,
            "usage": usage,
            "elapsed": elapsed,
            "prompt_tokens": usage.get("prompt_tokens", 0),
            "completion_tokens": usage.get("completion_tokens", 0),
            "reasoning_tokens": reasoning,
            "visible_tokens": usage.get("completion_tokens", 0) - reasoning,
        } # type: ignore
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"    [{proxy}] ERROR {e.code}: {body[:200]}")
        return None
    except Exception as e:
        print(f"    [{proxy}] ERROR: {e}")
        return None


def _dual_call(messages: list[dict], label: str = "", turn: int = 0) -> dict:
    """Send identical messages to both proxies, return comparison dict."""
    result = {"label": label, "turn": turn}
    for proxy in ["cache", "token"]:
        r = _call(proxy, messages, f"{label}/{proxy}")
        if r:
            result[proxy] = r
            print(f"    [{proxy:5s}] prompt={r['prompt_tokens']:>6d}  completion={r['completion_tokens']:>5d}  latency={r['elapsed']:.1f}s")
        else:
            result[proxy] = None
            print(f"    [{proxy:5s}] FAILED")
    RESULTS.append(result)
    return result


def _get_stats(proxy: str) -> dict:
    try:
        with urllib.request.urlopen(PROXIES[proxy]["stats"], timeout=5) as r:
            return json.loads(r.read())
    except Exception:
        return {}


def _print_compare(label: str = ""):
    """Print side-by-side stats from both proxies."""
    sc = _get_stats("cache")
    st = _get_stats("token")
    sc_s = sc.get("summary", {})
    st_s = st.get("summary", {})
    sc_c = sc_s.get("compression", {})
    st_c = st_s.get("compression", {})
    sc_u = sc_s.get("uncompressed_requests", {})
    st_u = st_s.get("uncompressed_requests", {})

    print(f"\n  {'─' * 50} {label}")
    print(f"  {'':20s} {'CACHE (:8787)':>20s}  {'TOKEN (:8788)':>20s}")
    print(f"  {'requests:':20s} {sc_s.get('api_requests','?'):>20}  {st_s.get('api_requests','?'):>20}")
    print(f"  {'compressed:':20s} {sc_c.get('requests_compressed','?'):>20}  {st_c.get('requests_compressed','?'):>20}")
    print(f"  {'avg_pct:':20s} {str(sc_c.get('avg_compression_pct','?')) + '%':>20s}  {str(st_c.get('avg_compression_pct','?')) + '%':>20s}")
    print(f"  {'tokens_removed:':20s} {sc_c.get('total_tokens_removed','?'):>20}  {st_c.get('total_tokens_removed','?'):>20}")
    print(f"  {'prefix_frozen:':20s} {sc_u.get('prefix_frozen','?'):>20}  {st_u.get('prefix_frozen','?'):>20}")
    print(f"  {'total_before:':20s} {sc_c.get('total_tokens_before_with_cli_filtering','?'):>20}  {st_c.get('total_tokens_before_with_cli_filtering','?'):>20}")


def _final_report():
    """Print final comparison summary from accumulated RESULTS."""
    cache_ok = sum(1 for r in RESULTS if r.get("cache"))
    token_ok = sum(1 for r in RESULTS if r.get("token"))
    cache_prompt = sum(r["cache"]["prompt_tokens"] for r in RESULTS if r.get("cache"))
    token_prompt = sum(r["token"]["prompt_tokens"] for r in RESULTS if r.get("token"))
    cache_comp = sum(r["cache"]["completion_tokens"] for r in RESULTS if r.get("cache"))
    token_comp = sum(r["token"]["completion_tokens"] for r in RESULTS if r.get("token"))
    cache_lat = sum(r["cache"]["elapsed"] for r in RESULTS if r.get("cache"))
    token_lat = sum(r["token"]["elapsed"] for r in RESULTS if r.get("token"))

    print("\n" + "=" * 70)
    print(f"  FINAL COMPARISON - {MODEL}  |  1M ctx / {MAX_OUTPUT:,} out")
    print("=" * 70)
    print(f"  {'':25s} {'CACHE (prefix-freeze)':>20s}  {'TOKEN (compress)':>20s}")
    print(f"  {'─' * 25} {'─' * 20}  {'─' * 20}")
    print(f"  {'Successful calls:':25s} {cache_ok:>20}  {token_ok:>20}")
    print(f"  {'Total prompt tokens:':25s} {cache_prompt:>20,}  {token_prompt:>20,}")
    print(f"  {'Total completion tokens:':25s} {cache_comp:>20,}  {token_comp:>20,}")
    print(f"  {'Total latency:':25s} {cache_lat:>19.1f}s  {token_lat:>19.1f}s")
    print(f"  {'Avg prompt/turn:':25s} {cache_prompt // max(cache_ok, 1):>20,}  {token_prompt // max(token_ok, 1):>20,}")

    # Per-turn breakdown
    print(f"\n  {'Turn':>4s}  {'Payload':20s}  {'Cache prompt':>12s}  {'Token prompt':>12s}  {'Delta':>8s}")
    print(f"  {'─' * 4}  {'─' * 20}  {'─' * 12}  {'─' * 12}  {'─' * 8}")
    for r in RESULTS:
        cp = r["cache"]["prompt_tokens"] if r.get("cache") else 0
        tp = r["token"]["prompt_tokens"] if r.get("token") else 0
        delta = tp - cp
        sign = "+" if delta > 0 else ""
        print(f"  {r['turn']:>4d}  {r['label']:20s}  {cp:>12,}  {tp:>12,}  {sign}{delta:>7,}")

    # Final proxy stats
    _print_compare("PROXY STATS")
    print("=" * 70)
    print(f"\n  Cache mode:  freezes prior turns → prefix-cache hit rate")
    print(f"  Token mode:  rewrites prior turns → compression savings")
    print(f"  Note: Chat Completions API may not compress in either mode.")


# ═══════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════

_key = _load_key()
if not _key:
    print("ERROR: HEADROOM_DEEPSEEK_KEY not set in .env")
    exit(1)

print("╔══════════════════════════════════════════════════╗")
print("║  Dual Proxy Side-by-Side Comparison              ║")
print("╠══════════════════════════════════════════════════╣")
print(f"║  Model:  {MODEL}  -  1M ctx / {MAX_OUTPUT:,} out")
print(f"║  Cache:  :8787  -  {PROXIES['cache']['desc']}")
print(f"║  Token:  :8788  -  {PROXIES['token']['desc']}")
print("╚══════════════════════════════════════════════════╝\n")

# Load test files
test_files = {
    "proxy-start.py": (REPO / "scripts" / "proxy-start.py").read_text(),
    "report.py": (REPO / "tests" / "report.py").read_text(),
}
print(f"Loaded {len(test_files)} test files ({sum(len(v) for v in test_files.values())} total chars)\n")

# ── Test 1: Simple file content ──
print("─── Test 1: Embedded file content ───")
_dual_call(
    [
        {"role": "system", "content": "You are a coding assistant. Be concise."},
        {"role": "user", "content": f"What does this script do? One sentence.\n```python\n{test_files['proxy-start.py']}\n```"},
    ],
    label="file-read",
    turn=1,
)
_print_compare("after T1")

# ── Test 2: Multi-turn accumulation ──
print("\n─── Test 2: Multi-turn (file + prior response) ───")
# We need the responses from T1 to build T2 messages
t1_cache_resp = RESULTS[-1].get("cache", {}).get("content", "") if RESULTS else ""
t1_token_resp = RESULTS[-1].get("token", {}).get("content", "") if RESULTS else ""

t2_messages_template = lambda resp: [
    {"role": "system", "content": "You are a coding assistant. Be concise."},
    {"role": "user", "content": f"What does this script do? One sentence.\n```python\n{test_files['proxy-start.py']}\n```"},
    {"role": "assistant", "content": resp},
    {"role": "user", "content": f"Now what about this one? One sentence.\n```python\n{test_files['report.py']}\n```"},
]

# Send with cache's own prior response
result_t2 = {"label": "multi-turn", "turn": 2}
for proxy in ["cache", "token"]:
    prior_resp = t1_cache_resp if proxy == "cache" else t1_token_resp
    r = _call(proxy, t2_messages_template(prior_resp), f"T2/{proxy}")
    if r:
        result_t2[proxy] = r
        print(f"    [{proxy:5s}] prompt={r['prompt_tokens']:>6d}  completion={r['completion_tokens']:>5d}  latency={r['elapsed']:.1f}s")
    else:
        result_t2[proxy] = None
        print(f"    [{proxy:5s}] FAILED")
RESULTS.append(result_t2)
_print_compare("after T2")

# ── Test 3: 3-turn large context stress ──
print("\n─── Test 3: 3-turn large context stress ───")
LARGE_TEXT = (
    "HEADROOM PROXY INTEGRATION TEST DATA BLOCK. "
    "This repetitive text simulates tool output from file reads. "
    "The fox compression algorithm should detect patterns here. "
) * 40

for turn_i in range(3):
    # Build fresh conversation for this turn (not accumulated - test each independently)
    # Actually, let's accumulate for cache vs token comparison
    pass

# Accumulated 3-turn conversation
conv_cache = [{"role": "system", "content": "You are a helpful assistant. Be concise. One sentence answer."}]
conv_token = [{"role": "system", "content": "You are a helpful assistant. Be concise. One sentence answer."}]

for i in range(3):
    user_msg = {"role": "user", "content": f"Turn {i + 1}: How many times does 'fox' appear in this?\n\n{LARGE_TEXT}"}

    conv_cache.append(user_msg)
    conv_token.append(user_msg)

    result_t3 = {"label": f"stress-t{i+1}", "turn": 3 + i}

    for proxy, conv in [("cache", conv_cache), ("token", conv_token)]:
        r = _call(proxy, conv, f"T3.{i+1}/{proxy}")
        if r:
            result_t3[proxy] = r
            conv.append({"role": "assistant", "content": r["content"]})
            print(f"    [{proxy:5s}] T{i+1}: prompt={r['prompt_tokens']:>6d}  completion={r['completion_tokens']:>5d}  latency={r['elapsed']:.1f}s")
        else:
            result_t3[proxy] = None
            print(f"    [{proxy:5s}] T{i+1}: FAILED")
    RESULTS.append(result_t3)

_print_compare("after T3")

# ── Test 4: CCR retrieval (both proxies) ──
print("\n─── Test 4: CCR /v1/retrieve ───")
for proxy in ["cache", "token"]:
    url = PROXIES[proxy]["retrieve"]
    try:
        req = urllib.request.Request(url, data=json.dumps({"hash": "test123"}).encode(),
                                     headers={"Content-Type": "application/json"})
        resp = urllib.request.urlopen(req, timeout=5)
        print(f"    [{proxy:5s}] retrieve: {resp.status}")
    except urllib.error.HTTPError as e:
        print(f"    [{proxy:5s}] retrieve: {e.code} (no compressed entries)")

# ── Final report ──
_final_report()
print("\n✓ Dual proxy comparison complete.")
