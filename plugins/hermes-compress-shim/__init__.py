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
    """Monkey-patch conversation loop to compress api_messages."""
    try:
        from agent.conversation_loop import run_conversation as _orig
    except ImportError:
        return

    import functools

    @functools.wraps(_orig)
    def _patched(agent, system_message, messages, conversation_history, turn_id, user_message, **kw):
        call = getattr(agent, "_call_llm_with_retry", None) or getattr(agent, "_make_api_call", None)
        if call is None:
            return _orig(agent, system_message, messages, conversation_history, turn_id, user_message, **kw)

        @functools.wraps(call)
        def _with_compress(*a, **kw):
            msgs = kw.get("messages") or kw.get("api_messages")
            if msgs is None and len(a) > 0 and isinstance(a[0], list):
                msgs = a[0]
            if msgs and isinstance(msgs, list) and len(msgs) > 1:
                compressed = _compress(msgs)
                if "messages" in kw:
                    kw["messages"] = compressed
                elif "api_messages" in kw:
                    kw["api_messages"] = compressed
                na = list(a)
                for i, v in enumerate(na):
                    if v is msgs:
                        na[i] = compressed
                        break
                a = tuple(na)
            return call(*a, **kw)

        setattr(agent, call.__name__, _with_compress)
        return _orig(agent, system_message, messages, conversation_history, turn_id, user_message, **kw)

    import agent.conversation_loop
    agent.conversation_loop.run_conversation = _patched
