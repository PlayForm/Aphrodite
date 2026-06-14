"""
HermesCompress Shim Plugin — injects headroom compression into the conversation loop.

Only two jobs: compress api_messages + provide headroom_retrieve tool.
No measurement. No filtering. No response handling.

When a local headroom proxy is active (base_url → 127.0.0.1 / localhost),
local compression is SKIPPED — the proxy handles it.  The headroom_retrieve
tool always works, hitting the token-mode proxy on port 8788.

Debug: set HERMES_COMPRESS_DEBUG=1 to enable verbose diagnostics.
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.request
import urllib.error

DEBUG = os.environ.get("HERMES_COMPRESS_DEBUG", "") == "1"

def _dbg(msg: str) -> None:
    if DEBUG:
        print(f"[hermes-compress-shim] {msg}", file=sys.stderr)

MODEL = "deepseek-v4-pro"
PROXY_RETRIEVE_URL = "http://127.0.0.1:8788"   # token-mode proxy for /v1/retrieve
PROXY_CHAT_URL = "http://127.0.0.1:8788/v1"     # token-mode proxy for chat completions

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

    def _try_httpx() -> str:
        import httpx
        try:
            resp = httpx.post(f"{PROXY_RETRIEVE_URL}/v1/retrieve", json=payload, timeout=5)
        except httpx.ConnectError:
            return json.dumps({
                "error": "Headroom proxy not running on port 8788. "
                         "Content can only be retrieved when the token-mode proxy is active. "
                         "Re-run the original command instead."
            })
        except httpx.TimeoutException:
            return json.dumps({"error": "Proxy timed out. Re-run original command."})
        if resp.status_code == 404:
            return json.dumps({"error": "Content expired (CCR TTL elapsed). Re-run original command."})
        if resp.status_code != 200:
            return json.dumps({"error": f"Proxy HTTP {resp.status_code}"})
        data = resp.json()
        return json.dumps({"original_content": data.get("original_content", ""),
                           "original_tokens": data.get("original_tokens")})

    def _try_urllib() -> str:
        try:
            req = urllib.request.Request(f"{PROXY_RETRIEVE_URL}/v1/retrieve",
                data=json.dumps(payload).encode(), headers={"Content-Type": "application/json"})
            data = json.loads(urllib.request.urlopen(req, timeout=5).read())
            return json.dumps({"original_content": data.get("original_content", ""),
                               "original_tokens": data.get("original_tokens")})
        except urllib.error.HTTPError as e:
            return json.dumps({"error": f"Content expired (HTTP {e.code}). Re-run original command."})
        except (urllib.error.URLError, ConnectionRefusedError, OSError):
            return json.dumps({
                "error": "Headroom proxy not running (port 8788). "
                         "Re-run the original command instead."
            })
        except Exception:
            return json.dumps({"error": "Proxy unreachable. Re-run original command."})

    try:
        return _try_httpx()
    except ImportError:
        return _try_urllib()


# ═══════════════════════════════════════════════════════════
# Compression engine (local, used only when proxy is absent)
# ═══════════════════════════════════════════════════════════

_ENGINE = None
_ENGINE_ERROR = None


def _engine():
    global _ENGINE, _ENGINE_ERROR
    if _ENGINE is None:
        _dbg("_engine: initialising Compress()...")
        try:
            from hermes_compress import Compress, CompressOption
            _dbg(f"_engine: imports OK (Compress={Compress}, CompressOption={CompressOption})")
            opt = CompressOption()
            opt.Enabled = True
            opt.Mode = "inline"
            opt.ProtectRecent = COMPRESS_CONFIG["protect_recent"]
            opt.MinTokensToCompress = COMPRESS_CONFIG["min_tokens"]
            opt.TargetRatio = COMPRESS_CONFIG["target_ratio"]
            opt.PrecompressTools = COMPRESS_CONFIG["precompress"]
            opt.AggressiveKompress = COMPRESS_CONFIG["aggressive_kompress"]
            opt.DeduplicateResults = COMPRESS_CONFIG["deduplicate"]
            _dbg(f"_engine: CompressOption built: enabled={opt.Enabled} mode={opt.Mode} "
                 f"protect_recent={opt.ProtectRecent} min_tokens={opt.MinTokensToCompress} "
                 f"target_ratio={opt.TargetRatio}")
            _ENGINE = Compress(model=MODEL, option=opt)
            _dbg(f"_engine: Compress instance created OK (type={type(_ENGINE).__name__})")
        except Exception as exc:
            _ENGINE = False
            _ENGINE_ERROR = f"{type(exc).__name__}: {exc}"
            print(f"[hermes-compress-shim] ERROR: engine init failed: {_ENGINE_ERROR}", file=sys.stderr)
            if DEBUG:
                import traceback
                traceback.print_exc(file=sys.stderr)
    return _ENGINE if _ENGINE is not False else None


def _compress(messages: list[dict]) -> list[dict]:
    _dbg(f"_compress: called with {len(messages)} messages")
    c = _engine()
    if not c:
        _dbg(f"_compress: engine NOT available (error: {_ENGINE_ERROR}) — returning unchanged")
        return messages
    try:
        # Estimate tokens before
        before_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in messages)
        _dbg(f"_compress: compressing {len(messages)} msgs, ~{before_chars} chars...")
        t0 = time.time()
        result = c.compress(messages)
        elapsed = time.time() - t0
        after_chars = sum(len(json.dumps(m, ensure_ascii=False)) for m in result.messages)
        savings = (1 - after_chars / before_chars) * 100 if before_chars > 0 else 0
        _dbg(f"_compress: done in {elapsed*1000:.0f}ms — {before_chars}→{after_chars} chars ({savings:.1f}%)")
        return result.messages
    except Exception as exc:
        _dbg(f"_compress: EXCEPTION {type(exc).__name__}: {exc}")
        if DEBUG:
            import traceback
            traceback.print_exc(file=sys.stderr)
        return messages


# ═══════════════════════════════════════════════════════════
# Proxy detection
# ═══════════════════════════════════════════════════════════

def _is_proxy_active(agent) -> bool:
    """Return True if the agent's base_url points to a local headroom proxy."""
    base = ""
    model_cfg = getattr(agent, "model_config", None)
    if isinstance(model_cfg, dict):
        base = str(model_cfg.get("base_url", ""))
    if not base:
        base = str(getattr(agent, "base_url", ""))
    if not base:
        cfg = getattr(agent, "config", None)
        if cfg:
            base = str(getattr(cfg, "base_url", ""))
    result = "127.0.0.1" in base or "localhost" in base
    _dbg(f"_is_proxy_active: base_url='{base}' → {result}")
    return result


# ═══════════════════════════════════════════════════════════
# Hermes plugin: register() + auto-patch
# ═══════════════════════════════════════════════════════════

def register(ctx) -> None:
    _dbg("register() called — registering headroom_retrieve tool")
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
    _dbg("register() → calling _patch_loop()")
    _patch_loop()


def _patch_loop():
    """Monkey-patch AIAgent._interruptible_api_call forwarders to compress messages."""
    _dbg("_patch_loop: importing agent.conversation_loop.run_conversation...")
    try:
        from agent.conversation_loop import run_conversation as _orig
        _dbg(f"_patch_loop: import OK — _orig={_orig}")
    except ImportError as exc:
        _dbg(f"_patch_loop: ImportError — {exc}")
        return

    import functools

    @functools.wraps(_orig)
    def _patched(*args, **kwargs):
        agent = args[0]
        _dbg(f"_patched: called — agent type={type(agent).__name__} args={len(args)} kwargs_keys={list(kwargs.keys())}")

        _api = getattr(agent, "_interruptible_api_call", None)
        _stream = getattr(agent, "_interruptible_streaming_api_call", None)
        _dbg(f"_patched: _api={'FOUND' if _api else 'MISSING'} _stream={'FOUND' if _stream else 'MISSING'}")

        if not _api and not _stream:
            print("[hermes-compress-shim] WARNING: no intercept hook found", file=sys.stderr)
            return _orig(*args, **kwargs)

        using_proxy = _is_proxy_active(agent)
        _dbg(f"_patched: using_proxy={using_proxy}")

        # ── Compression wrapper ───────────────────────────────────────
        def _make_wrapper(fn):
            fn_name = getattr(fn, "__name__", str(fn))
            @functools.wraps(fn)
            def _compress_hook(*a, **kw):
                _dbg(f"_compress_hook[{fn_name}]: called — positional_args={len(a)} "
                     f"kw_keys={list(kw.keys())} "
                     f"a[0]_type={type(a[0]).__name__ if a else 'N/A'} "
                     f"using_proxy={using_proxy}")
                if not using_proxy and a and isinstance(a[0], dict):
                    api_kwargs = a[0]
                    msgs = api_kwargs.get("messages")
                    _dbg(f"_compress_hook[{fn_name}]: msgs={'PRESENT' if msgs else 'MISSING'} "
                         f"type={type(msgs).__name__ if msgs else 'N/A'} "
                         f"len={len(msgs) if isinstance(msgs, list) else 'N/A'}")
                    if msgs and isinstance(msgs, list) and len(msgs) > 1:
                        _dbg(f"_compress_hook[{fn_name}]: compressing {len(msgs)} msgs...")
                        api_kwargs = {**api_kwargs, "messages": _compress(msgs)}
                        return fn(*(api_kwargs,) + a[1:], **kw)
                    else:
                        _dbg(f"_compress_hook[{fn_name}]: SKIP — msgs too small or not list")
                else:
                    _dbg(f"_compress_hook[{fn_name}]: SKIP — "
                         f"using_proxy={using_proxy} has_args={bool(a)} "
                         f"a0_is_dict={isinstance(a[0], dict) if a else False}")
                return fn(*a, **kw)
            return _compress_hook

        if _api:
            _dbg("_patched: wrapping _interruptible_api_call")
            setattr(agent, "_interruptible_api_call", _make_wrapper(_api))
        if _stream:
            _dbg("_patched: wrapping _interruptible_streaming_api_call")
            setattr(agent, "_interruptible_streaming_api_call", _make_wrapper(_stream))

        tag = "proxy-active (no local compression)" if using_proxy else "direct compression"
        msg = f"[hermes-compress-shim] ✓ patched agent API hooks — {tag}"
        print(msg, file=sys.stderr)
        _dbg(f"_patched: {msg}")
        return _orig(*args, **kwargs)

    import agent.conversation_loop
    agent.conversation_loop.run_conversation = _patched
    _dbg("_patch_loop: run_conversation monkey-patched successfully")
