#!/usr/bin/env python3
"""
Token-mode proxy integration test for Hermes.

Tests the full CCR compression → retrieval pipeline through headroom proxy
in token mode, simulating Hermes' Chat Completions traffic.

Usage:
    cd HermesCompress
    .venv/bin/python tests/test_token_proxy.py
"""

from __future__ import annotations

import json
import os
import time
import urllib.request
import urllib.error
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TOKEN_URL = "http://127.0.0.1:8788/v1/chat/completions"
RETRIEVE_URL = "http://127.0.0.1:8788/v1/retrieve"
STATS_URL = "http://127.0.0.1:8788/stats"

# DeepSeek v4-pro: 1.6T params, 49B active, 1M context, 384K max output
MODEL = "deepseek-v4-pro"
MAX_CONTEXT = 1_000_000
MAX_OUTPUT = 384_000
MAX_TOKENS = MAX_OUTPUT


def _load_key() -> str:
    env_file = REPO / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("HEADROOM_DEEPSEEK_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.getenv("HEADROOM_DEEPSEEK_KEY", "")


def _call(messages: list[dict], max_tokens: int = MAX_TOKENS, label: str = "") -> dict | None:
    """Send Chat Completions request to token proxy."""
    payload = {"model": MODEL, "messages": messages, "max_tokens": max_tokens}
    req = urllib.request.Request(
        TOKEN_URL,
        data=json.dumps(payload).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {_key}",
        },
    )
    start = time.time()
    try:
        resp = urllib.request.urlopen(req, timeout=60)
        data = json.loads(resp.read())
        usage = data.get("usage", {})
        content = data["choices"][0]["message"]["content"]
        elapsed = time.time() - start
        reasoning = usage.get("completion_tokens_details", {}).get("reasoning_tokens", 0)
        print(f"  [{label}] OK ({elapsed:.1f}s)")
        print(f"    prompt_tokens: {usage.get('prompt_tokens')},  completion: {usage.get('completion_tokens')} (reasoning={reasoning}, visible={usage.get('completion_tokens', 0) - reasoning})")
        print(f"    response:      {content[:200]}{'...' if len(content) > 200 else ''}")
        return {"content": content, "usage": usage}
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"  [{label}] ERROR {e.code}: {body[:300]}")
        return None
    except Exception as e:
        print(f"  [{label}] ERROR: {e}")
        return None


def _retrieve(hash_key: str, query: str = "") -> dict | None:
    """Call POST /v1/retrieve to fetch original content behind a CCR marker."""
    payload = {"hash": hash_key}
    if query:
        payload["query"] = query
    req = urllib.request.Request(
        RETRIEVE_URL,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
    )
    try:
        resp = urllib.request.urlopen(req, timeout=10)
        return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        print(f"    retrieve({hash_key}): {e.code} — {body[:200]}")
        return None
    except Exception as e:
        print(f"    retrieve({hash_key}): ERROR — {e}")
        return None


def _stats() -> dict:
    """Get proxy stats."""
    req = urllib.request.Request(STATS_URL)
    try:
        return json.loads(urllib.request.urlopen(req, timeout=5).read())
    except Exception:
        return {}


def _print_stats(label: str = ""):
    s = _stats()
    summary = s.get("summary", {})
    comp = summary.get("compression", {})
    unc = summary.get("uncompressed_requests", {})
    print(f"\n  ── STATS {label} ──")
    print(f"  mode:             {summary.get('mode')}")
    print(f"  requests:         {summary.get('api_requests')}")
    print(f"  compressed:       {comp.get('requests_compressed')}")
    print(f"  avg_pct:          {comp.get('avg_compression_pct')}%")
    print(f"  tokens_removed:   {comp.get('total_tokens_removed')}")
    print(f"  prefix_frozen:    {unc.get('prefix_frozen', 'N/A')}")
    print(f"  total_before:     {comp.get('total_tokens_before_with_cli_filtering', 'N/A')}")


# ═══════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════

_key = _load_key()
if not _key:
    print("ERROR: HEADROOM_DEEPSEEK_KEY not set in .env")
    exit(1)

print("╔══════════════════════════════════════════╗")
print("║  Token Proxy Integration Test            ║")
print("╠══════════════════════════════════════════╣")
print(f"║  Proxy:  {TOKEN_URL}")
print(f"║  Model:  {MODEL}")
print(f"║  Ctx:    1M  |  MaxOut:  384K")
print("╚══════════════════════════════════════════╝\n")

# Load test files
test_files = {
    "proxy-start.py": (REPO / "scripts" / "proxy-start.py").read_text(),
    "report.py": (REPO / "tests" / "report.py").read_text(),
}

print(f"Loaded {len(test_files)} test files ({sum(len(v) for v in test_files.values())} total chars)\n")

# ── Test 1: Simple file read (no tool calls) ──
print("─── Test 1: Embedded file content ───")
_res1 = _call(
    [
        {"role": "system", "content": "You are a coding assistant. Be concise."},
        {
            "role": "user",
            "content": f"What does this script do? One sentence.\n```python\n{test_files['proxy-start.py']}\n```",
        },
    ],
    label="T1-file",
)
_print_stats("after T1")

# ── Test 2: Multi-turn with accumulated context ──
if _res1:
    print("\n─── Test 2: Multi-turn accumulation ───")
    _res2 = _call(
        [
            {"role": "system", "content": "You are a coding assistant. Be concise."},
            {
                "role": "user",
                "content": f"What does this script do? One sentence.\n```python\n{test_files['proxy-start.py']}\n```",
            },
            {"role": "assistant", "content": _res1["content"]},
            {
                "role": "user",
                "content": f"Now what about this one? One sentence.\n```python\n{test_files['report.py']}\n```",
            },
        ],
        label="T2-accumulated",
    )
    _print_stats("after T2")

    # Check for CCR markers in response
    if _res2:
        for fmt in ["[N items", "<<ccr:", "[compressed"]:
            idx = _res2["content"].find(fmt)
            if idx >= 0:
                snippet = _res2["content"][max(0, idx - 10):idx + 100]
                print(f"\n  >>> CCR MARKER DETECTED: ...{snippet}...")
                break
        else:
            print("\n  No CCR markers (Chat Completions passes through uncompressed)")

# ── Test 3: Large conversation to stress test ──
print("\n─── Test 3: 3-turn large context stress ───")
LARGE_TEXT = (
    "HEADROOM PROXY INTEGRATION TEST DATA BLOCK. "
    "This repetitive text simulates tool output from file reads. "
    "The fox compression algorithm should detect patterns here. "
) * 40

conv = [
    {"role": "system", "content": "You are a helpful assistant. Be very concise. Answer in one sentence."},
]

for i in range(3):
    conv.append(
        {
            "role": "user",
            "content": f"Turn {i + 1}: Data block:\n\n{LARGE_TEXT}\n\nHow many times does 'fox' appear?",
        }
    )
    resp = _call(conv, label=f"T3-turn{i+1}")
    if resp:
        conv.append({"role": "assistant", "content": resp["content"]})
    else:
        print(f"  Turn {i+1} failed, stopping")
        break

_print_stats("after T3")

# ── Test 4: CCR retrieval ──
print("\n─── Test 4: CCR /v1/retrieve endpoint ───")
test_hashes = ["test123", "abc456", "deadbeef"]
for h in test_hashes:
    result = _retrieve(h)

# Also test with a hash from a CCR marker format
print("  Testing marker format hash...")
result = _retrieve("abc123,base64,4.5KB")
result = _retrieve("hash=abc123")

# ── Final summary ──
print("\n" + "=" * 50)
final = _stats()
summary = final.get("summary", {})
comp = summary.get("compression", {})
unc = summary.get("uncompressed_requests", {})
print(f"FINAL SUMMARY")
print(f"  Mode:            {summary.get('mode')}")
print(f"  Total requests:  {summary.get('api_requests')}")
print(f"  Compressed:      {comp.get('requests_compressed')}/{summary.get('api_requests', 1)}")
print(f"  Avg savings:     {comp.get('avg_compression_pct')}%")
print(f"  Tokens removed:  {comp.get('total_tokens_removed')}")
print(f"  Prefix frozen:   {unc.get('prefix_frozen', 'N/A')}")
print(f"  Best saving:     {comp.get('best_compression_pct')}%")
print("=" * 50)
print("\n✓ Token proxy test complete.")
