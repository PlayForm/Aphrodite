"""
HermesCompress Shim Plugin — injects headroom compression into the conversation loop.

Only two jobs: compress api_messages + provide headroom_retrieve tool.
No measurement. No filtering. No response handling.
"""

from __future__ import annotations

import json
import sys
import urllib.request
import urllib.error

MODEL = "deepseek-v4-pro"
PROXY_URL = "http://127.0.0.1:8787"  # for /v1/retrieve (if proxy is running)

COMPRESS_CONFIG = {
    "protect_recent": 1,
    "min_tokens": 100,
    "target_ratio": None,
    "precompress": True,
    "aggressive_kompress": True,
    "deduplicate": True,
}

# ═══════════════════════════════════════════════════════════
# CCR: headroom_retrieve tool
# ═══════════════════════════════════════════════════════════

HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind a headroom compression marker. "
        "Markers look like '[N items compressed ... hash=abc123]' or "
        "'<<ccr:abc,base64,4.5KB>>'. Extract just the hash. "
        "Content expires — if not found, re-run the original command."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "Hash from marker (e.g. 'abc123')"},
            "query": {"type": "string", "description": "Optional BM25 search query"},
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
            return json.dumps({"error": "Content expired. Re-run original command."})
        if resp.status_code != 200:
            return json.dumps({"error": f"Proxy HTTP {resp.status_code}"})
        data = resp.json()
        return json.dumps({"original_content": data.get("original_content", ""),
                           "original_tokens": data.get("original_tokens")})
    except ImportError:
        req = urllib.request.Request(f"{PROXY_URL}/v1/retrieve",
            data=json.dumps(payload).encode(), headers={"Content-Type": "application/json"})
        try:
            data = json.loads(urllib.request.urlopen(req, timeout=15).read())
            return json.dumps({"original_content": data.get("original_content", ""),
                               "original_tokens": data.get("original_tokens")})
        except urllib.error.HTTPError as e:
            return json.dumps({"error": f"Content expired (HTTP {e.code}). Re-run command."})
        except Exception as exc:
            return json.dumps({"error": f"Proxy unreachable. Re-run command."})


# ═══════════════════════════════════════════════════════════
# Compression engine
# ═══════════════════════════════════════════════════════════

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


# ═══════════════════════════════════════════════════════════
# Hermes plugin: register() + auto-patch
# ═══════════════════════════════════════════════════════════

def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
    _patch_loop()


def _patch_loop():
    """Monkey-patch AIAgent._interruptible_api_call forwarders to compress messages.

    The agent's API call forwarders (run_agent.py:4012,4183) receive
    ``api_kwargs`` as a dict with ``messages`` at ``api_kwargs["messages"]``.
    We wrap them to compress those messages before they reach the provider.
    """
    try:
        from agent.conversation_loop import run_conversation as _orig
    except ImportError:
        return

    import functools, sys

    @functools.wraps(_orig)
    def _patched(agent, system_message, messages, conversation_history, turn_id, user_message, **kw):
        _api = getattr(agent, "_interruptible_api_call", None)
        _stream = getattr(agent, "_interruptible_streaming_api_call", None)

        if not _api and not _stream:
            print("[hermes-compress-shim] WARNING: no intercept hook found", file=sys.stderr)
            return _orig(agent, system_message, messages, conversation_history, turn_id, user_message, **kw)

        # ── Compression wrapper ───────────────────────────────────────
        def _make_wrapper(fn):
            @functools.wraps(fn)
            def _compress_hook(*a, **kw):
                # Forwarder sig: _interruptible_*_api_call(self, api_kwargs, ...)
                # First positional arg after self is always api_kwargs dict.
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
