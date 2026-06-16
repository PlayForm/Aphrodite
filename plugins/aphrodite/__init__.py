"""
aphrodite v1.61.0 — CCR compression plugin for Hermes Agent.

Auto-install + launch aphrodite proxies:
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold

Modules:
  _core      — constants, thresholds, CCR regex, inline store
  _inline    — zlib fallback compression
  _marker    — CCR marker formatting, proxy compression, marker parsing
  _binary    — binary download + platform detection
  _proxy     — proxy lifecycle (env, health, launch)
  _tools     — 9 tool handlers + schemas, file tracking, conversation memory
  _hooks     — Hook handlers (transform_tool_result, terminal, pre_llm, post_llm)
  _engine    — ContextEngine for Hermes compression pipeline

On session_start: downloads binary, launches token proxy on :9798.
"""
import os
import logging
import json
import time
import hashlib

# ── Core imports (re-export everything) ──────────────────────────
from ._core import (
    PORTS, REPO, BIN_VERSION, PLUGIN_VERSION, BINARY_DIR, BINARY, ENV_FILE, _log,
    _cfg_int, ENGINE_THRESHOLD_PCT, ENGINE_PROTECT_FIRST, ENGINE_PROTECT_LAST,
    ENGINE_MIN_MSGS, TOOL_THRESHOLD_TOKEN, TOOL_THRESHOLD_CACHE,
    TERMINAL_THRESHOLD, INLINE_THRESHOLD, RECURSIVE_DEPTH,
    DEBUG_LOGGING, CATALOG_MODE, _DEV, _CCR_RE, _inline_store,
)

from ._inline import _inline_compress, _inline_retrieve
from ._marker import _ccr_marker, _compress_via_proxy, _parse_ccr_markers
from ._binary import _detect_platform, _download_binary, _ensure_binary
from ._proxy import _load_env, _alive_cache, _alive, _start, on_start, _wait_alive
from ._resolve import _resolve_one, _resolve_recursive

# ── Tools + state ─────────────────────────────────────────────────
from ._tools import (
    _retrieve_handler, _compress_handler,
    COMPRESS_SCHEMA, RETRIEVE_SCHEMA,
)

# ── Hooks (contains some tools + all hook handlers) ────────────────
from ._hooks import (
    _transform_tool_result, _store_conversation_turn, _git_summary,
    _pre_llm_hook, _transform_terminal_hook, _parse_ccr_markers,
    _rebuild_handler, _stats_handler, _files_handler, _diff_handler,
    _catalog_handler, _search_handler, _test_handler,
    _track_file_refs, _fmt_size, _inline_clear,
    _extract_preview, _group_into_turns,
    REBUILD_SCHEMA, STATS_SCHEMA,
    FILES_SCHEMA, DIFF_SCHEMA, CATALOG_SCHEMA, SEARCH_SCHEMA, TEST_SCHEMA,
    _referenced_files, _recent_markers, _conv_index, _turn_counter,
    _git_cache, _FILE_TOOLS,
)

# ── Engine ────────────────────────────────────────────────────────
from ._engine import (
    AphroditeContextEngine, _engine, _set_engine, get_engine, _fire_hook,
)

# ── Plugin registration ───────────────────────────────────────────
def register(ctx):
    """Register hooks, tools, and context engine with Hermes."""
    _ensure_binary()
    ctx.register_hook("on_session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
    ctx.register_hook("post_llm_call", _store_conversation_turn)
    ctx.register_hook("transform_tool_result", _transform_tool_result)

    ctx.register_tool(name="aphrodite_rebuild", schema=REBUILD_SCHEMA, handler=_rebuild_handler)
    ctx.register_tool(name="aphrodite_compress", schema=COMPRESS_SCHEMA, handler=_compress_handler)
    ctx.register_tool(name="aphrodite_retrieve", schema=RETRIEVE_SCHEMA, handler=_retrieve_handler)
    ctx.register_tool(name="aphrodite_stats", schema=STATS_SCHEMA, handler=_stats_handler)
    ctx.register_tool(name="aphrodite_files", schema=FILES_SCHEMA, handler=_files_handler)
    ctx.register_tool(name="aphrodite_diff", schema=DIFF_SCHEMA, handler=_diff_handler)
    ctx.register_tool(name="aphrodite_search", schema=SEARCH_SCHEMA, handler=_search_handler)
    ctx.register_tool(name="aphrodite_test", schema=TEST_SCHEMA, handler=_test_handler)
    ctx.register_tool(name="aphrodite_catalog", schema=CATALOG_SCHEMA, handler=_catalog_handler)

    engine_configured = os.environ.get("APHRODITE_CONTEXT_ENGINE", "") == "1"
    if engine_configured:
        try:
            ctx.register_context_engine(AphroditeContextEngine())
            _log.info("aphrodite context engine registered")
        except Exception as e:
            _log.debug("context engine registration skipped: %s", e)
    else:
        _log.info("context engine not registered - set APHRODITE_CONTEXT_ENGINE=1 to enable")

    _log.info("aphrodite v%s registered — 9 tools + hooks", PLUGIN_VERSION)

    if DEBUG_LOGGING:
        lines = [
            "=" * 60,
            f"APHRODITE v{PLUGIN_VERSION} - DEBUG MODE",
            f"  Mode: {'proxy+hooks' if not engine_configured else 'proxy+hooks+engine'} | Engine: {'enabled' if engine_configured else 'off'} | Dev: {'on' if _DEV else 'off'}",
            f"  Thresholds: terminal={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_token={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE}",
            f"  Engine: threshold={ENGINE_THRESHOLD_PCT}% protect={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min_msgs={ENGINE_MIN_MSGS}",
            f"  CCR: regex={_CCR_RE.pattern} depth={RECURSIVE_DEPTH}",
            "  Tools: retrieve, compress, stats, rebuild, files, diff, search, test, catalog",
            f"  Catalog mode: {CATALOG_MODE} (APHRODITE_CATALOG=full|compact|tool)",
            "  Proxies: cache=:9797 token=:9798 | waiting for session_start...",
            "=" * 60,
        ]
        for line in lines:
            print(line)
            _log.info(line)
