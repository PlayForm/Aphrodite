"""
aphrodite v1.61.0 - CCR compression plugin for Hermes Agent.

Auto-install + launch aphrodite proxies:
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold

Modules:
  _core      - constants, thresholds, CCR regex, inline store
  _inline    - zlib fallback compression
  _marker    - CCR marker formatting, proxy compression, marker parsing
  _binary    - binary download + platform detection
  _proxy     - proxy lifecycle (env, health, launch)
  _tools     - 9 tool handlers + schemas, file tracking, conversation memory
  _hooks     - Hook handlers (transform_tool_result, terminal, pre_llm, post_llm)
  _engine    - ContextEngine for Hermes compression pipeline

On session_start: downloads binary, launches token proxy on :9798.
"""

import hashlib
import json
import logging
import os
import sys
import time
from pathlib import Path

# ── Core imports (re-export everything) ──────────────────────────
from ._core import (
    _CCR_RE,
    _DEV,
    _FILE_TOOLS,
    BIN_VERSION,
    BINARY,
    BINARY_DIR,
    CATALOG_MODE,
    CONTEXT_ENGINE,
    DEBUG_LOGGING,
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    ENV_FILE,
    INLINE_THRESHOLD,
    PLUGIN_VERSION,
    PORTS,
    RECURSIVE_DEPTH,
    REPO,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _cfg_int,
    _conv_index,
    _fmt_size,
    _get_turn_counter,
    _git_cache,
    _increment_turn,
    _inline_clear,
    _inline_store,
    _recent_markers,
    _referenced_files,
    _reset_turn_counter,
)

# ── Engine ────────────────────────────────────────────────────────
from ._engine import (
    AphroditeContextEngine,
    _fire_hook,
    _set_engine,
    get_engine,
)

# ── Hooks (contains some tools + all hook handlers) ────────────────
from ._hooks import (
    CATALOG_SCHEMA,
    DIFF_SCHEMA,
    FILES_SCHEMA,
    PREFETCH_SCHEMA,
    REBUILD_SCHEMA,
    RECLASSIFY_SCHEMA,
    SEARCH_SCHEMA,
    STATS_SCHEMA,
    TEST_SCHEMA,
    _aphrodite_reclassify_handler,
    _catalog_handler,
    _diff_handler,
    _extract_preview,
    _files_handler,
    _git_summary,
    _group_into_turns,
    _pre_llm_hook,
    _prefetch_handler,
    _rebuild_handler,
    _search_handler,
    _stats_handler,
    _store_conversation_turn,
    _test_handler,
    _track_file_refs,
    _transform_terminal_hook,
    _transform_tool_result,
)
from ._inline import _inline_compress, _inline_retrieve
from ._marker import _ccr_marker, _compress_via_proxy, _parse_ccr_markers
from ._proxy import _alive, _alive_cache, _load_env, _start, _wait_alive, on_start
from ._resolve import _resolve_one, _resolve_recursive

# ── Tools + state ─────────────────────────────────────────────────
from ._tools import (
    COMPRESS_SCHEMA,
    RETRIEVE_SCHEMA,
    _compress_handler,
    _retrieve_handler,
)

# Local logger (not re-exported from _core)
_log = logging.getLogger("aphrodite")

# Sync docstring version with PLUGIN_VERSION
__doc__ = (__doc__ or "").replace("v1.61.0", f"v{PLUGIN_VERSION}")


# ── Plugin registration ───────────────────────────────────────────
def register(ctx):
    """Register hooks, tools, and context engine with Hermes."""
    ctx.register_hook("on_session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
    ctx.register_hook("post_llm_call", _store_conversation_turn)
    ctx.register_hook("transform_tool_result", _transform_tool_result)

    ctx.register_tool(name="aphrodite_rebuild", schema=REBUILD_SCHEMA, handler=_rebuild_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_compress", schema=COMPRESS_SCHEMA, handler=_compress_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_retrieve", schema=RETRIEVE_SCHEMA, handler=_retrieve_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_stats", schema=STATS_SCHEMA, handler=_stats_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_files", schema=FILES_SCHEMA, handler=_files_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_diff", schema=DIFF_SCHEMA, handler=_diff_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_search", schema=SEARCH_SCHEMA, handler=_search_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_test", schema=TEST_SCHEMA, handler=_test_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_catalog", schema=CATALOG_SCHEMA, handler=_catalog_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_reclassify", schema=RECLASSIFY_SCHEMA, handler=_aphrodite_reclassify_handler, toolset="aphrodite")
    ctx.register_tool(name="aphrodite_prefetch", schema=PREFETCH_SCHEMA, handler=_prefetch_handler, toolset="aphrodite")

    engine_configured = CONTEXT_ENGINE
    if engine_configured:
        try:
            engine = AphroditeContextEngine()
            ctx.register_context_engine(engine)
            # Explicitly register on_session_start - Hermes may not auto-call it on engines
            ctx.register_hook("on_session_start", engine.on_session_start)
            _log.info("aphrodite context engine registered")
        except Exception as e:
            msg = f"aphrodite context engine registration failed [{type(e).__name__}]: {e}"
            _log.warning(msg)
            print(msg, file=sys.stderr)
    else:
        _log.info("context engine not registered - set APHRODITE_CONTEXT_ENGINE=1 to enable")

    # -- Bundle skills (namespaced as aphrodite:*) -----------------------------
    _skills_dir = Path(__file__).parent / "skills"
    _skills = [
        ("aphrodite-dev-workflow", "End-to-end aphrodite development: cargo watch, proxy, smoke tests"),
        ("aphrodite-hook-reference", "Complete Hermes hook API reference for the aphrodite plugin"),
        ("aphrodite-iterate-release", "Iterative aphrodite development loop: fix, bump, build, test"),
    ]
    for _name, _desc in _skills:
        ctx.register_skill(_name, _skills_dir / _name / "SKILL.md", _desc)

    _log.info("aphrodite v%s registered - 10 tools + 3 skills + hooks", PLUGIN_VERSION)

    if DEBUG_LOGGING:
        lines = [
            "=" * 60,
            f"APHRODITE v{PLUGIN_VERSION} - DEBUG MODE",
            f"  Mode: {'proxy+hooks' if not engine_configured else 'proxy+hooks+engine'} | Engine: {'enabled' if engine_configured else 'off'} | Dev: {'on' if _DEV else 'off'}",
            f"  Thresholds: terminal={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_token={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE}",
            f"  Engine: threshold={ENGINE_THRESHOLD_PCT}% protect={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min_msgs={ENGINE_MIN_MSGS}",
            f"  CCR: regex={_CCR_RE.pattern} depth={RECURSIVE_DEPTH}",
            "  Tools: retrieve, compress, stats, rebuild, files, diff, search, test, catalog, reclassify",
            f"  Catalog mode: {CATALOG_MODE} (APHRODITE_CATALOG=full|compact|tool)",
            "  Proxies: cache=:9797 token=:9798 | waiting for session_start...",
            "=" * 60,
        ]
        for line in lines:
            print(line)
            _log.info(line)
