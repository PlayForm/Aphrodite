# ruff: noqa: E402
"""aphrodite — hook handlers (split across submodules for maintainability).

Re-exports all public symbols from submodules. Each submodule is <250 lines
with one primary export pattern.
"""

import logging

_log = logging.getLogger("aphrodite.hooks")

# ── Transform: tool result compression & formatting ──────────────
# ── Catalog: compression catalog handler + table-of-contents ─────
from .catalog import CATALOG_SCHEMA, _build_toc, _catalog_handler, _fmt_catalog

# ── Classify: classifier poll for skip decisions ─────────────────
from .classify import _classifier_says_skip

# ── Diff: conversation turn history ──────────────────────────────
from .diff import DIFF_SCHEMA, _diff_handler, _fmt_diff

# ── Files: file reference tracking ───────────────────────────────
from .files import FILES_SCHEMA, _files_handler, _fmt_files, _track_file_refs

# ── Git: diff summary + automation hooks ─────────────────────────
from .git import _git_summary

# ── Live: streaming terminal containers ──────────────────────────

# ── Prefetch: background file load + compress ────────────────────
from .prefetch import (
    PREFETCH_SCHEMA,
    PREFETCH_STATUS_SCHEMA,
    _prefetch_handler,
    _prefetch_registry,
    _prefetch_status_handler,
)

# ── Rebuild: cargo rebuild + proxy restart ───────────────────────
from .rebuild import REBUILD_SCHEMA, _rebuild_handler

# ── Reclassify: retroactive metadata enrichment ──────────────────
from .reclassify import RECLASSIFY_SCHEMA, _aphrodite_reclassify_handler

# ── Search: trigram-indexed CCR search ───────────────────────────
from .search import SEARCH_SCHEMA, _search_handler

# ── Session: instruction injection, pre-LLM hook, turn storage ───
from .session import (
    _inject_session_instruction,
    _pre_llm_hook,
    _store_conversation_turn,
)

# ── Session helpers: turn grouping, preview extraction, read keywords ─
from .session_helpers import _extract_preview, _group_into_turns

# ── Stats: proxy health + engine status ──────────────────────────
from .stats import STATS_SCHEMA, _fmt_stats, _stats_handler

# ── Terminal: terminal output compression ────────────────────────
from .terminal import _transform_terminal_hook

# ── Test: smoke test suite ───────────────────────────────────────
from .test import TEST_SCHEMA, _test_handler
from .transform import (
    _ESSENTIAL_TOOLS,
    _extract_tool_metadata,
    _format_aphrodite_output,
    _transform_tool_result,
)

__all__ = [
    "CATALOG_SCHEMA",
    "DIFF_SCHEMA",
    "FILES_SCHEMA",
    "PREFETCH_SCHEMA",
    "PREFETCH_STATUS_SCHEMA",
    "REBUILD_SCHEMA",
    "RECLASSIFY_SCHEMA",
    "SEARCH_SCHEMA",
    "STATS_SCHEMA",
    "TEST_SCHEMA",
    "_aphrodite_reclassify_handler",
    "_build_toc",
    "_catalog_handler",
    "_classifier_says_skip",
    "_diff_handler",
    "_extract_preview",
    "_extract_tool_metadata",
    "_files_handler",
    "_fmt_catalog",
    "_fmt_diff",
    "_fmt_files",
    "_fmt_stats",
    "_format_aphrodite_output",
    "_git_summary",
    "_group_into_turns",
    "_inject_session_instruction",
    "_pre_llm_hook",
    "_prefetch_handler",
    "_prefetch_registry",
    "_prefetch_status_handler",
    "_rebuild_handler",
    "_search_handler",
    "_stats_handler",
    "_store_conversation_turn",
    "_test_handler",
    "_track_file_refs",
    "_transform_terminal_hook",
    "_transform_tool_result",
]
