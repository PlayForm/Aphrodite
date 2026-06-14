"""
@playform/hermes-compress - Headroom-powered context compression.

Slash LLM token usage by 25-60% per API call. Two modes, one API:

  Inline - headroom library in-process (50-80ms warm, default)
  Proxy  - headroom as separate server (zero code changes)

Standalone usage:
    from hermes_compress import Compress, Proxy

    # Inline
    c = Compress(model="deepseek-v4-pro", enabled=True)
    result = c.compress(messages)

    # Proxy
    proxy = Proxy(port=8787)
    proxy.start()

CLI:
    hermes-compress proxy --port 8787
    hermes-compress compress "some text"
    hermes-compress stats

Hermes plugin:
    Installed as ~/.hermes/plugins/hermes-compress/
    Auto-loaded by Hermes - register(ctx) handles tool registration.
"""

from __future__ import annotations

__version__ = "0.7.2"
__all__ = [
    "Compress",
    "CompressResult",
    "CompressOption",
    "Proxy",
    "register",
    "check_for_updates",
    "install_update",
]

from hermes_compress._compress import Compress, CompressResult, Proxy
from hermes_compress._option import CompressOption
from hermes_compress._config import _get_integration_mode, get_headroom_config


def register(ctx):
    """Hermes plugin entry point - registers tools and hooks."""

    ctx.register_tool(
        name="headroom_stats",
        toolset="compression",
        schema={
            "type": "function",
            "function": {
                "name": "headroom_stats",
                "description": "Show headroom compression statistics for the current session",
                "parameters": {"type": "object", "properties": {}},
            },
        },
        handler=_headroom_stats_handler,
        description="Show compression savings and pipeline stats",
    )

    ctx.register_tool(
        name="headroom_compress",
        toolset="compression",
        schema={
            "type": "function",
            "function": {
                "name": "headroom_compress",
                "description": "Manually compress a block of text or JSON with headroom",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to compress (JSON, code, text, etc.)",
                        },
                    },
                    "required": ["content"],
                },
            },
        },
        handler=_headroom_compress_handler,
        description="Compress any text/JSON/code with headroom",
    )

    ctx.register_hook("pre_llm_call", _pre_llm_call_hook)
    ctx.register_hook("transform_tool_result", _transform_tool_result_hook)


# ── Tool handlers ───────────────────────────────────────────────────────


def _headroom_stats_handler(args=None, **kwargs) -> str:
    import json
    try:
        from hermes_compress._compress import _active_compressor
        comp = _active_compressor
        if comp is None:
            return json.dumps({"error": "Compressor not initialized yet"})
        return json.dumps(comp.stats, indent=2)
    except Exception as e:
        return json.dumps({"error": str(e)})


def _headroom_compress_handler(args=None, **kwargs) -> str:
    args = args if isinstance(args, dict) else {}
    content = args.get("content", "")
    import json
    try:
        from headroom import compress as _hr_compress
        result = _hr_compress(
            [{"role": "user", "content": content}],
            model="gpt-4o",
        )
        return json.dumps({
            "tokens_before": result.tokens_before,
            "tokens_after": result.tokens_after,
            "tokens_saved": result.tokens_saved,
            "compression_ratio": round(result.compression_ratio * 100, 1),
            "compressed": result.messages[0]["content"][:2000] if result.messages else "",
        }, indent=2)
    except Exception as e:
        return json.dumps({"error": str(e)})


# ── Hooks ────────────────────────────────────────────────────────────────


def _pre_llm_call_hook(messages=None, conversation_history=None, model=None, **kwargs):
    """Pre-LLM call hook. In dev mode, injects compression stats context.
    In production, returns None - compression is silent."""
    import logging
    logger = logging.getLogger(__name__)

    try:
        from hermes_compress._dev import is_dev
        if not is_dev():
            return None

        cfg = get_headroom_config()
        if not cfg["enabled"] or cfg["integration"] == "proxy":
            return None

        from hermes_compress._compress import _active_compressor
        comp = _active_compressor
        if comp is None:
            return None

        stats = comp.stats
        saved = stats.get("total_tokens_saved", 0)
        calls = stats.get("calls", 0)
        duration = stats.get("total_duration_ms", 0)

        return {"context": (
            f"[hermes-compress v{__version__} active | "
            f"{calls} calls | "
            f"{saved:,} tokens saved | "
            f"{duration:.0f}ms total]"
        )}
    except Exception:
        pass
    return None


def _transform_tool_result_hook(
    tool_name="",
    args=None,
    result="",
    tool_call_id="",
    task_id="",
    session_id="",
    turn_id="",
    api_request_id="",
    duration_ms=0,
    status="",
    error_type="",
    error_message="",
    **kwargs,
):
    """Transform tool result - compress tool output at capture time.

    Dev mode: forces read_file/terminal/execute_code/patch to minimal
    compression so the developer can always read tool output.

    Production: passes through to headroom with configured settings.
    Only guard: revert if headroom produces empty output."""
    import logging
    logger = logging.getLogger(__name__)

    # Fast path: nothing to compress
    if not result or not isinstance(result, str) or not result.strip():
        return result

    try:
        cfg = get_headroom_config()
        if not cfg["enabled"]:
            return result

        from hermes_compress._strategies import get_strategy
        from hermes_compress._dev import is_dev

        strategy = get_strategy(tool_name or "", dev_mode=is_dev())
        tier = strategy.get("tier", "balanced")
        if tier == "skip":
            return result

        # Dev mode: dev-critical tools always return original.
        if is_dev() and tool_name in ("read_file", "terminal", "execute_code", "patch", "read_terminal"):
            return result

        # Minimum content threshold - per-tool strategy, not global
        min_tokens = strategy.get("min_tokens_to_compress", 250)
        if len(result) < min_tokens:
            return result

        # Get or create compressor (hot-reloads on config change)
        from hermes_compress._compress import Compress, CompressOption, _active_compressor

        compressor = _active_compressor
        _need_new = (
            compressor is None
            or not getattr(compressor, "_option", None)
            or compressor._option.TargetRatio != cfg.get("target_ratio")
            or compressor._option.PrecompressTools != cfg.get("precompress_tools", False)
            or compressor._option.AggressiveKompress != cfg.get("aggressive_kompress", False)
            or compressor._option.DeduplicateResults != cfg.get("deduplicate_results", False)
        )
        if _need_new:
            option = CompressOption(
                Enabled=True,
                Mode=cfg["mode"],
                ProtectRecent=strategy.get("protect_recent", cfg["protect_recent"]),
                TargetRatio=strategy.get("target_ratio") or cfg.get("target_ratio"),
                MinTokensToCompress=strategy.get("min_tokens_to_compress", 250),
                PrecompressTools=cfg.get("precompress_tools", False),
                AggressiveKompress=cfg.get("aggressive_kompress", False),
                DeduplicateResults=cfg.get("deduplicate_results", False),
                VerboseStats=cfg.get("verbose_stats", False),
            )
            compressor = Compress(option=option, model=None)
            import hermes_compress._compress as hc_mod
            hc_mod._active_compressor = compressor

        # Wrap in minimal message list
        tool_msg = {
            "role": "tool",
            "content": result,
            "tool_call_id": tool_call_id or "unknown",
            "name": tool_name or "unknown",
        }
        messages = [
            {"role": "system", "content": "Tool result compression pass-through."},
            tool_msg,
        ]

        compress_result = compressor.compress(messages)

        # Extract compressed content
        for m in compress_result.messages:
            if m.get("role") == "tool" and m.get("tool_call_id") == tool_msg["tool_call_id"]:
                compressed = m.get("content", "")
                # Guard: revert if empty or too small to be useful
                if compressed and compressed.strip() and len(compressed) >= 200:
                    return compressed
                return result

        return result

    except Exception:
        return result
