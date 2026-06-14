#!/usr/bin/env python3
"""
HermesCompress Dynamic Shim — injects headroom compression into Hermes Agent.

Hooks into ``agent/conversation_loop.py:674`` — right after the system prompt
is prepended to ``api_messages`` and immediately before the LLM API call.

ONLY TWO JOBS:
  1. Compress api_messages via headroom inline library
  2. Register headroom_retrieve tool for CCR marker resolution

No measurement. No filtering. No response handling.
Leave everything else to Hermes and DeepSeek.

INSTALL:
    .venv/bin/python tests/shim_hermes_compress.py --patch

TEST (standalone, no Hermes needed):
    .venv/bin/python tests/shim_hermes_compress.py --test
"""

from __future__ import annotations

import json
import sys
import urllib.request
import urllib.error
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

MODEL = "deepseek-v4-pro"
PROXY_URL = "http://127.0.0.1:8787"  # for /v1/retrieve only

COMPRESS_CONFIG = {
    "protect_recent": 1,
    "min_tokens": 100,
    "target_ratio": None,  # let headroom decide
    "precompress": True,
    "aggressive_kompress": True,
    "deduplicate": True,
}

# ═══════════════════════════════════════════════════════════════════════
# CCR: headroom_retrieve tool
# ═══════════════════════════════════════════════════════════════════════

HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original uncompressed content behind a headroom compression "
        "marker. Markers look like '[N items compressed ... hash=abc123]' or "
        "'<<ccr:abc123>>' or '<<ccr:abc,base64,4.5KB>>'. Extract just the hash "
        "and call this tool. Content expires after TTL — if expired, re-run "
        "the original command instead."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": "Hash from the compression marker (e.g. 'abc123' from '[... hash=abc123]')",
            },
            "query": {
                "type": "string",
                "description": "Optional BM25 search query to filter large results",
            },
        },
        "required": ["hash"],
    },
}


def _normalize_hash(raw: str) -> str:
    h = raw.strip("<>").removeprefix("ccr:").removeprefix("hash=")
    return h.split(",")[0].strip()


def _handle_headroom_retrieve(args: dict) -> str:
    hash_key = _normalize_hash(str(args.get("hash") or "").strip())
    if not hash_key:
        return json.dumps({"error": "hash required"})

    payload: dict = {"hash": hash_key}
    query = str(args.get("query") or "").strip()
    if query:
        payload["query"] = query

    try:
        import httpx
        resp = httpx.post(f"{PROXY_URL}/v1/retrieve", json=payload, timeout=15)
        if resp.status_code == 404:
            return json.dumps({"error": "Content expired or not found. Re-run original command."})
        if resp.status_code != 200:
            return json.dumps({"error": f"Proxy HTTP {resp.status_code}"})
        data = resp.json()
        return json.dumps({
            "original_content": data.get("original_content", ""),
            "original_tokens": data.get("original_tokens"),
            "tool_name": data.get("tool_name"),
        })
    except ImportError:
        req = urllib.request.Request(
            f"{PROXY_URL}/v1/retrieve",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        try:
            resp_data = urllib.request.urlopen(req, timeout=15)
            data = json.loads(resp_data.read())
            return json.dumps({
                "original_content": data.get("original_content", ""),
                "original_tokens": data.get("original_tokens"),
                "tool_name": data.get("tool_name"),
            })
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return json.dumps({"error": "Content expired. Re-run original command."})
            return json.dumps({"error": f"Proxy HTTP {e.code}"})
        except Exception as exc:
            return json.dumps({"error": f"Proxy unreachable ({type(exc).__name__}). Re-run original command."})


# ═══════════════════════════════════════════════════════════════════════
# Hermes plugin registration
# ═══════════════════════════════════════════════════════════════════════

def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )


# ═══════════════════════════════════════════════════════════════════════
# Compression engine (lazy init)
# ═══════════════════════════════════════════════════════════════════════

_ENGINE = None


def _engine():
    global _ENGINE
    if _ENGINE is None:
        try:
            from hermes_compress import Compress, CompressOption
            opt = CompressOption()
            opt.Enabled = True
            opt.Mode = "inline"
            opt.ProtectRecent = COMPRESS_CONFIG["protect_recent"]
            opt.MinTokensToCompress = COMPRESS_CONFIG["min_tokens"]
            opt.TargetRatio = COMPRESS_CONFIG["target_ratio"]
            opt.PrecompressTools = COMPRESS_CONFIG["precompress"]
            opt.AggressiveKompress = COMPRESS_CONFIG["aggressive_kompress"]
            opt.DeduplicateResults = COMPRESS_CONFIG["deduplicate"]
            _ENGINE = Compress(model=MODEL, option=opt)
        except Exception:
            _ENGINE = False
    return _ENGINE if _ENGINE is not False else None


def _compress(messages: list[dict]) -> list[dict]:
    c = _engine()
    if not c:
        return messages
    try:
        return c.compress(messages).messages
    except Exception:
        return messages


# ═══════════════════════════════════════════════════════════════════════
# Monkey-patch
# ═══════════════════════════════════════════════════════════════════════

def patch() -> bool:
    """Monkey-patch AIAgent._interruptible_api_call forwarders to compress messages."""
    try:
        from agent.conversation_loop import run_conversation as _orig
    except ImportError:
        print("[hermes-compress-shim] ERROR: not running inside Hermes Agent", file=sys.stderr)
        return False

    import functools

    @functools.wraps(_orig)
    def _patched(agent, system_message, messages, conversation_history, turn_id, user_message, **kw):
        _api = getattr(agent, "_interruptible_api_call", None)
        _stream = getattr(agent, "_interruptible_streaming_api_call", None)

        if not _api and not _stream:
            print("[hermes-compress-shim] WARNING: no intercept hook found", file=sys.stderr)
            return _orig(agent, system_message, messages, conversation_history, turn_id, user_message, **kw)

        def _make_wrapper(fn):
            @functools.wraps(fn)
            def _compress_hook(*a, **kw):
                if a and isinstance(a[0], dict):
                    api_kwargs = a[0]
                    msgs = api_kwargs.get("messages")
                    if msgs and isinstance(msgs, list) and len(msgs) > 1:
                        api_kwargs = {**api_kwargs, "messages": _compress(msgs)}
                        return fn(*(api_kwargs,) + a[1:], **kw)
                return fn(*a, **kw)
            return _compress_hook

        if _api:
            setattr(agent, "_interruptible_api_call", _make_wrapper(_api))
        if _stream:
            setattr(agent, "_interruptible_streaming_api_call", _make_wrapper(_stream))

        print("[hermes-compress-shim] ✓ patched agent API hooks", file=sys.stderr)
        return _orig(agent, system_message, messages, conversation_history, turn_id, user_message, **kw)

    import agent.conversation_loop
    agent.conversation_loop.run_conversation = _patched
    print("[hermes-compress-shim] ✓ patched conversation loop")
    return True


# ═══════════════════════════════════════════════════════════════════════
# Standalone test
# ═══════════════════════════════════════════════════════════════════════

def _test():
    files = {
        "proxy-start.py": (REPO / "scripts" / "proxy-start.py").read_text(),
        "report.py": (REPO / "tests" / "report.py").read_text(),
    }

    LARGE = ("HEADROOM COMPRESSION TEST. " * 40)
    msgs = [{"role": "system", "content": "Be concise."}]
    for i in range(5):
        msgs.append({"role": "user", "content": f"Turn {i+1}: data."})
        msgs.append({"role": "assistant", "content": None, "tool_calls": [
            {"id": f"c{i}", "type": "function",
             "function": {"name": "read_file", "arguments": f'{{"path":"d{i}.txt"}}'}}
        ]})
        msgs.append({"role": "tool", "content": LARGE, "tool_call_id": f"c{i}"})
        msgs.append({"role": "assistant", "content": f"Turn {i+1} ok."})
        fname = "proxy-start.py" if i % 2 == 0 else "report.py"
        msgs.append({"role": "user", "content": f"Read {fname}."})
        msgs.append({"role": "assistant", "content": None, "tool_calls": [
            {"id": f"cc{i}", "type": "function",
             "function": {"name": "read_file", "arguments": f'{{"path":"{fname}"}}'}}
        ]})
        msgs.append({"role": "tool", "content": files[fname], "tool_call_id": f"cc{i}"})

    print(f"Test: {len(msgs)} messages")
    result = _compress(msgs)
    # Only report what changed — no measurement, just structural check
    tool_out = sum(1 for m in result if m.get("role") == "tool")
    print(f"  messages: {len(result)} (was {len(msgs)})")
    print(f"  tool outputs: {tool_out}")
    has_ccr = any("<<ccr:" in str(m.get("content", "")) or "[compressed" in str(m.get("content", ""))
                  for m in result if m.get("role") == "tool")
    print(f"  CCR markers: {'yes' if has_ccr else 'no'}")
    print("  ✓ ready")


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser(description="HermesCompress Dynamic Shim")
    p.add_argument("--test", action="store_true", help="Standalone compression test")
    p.add_argument("--patch", action="store_true", help="Install monkey-patch")
    a = p.parse_args()
    if a.test:
        _test()
    elif a.patch:
        patch()
    else:
        p.print_help()
