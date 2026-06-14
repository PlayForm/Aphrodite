#!/usr/bin/env python3
"""Benchmark: direct inline compression vs proxy (token mode).

Direct:    shim compresses messages before DeepSeek API call
Proxy:     routes through headroom proxy :8788 (prefix-cache only, no Chat Completions compression)

Compares token counts, latency, and cache behaviour.
"""
import json
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV_PYTHON = str(REPO / ".venv" / "bin" / "python")

# --- Token estimator ---
def est_tokens(text):
    if text is None: return 0
    if isinstance(text, list): return sum(est_tokens(str(x)) for x in text)
    if isinstance(text, dict): return sum(est_tokens(str(v)) for v in text.values())
    return len(str(text)) // 4

# --- Build 85-msg payload (same as live-benchmark v2) ---
def build_payload():
    readme = (REPO / "README.md").read_text()
    listing = "\n".join([
        "total 248",
        "drwxr-xr-x  12 user  staff    384 Jun 14 10:00 .",
        "drwxr-xr-x   8 user  staff    256 Jun 14 10:00 ..",
        "-rw-r--r--   1 user  staff   1234 Jun 14 10:01 __init__.py",
        "-rw-r--r--   1 user  staff   5678 Jun 14 10:02 _compress.py",
        "-rw-r--r--   1 user  staff   3456 Jun 14 10:03 _strategies.py",
        "-rw-r--r--   1 user  staff   9012 Jun 14 10:04 _optimize.py",
        "-rw-r--r--   1 user  staff   2345 Jun 14 10:05 _dev.py",
        "-rw-r--r--   1 user  staff  12345 Jun 14 10:07 README.md",
        "-rw-r--r--   1 user  staff   7890 Jun 14 10:08 pyproject.toml",
    ])
    code_sample = "def compress_messages(msgs, opt):\n    if not opt.Enabled:\n        return msgs\n    msgs = _pre_process(msgs)\n    msgs = _optimize(msgs, opt)\n    msgs = _apply_strategies(msgs)\n    msgs = _truncate(msgs, opt.MaxTokens)\n    msgs = _dedup(msgs)\n    result = headroom.compress(msgs)\n    return result.messages\n"
    web_result = "DeepSeek v4-pro: 1.6T MoE, 49B active/token, 1M context, 384K output. MLA KV-cache 93.3% reduction.\n"
    json_data = json.dumps({"benchmark_results": [{"config": f"cfg_{i}", "tokens_before": 25000+i*5000, "tokens_after": 11000+i*3000, "savings_pct": round(45.0+i*3.5,1)} for i in range(15)]}, indent=2)
    log_data = "\n".join(["hermes_compress/_compress.py:234: result = headroom.compress(messages)" for _ in range(12)])

    tools_by_type = [
        ("terminal",      "ls -la", listing, "SmartCrusher"),
        ("code",          "read_file _compress.py", code_sample, "CodeCompressor"),
        ("web_search",    "deepseek v4-pro specs", web_result, "Kompress"),
        ("json",          "benchmark_results.json", json_data, "SmartCrusher+Compaction"),
        ("log",           "grep compress hermes_compress/", log_data, "ContentRouter"),
    ]

    msgs = [{"role": "system", "content": "You are a helpful coding assistant. Be concise."}]
    for turn in range(12):
        msgs.append({"role": "user", "content": f"Turn {turn+1}: run analysis"})
        for offset in range(2):
            idx = (turn * 2 + offset) % 5
            name, cmd, content, stage = tools_by_type[idx]
            tool_call_id = f"t{turn}_{name}"
            msgs.append({"role": "assistant", "content": None, "tool_calls": [
                {"id": tool_call_id, "type": "function", "function": {"name": name, "arguments": json.dumps({"cmd": cmd})}}
            ]})
            msgs.append({"role": "tool", "content": content, "tool_call_id": tool_call_id})
        if turn % 2 == 0:
            msgs.append({"role": "assistant", "content": None, "tool_calls": [
                {"id": f"dedup{turn}", "type": "function", "function": {"name": "read_file", "arguments": json.dumps({"path": "README.md"})}}
            ]})
            msgs.append({"role": "tool", "content": readme, "tool_call_id": f"dedup{turn}"})
        msgs.append({"role": "assistant", "content": f"Turn {turn+1} analysis complete."})
    return msgs

# --- Run direct compression (shim path) ---
def run_direct(messages):
    script = """
import json, sys, time
sys.path.insert(0, %r)
from hermes_compress import Compress, CompressOption
opt = CompressOption()
opt.Enabled = True
opt.Mode = 'inline'
opt.ProtectRecent = 1
opt.MinTokensToCompress = 100
c = Compress(model='deepseek-v4-pro', option=opt)
payload = json.loads(sys.stdin.read())
t0 = time.time()
result = c.compress(payload)
elapsed = time.time() - t0
orig_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in payload)
comp_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in result.messages)
role_counts = {}
for m in result.messages:
    r = m.get('role', 'unknown')
    role_counts[r] = role_counts.get(r, 0) + 1
ccr = sum(1 for m in result.messages if '<<ccr:' in str(m.get('content', '')))
print(json.dumps({
    'mode': 'direct',
    'messages_before': len(payload),
    'messages_after': len(result.messages),
    'chars_before': orig_chars,
    'chars_after': comp_chars,
    'savings_pct': round((1 - comp_chars/orig_chars)*100, 1) if orig_chars > 0 else 0,
    'latency_ms': round(elapsed*1000),
    'roles': role_counts,
    'ccr_markers': ccr,
}))
""" % str(REPO)
    proc = subprocess.run(
        [VENV_PYTHON, "-c", script],
        input=json.dumps(messages).encode(),
        capture_output=True, timeout=120
    )
    if proc.returncode != 0:
        return {"mode": "direct", "error": proc.stderr.decode()[:500]}
    return json.loads(proc.stdout)

# --- Run via proxy (token mode) ---
def run_proxy(messages):
    """Send Chat Completions request through headroom token proxy."""
    import urllib.request, urllib.error
    payload = {
        "model": "deepseek-v4-pro",
        "messages": messages,
        "max_tokens": 100,
        "temperature": 0,
    }
    t0 = time.time()
    try:
        req = urllib.request.Request(
            "http://127.0.0.1:8788/v1/chat/completions",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        resp = json.loads(urllib.request.urlopen(req, timeout=30).read())
        elapsed = time.time() - t0
        usage = resp.get("usage", {})
        return {
            "mode": "proxy (token :8788)",
            "messages_sent": len(messages),
            "prompt_tokens": usage.get("prompt_tokens", 0),
            "completion_tokens": usage.get("completion_tokens", 0),
            "cache_read_tokens": usage.get("prompt_tokens_details", {}).get("cache_read_tokens", 0),
            "latency_ms": round(elapsed * 1000),
            "status": "ok",
        }
    except Exception as exc:
        return {
            "mode": "proxy (token :8788)",
            "error": str(exc),
            "latency_ms": round((time.time() - t0) * 1000),
        }

# --- Main ---
print("Building 85-message payload...")
msgs = build_payload()
orig_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in msgs)
orig_tokens = est_tokens(json.dumps(msgs))
print(f"  {len(msgs)} messages, {orig_chars:,} chars (~{orig_tokens:,} est tokens)")
print()

# Warm up headroom (first call is slow — Kompress model download)
print("Warm-up (Kompress model load)...")
warm = subprocess.run(
    [VENV_PYTHON, "-c", f"""
import json, sys
sys.path.insert(0, {str(REPO)!r})
from hermes_compress import Compress, CompressOption
opt = CompressOption(); opt.Enabled = True; opt.Mode = 'inline'
c = Compress(model='deepseek-v4-pro', option=opt)
c.compress([{{'role': 'system', 'content': 'warm'}}, {{'role': 'user', 'content': 'up'}}])
print('warm-up OK')
"""],
    capture_output=True, timeout=120
)
print(f"  {warm.stdout.decode().strip()}")
print()

# Run direct
print("=== DIRECT (inline shim) ===")
direct = run_direct(msgs)
for k, v in direct.items():
    if k != "roles":
        print(f"  {k}: {v}")
print(f"  roles: {direct.get('roles', {})}")
print()

# Run proxy
print("=== PROXY (token mode :8788) ===")
proxy = run_proxy(msgs)
for k, v in proxy.items():
    print(f"  {k}: {v}")
print()

# Summary
print("=== COMPARISON ===")
if "savings_pct" in direct:
    print(f"  Direct compression savings:  {direct['savings_pct']}%")
    print(f"  Direct chars:  {direct['chars_before']:,} → {direct['chars_after']:,}")
    print(f"  Direct latency: {direct['latency_ms']}ms")
if "prompt_tokens" in proxy:
    cache_hit = proxy.get("cache_read_tokens", 0)
    cache_pct = (cache_hit / proxy["prompt_tokens"] * 100) if proxy["prompt_tokens"] > 0 else 0
    print(f"  Proxy prompt tokens:  {proxy['prompt_tokens']:,}")
    print(f"  Proxy cache hit:      {cache_hit:,} tokens ({cache_pct:.1f}%)")
    print(f"  Proxy latency:        {proxy['latency_ms']}ms")
    # Note: proxy doesn't compress Chat Completions — just provides cache freezing
    print()
    print("  NOTE: Proxy token mode does NOT compress Chat Completions.")
    print("  It provides DeepSeek prefix-cache freezing for cost savings.")
    print("  Only the inline shim (direct) reduces token count.")
