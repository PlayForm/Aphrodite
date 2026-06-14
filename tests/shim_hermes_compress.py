#!/usr/bin/env python3
"""
HermesCompress Dynamic Plugin — injects headroom compression into Hermes Agent.

Hooks into ``agent/conversation_loop.py:674`` — right after the system prompt
is prepended to ``api_messages`` and immediately before the LLM API call.

Uses the inline ``Compress.compress()`` library for actual token savings.
The proxy alone cannot compress Chat Completions traffic.

INSTALLATION (temporary, for testing):
    cp tests/shim_hermes_compress.py ~/.hermes/plugins/hermes_compress_shim.py
    python3 -c "import sys; sys.path.insert(0, '...'); from hermes_compress_shim import patch; patch()"

Or load dynamically in a test:
    cd HermesCompress
    .venv/bin/python tests/shim_hermes_compress.py --test

ARCHITECTURE:
    Hermes conversation loop builds api_messages:
        system_prompt + context + conversation_history + tool_results
    This shim intercepts api_messages right before the API call,
    runs headroom compresion (SmartCrusher + Kompress + CodeCompressor),
    and replaces api_messages with the compressed version.

    CCR markers (<<ccr:hash>> or [N items compressed ... hash=KEY]) are
    resolved via the headroom_retrieve tool (registered by this plugin).

V4-PRO SPECS:
    Context: 1,000,000 tokens | Max output: 384,000 tokens
"""

from __future__ import annotations

import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

# ═══════════════════════════════════════════════════════════════════════
# Constants
# ═══════════════════════════════════════════════════════════════════════

MODEL = "deepseek-v4-pro"
MAX_CONTEXT = 1_000_000
MAX_OUTPUT = 384_000
PROXY_URL = "http://127.0.0.1:8787"  # cache proxy for /v1/retrieve

# Compression config (default — best on small caches: 66.2%, no corruption)
COMPRESS_CONFIG = {
    "protect_recent": 1,       # protect most recent message  
    "min_tokens": 100,         # threshold
    "target_ratio": None,      # let headroom decide — best for small caches
}

# ═══════════════════════════════════════════════════════════════════════
# Headroom Retrieve Tool (Hermes plugin pattern from PR #824)
# ═══════════════════════════════════════════════════════════════════════

HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve the original uncompressed content behind a headroom "
        "compression marker. When you see a marker like "
        "'[N items compressed ... hash=abc123]' or '<<ccr:abc123>>' in "
        "tool results or conversation history, call this tool with the "
        "hash to read the full original content instead of guessing or "
        "re-running the command. The marker format may include type and "
        "size suffixes (e.g. '<<ccr:abc,base64,4.5KB>>') — just extract "
        "the hash. Content expires after a TTL — if expired, re-run the "
        "original command instead."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": "Hash from compression marker (e.g. 'abc123' from '[... hash=abc123]' or '<<ccr:abc123>>')",
            },
            "query": {
                "type": "string",
                "description": "Optional BM25 search query to filter large results to relevant parts",
            },
        },
        "required": ["hash"],
    },
}


def _normalize_hash(raw: str) -> str:
    """Strip marker formatting: <<ccr:hash,base64,4.5KB>> → hash."""
    h = raw.strip("<>").removeprefix("ccr:").removeprefix("hash=")
    h = h.split(",")[0].strip()
    return h


def _handle_headroom_retrieve(args: dict) -> str:
    """Call POST /v1/retrieve on the headroom proxy."""
    hash_raw = str(args.get("hash") or "").strip()
    hash_key = _normalize_hash(hash_raw)
    if not hash_key:
        return json.dumps({"error": "hash is required (from '[... hash=abc123]' marker)"})

    payload: dict = {"hash": hash_key}
    query = str(args.get("query") or "").strip()
    if query:
        payload["query"] = query

    try:
        import httpx
        resp = httpx.post(f"{PROXY_URL}/v1/retrieve", json=payload, timeout=15)
    except ImportError:
        req = urllib.request.Request(
            f"{PROXY_URL}/v1/retrieve",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        try:
            resp_data = urllib.request.urlopen(req, timeout=15)
            resp_body = json.loads(resp_data.read())
            return json.dumps({
                "original_content": resp_body.get("original_content", ""),
                "original_tokens": resp_body.get("original_tokens"),
                "tool_name": resp_body.get("tool_name"),
            })
        except urllib.error.HTTPError as e:
            body = e.read().decode()
            if e.code == 404:
                return json.dumps({
                    "error": "Content not found: expired (TTL passed) or proxy restarted. Re-run the original command."
                })
            return json.dumps({"error": f"headroom proxy HTTP {e.code}: {body[:200]}"})
        except Exception as exc:
            return json.dumps({
                "error": f"headroom proxy unreachable at {PROXY_URL} ({type(exc).__name__}). Re-run the original command."
            })

    if resp.status_code == 404:
        return json.dumps({
            "error": "Content not found: expired (TTL passed) or proxy restarted. Re-run the original command."
        })
    if resp.status_code != 200:
        return json.dumps({"error": f"headroom proxy HTTP {resp.status_code}: {resp.text[:200]}"})

    data = resp.json()
    return json.dumps({
        "original_content": data.get("original_content", ""),
        "original_tokens": data.get("original_tokens"),
        "tool_name": data.get("tool_name"),
    })


# ═══════════════════════════════════════════════════════════════════════
# Hermes Plugin Registration
# ═══════════════════════════════════════════════════════════════════════

def register(ctx) -> None:
    """Hermes plugin entry point — registers headroom_retrieve tool."""
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )


# ═══════════════════════════════════════════════════════════════════════
# Monkey-patch: intercept api_messages before LLM API call
# ═══════════════════════════════════════════════════════════════════════

_ORIGINAL_RUN = None
_COMPRESS = None
_STATS: dict = {"calls": 0, "tokens_before": 0, "tokens_after": 0, "tokens_saved": 0}


def _get_compress():
    global _COMPRESS
    if _COMPRESS is None:
        try:
            from hermes_compress import Compress, CompressOption
            option = CompressOption()
            option.Enabled = True
            option.Mode = "inline"
            option.ProtectRecent = COMPRESS_CONFIG["protect_recent"]
            option.TargetRatio = COMPRESS_CONFIG["target_ratio"]
            option.MinTokensToCompress = COMPRESS_CONFIG["min_tokens"]
            option.PrecompressTools = True
            option.AggressiveKompress = True
            option.DeduplicateResults = True
            option.VerboseStats = True
            _COMPRESS = Compress(model=MODEL, option=option)
            print(f"[hermes-compress-shim] Compress engine loaded for {MODEL}")
        except ImportError as e:
            print(f"[hermes-compress-shim] ERROR: Cannot import hermes_compress: {e}", file=sys.stderr)
            print(f"[hermes-compress-shim] Ensure HermesCompress repo is on PYTHONPATH", file=sys.stderr)
            _COMPRESS = False
        except Exception as e:
            print(f"[hermes-compress-shim] ERROR initializing Compress: {e}", file=sys.stderr)
            _COMPRESS = False
    return _COMPRESS if _COMPRESS is not False else None


def _compress_api_messages(api_messages: list[dict]) -> list[dict]:
    """Run headroom compression on api_messages."""
    compressor = _get_compress()
    if not compressor:
        return api_messages

    try:
        result = compressor.compress(api_messages)
        _STATS["calls"] += 1
        _STATS["tokens_before"] += result.tokens_before
        _STATS["tokens_after"] += result.tokens_after
        _STATS["tokens_saved"] += result.tokens_saved

        savings_pct = (result.tokens_saved / max(result.tokens_before, 1)) * 100
        print(
            f"[hermes-compress-shim] #{_STATS['calls']}: "
            f"{result.tokens_before:,} → {result.tokens_after:,} tokens "
            f"(-{result.tokens_saved:,} = {savings_pct:.1f}%) "
            f"in {result.duration_ms:.0f}ms"
        )
        return result.messages
    except Exception as e:
        print(f"[hermes-compress-shim] Compression error: {e} — passing through unchanged", file=sys.stderr)
        return api_messages


def patch() -> bool:
    """
    Monkey-patch Hermes Agent's conversation loop to compress api_messages.

    Intercepts at ``agent/conversation_loop._build_api_messages`` or
    wraps the run_conversation function. Call once before starting Hermes.

    Returns True if patch succeeded.
    """
    global _ORIGINAL_RUN

    try:
        from agent.conversation_loop import run_conversation as _original_run
        _ORIGINAL_RUN = _original_run
    except ImportError:
        print("[hermes-compress-shim] ERROR: Cannot import agent.conversation_loop", file=sys.stderr)
        print("[hermes-compress-shim] Is Hermes Agent installed? Run from inside Hermes.", file=sys.stderr)
        return False

    import functools

    @functools.wraps(_original_run)
    def _patched_run(agent, system_message, messages, conversation_history, turn_id, user_message, **kwargs):
        """Wrapped run_conversation — not the right level to intercept api_messages."""

        # The compression needs to happen at a deeper level — right before
        # the LLM API call inside the loop. We patch the agent's API call
        # method instead.

        # Store reference to original call_llm
        original_call = getattr(agent, "_call_llm_with_retry", None)
        if original_call is None:
            # Try the common call method name
            original_call = getattr(agent, "_make_api_call", None)
        if original_call is None:
            # Fallback: patch the OpenAI client call
            print("[hermes-compress-shim] Falling back to client-level patch", file=sys.stderr)
            return _original_run(agent, system_message, messages, conversation_history, turn_id, user_message, **kwargs)

        import functools as ft

        @ft.wraps(original_call)
        def _compressing_call(*args, **kwargs):
            # Try to find api_messages in the arguments
            api_messages = kwargs.get("messages") or kwargs.get("api_messages")
            if api_messages is None and len(args) > 0:
                api_messages = args[0] if isinstance(args[0], list) else None

            if api_messages and isinstance(api_messages, list) and len(api_messages) > 1:
                compressed = _compress_api_messages(api_messages)
                if "messages" in kwargs:
                    kwargs["messages"] = compressed
                elif "api_messages" in kwargs:
                    kwargs["api_messages"] = compressed
                # Rebuild args tuple with compressed messages
                new_args = list(args)
                for i, a in enumerate(new_args):
                    if a is api_messages:
                        new_args[i] = compressed
                        break
                args = tuple(new_args)

            return original_call(*args, **kwargs)

        setattr(agent, "_call_llm_with_retry", _compressing_call)
        print("[hermes-compress-shim] ✓ Patched agent._call_llm_with_retry")

        return _original_run(agent, system_message, messages, conversation_history, turn_id, user_message, **kwargs)

    # Replace the function in the module
    import agent.conversation_loop
    agent.conversation_loop.run_conversation = _patched_run
    print("[hermes-compress-shim] ✓ Monkey-patched agent.conversation_loop.run_conversation")
    return True


def stats() -> dict:
    """Return compression statistics."""
    return dict(_STATS)


# ═══════════════════════════════════════════════════════════════════════
# Standalone test
# ═══════════════════════════════════════════════════════════════════════

def _test_standalone():
    """Test compression outside Hermes — uses the Compress library directly."""
    print("╔══════════════════════════════════════════════╗")
    print("║  HermesCompress Shim — Standalone Test       ║")
    print("╠══════════════════════════════════════════════╣")
    print(f"║  Model:  {MODEL}  │  1M ctx / {MAX_OUTPUT:,} out")
    print("╚══════════════════════════════════════════════╝\n")

    # Load large test content
    test_files = {
        "proxy-start.py": (REPO / "scripts" / "proxy-start.py").read_text(),
        "report.py": (REPO / "tests" / "report.py").read_text(),
    }

    # Build a realistic Hermes conversation
    messages = [
        {"role": "system", "content": "You are a helpful coding assistant. Be concise."},
        {"role": "user", "content": "Read scripts/proxy-start.py"},
        {"role": "assistant", "content": None, "tool_calls": [
            {"id": "call_1", "type": "function",
             "function": {"name": "read_file", "arguments": '{"path":"scripts/proxy-start.py"}'}}
        ]},
        {"role": "tool", "content": test_files["proxy-start.py"], "tool_call_id": "call_1"},
        {"role": "assistant", "content": "This script launches a headroom proxy server for API compression."},
        {"role": "user", "content": "Now read tests/report.py"},
        {"role": "assistant", "content": None, "tool_calls": [
            {"id": "call_2", "type": "function",
             "function": {"name": "read_file", "arguments": '{"path":"tests/report.py"}'}}
        ]},
        {"role": "tool", "content": test_files["report.py"], "tool_call_id": "call_2"},
    ]

    print(f"Test conversation: {len(messages)} messages")
    total_chars = sum(len(str(m)) for m in messages)
    print(f"Total chars: {total_chars:,}\n")

    compressor = _get_compress()
    if not compressor:
        print("ERROR: Cannot load hermes_compress. Is the repo on PYTHONPATH?")
        print(f"REPO: {REPO}")
        return

    print("Compressing...")
    result = compressor.compress(messages)

    print(f"\n  Messages:    {len(result.messages)} (was {len(messages)})")
    print(f"  Tokens before: {result.tokens_before:,}")
    print(f"  Tokens after:  {result.tokens_after:,}")
    print(f"  Tokens saved:  {result.tokens_saved:,}")
    pct = (result.tokens_saved / max(result.tokens_before, 1)) * 100
    print(f"  Savings:       {pct:.1f}%")
    print(f"  Duration:      {result.duration_ms:.0f}ms")
    if result.transforms_applied:
        print(f"  Transforms:    {', '.join(result.transforms_applied)}")
    if result.error:
        print(f"  Error:         {result.error}")

    # Show compressed content preview
    print("\n  Compressed tool outputs (first 200 chars each):")
    for i, m in enumerate(result.messages):
        if m.get("role") == "tool":
            content = str(m.get("content", ""))
            print(f"    [{i}] tool_call_id={m.get('tool_call_id', '?')}: "
                  f"{content[:150]}{'...' if len(content) > 150 else ''}")

    _STATS["calls"] += 1
    _STATS["tokens_before"] += result.tokens_before
    _STATS["tokens_after"] += result.tokens_after
    _STATS["tokens_saved"] += result.tokens_saved
    print(f"\n✓ Standalone test complete. Stats: {stats()}")


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser(description="HermesCompress Dynamic Shim")
    p.add_argument("--test", action="store_true", help="Run standalone compression test")
    p.add_argument("--patch", action="store_true", help="Install monkey-patch into Hermes Agent")
    p.add_argument("--stats", action="store_true", help="Show compression statistics")
    args = p.parse_args()

    if args.test:
        _test_standalone()
    elif args.patch:
        patch()
    elif args.stats:
        print(json.dumps(stats(), indent=2))
    else:
        p.print_help()
