"""aphrodite core - constants, thresholds, CCR regex, inline store."""

import logging
import os
import re
from collections import OrderedDict

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.5.60"  # binary download version (must match Cargo.toml)
PLUGIN_VERSION = "1.62.6"  # plugin version
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")


# ── Configurable thresholds (env vars) ────────────────────────
def _cfg_int(name, default):
    try:
        return int(os.environ.get(name, str(default)))
    except Exception:
        return default


ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 50)
ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 1)
ENGINE_PROTECT_LAST = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 1)
ENGINE_MIN_MSGS = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 4)
TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 1024)
TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 8192)
TERMINAL_THRESHOLD = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 2048)
INLINE_THRESHOLD = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)
RECURSIVE_DEPTH = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)
DEBUG_LOGGING = os.environ.get("APHRODITE_DEBUG", "") == "1"
CATALOG_MODE = os.environ.get("APHRODITE_CATALOG", "compact")

_DEV = os.environ.get("APHRODITE_DEV", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"

if _DEV:
    _log.warning("aphrodite DEV MODE - plugin disabled, use cargo watch for proxies")
if DEBUG_LOGGING:
    _log.setLevel(logging.DEBUG)
    _log.debug(
        "aphrodite v%s debug logging enabled | engine_threshold=%s protect_first=%s protect_last=%s "
        "min_msgs=%s tool_token=%s tool_cache=%s term=%s inline=%s",
        PLUGIN_VERSION,
        ENGINE_THRESHOLD_PCT,
        ENGINE_PROTECT_FIRST,
        ENGINE_PROTECT_LAST,
        ENGINE_MIN_MSGS,
        TOOL_THRESHOLD_TOKEN,
        TOOL_THRESHOLD_CACHE,
        TERMINAL_THRESHOLD,
        INLINE_THRESHOLD,
    )

# ── CCR regex (shared) ───────────────────────────────────────
_CCR_RE = re.compile(r"<<<CCR:([^>]{1,100})>>>")

# ── Inline compression store (session-scoped) ─────────────────
_inline_store = OrderedDict()

# ── Shared session state ──────────────────────────────────────
_referenced_files = {}  # {filepath: last_tool_name}
_recent_markers = []  # [{hash, type, size, preview}]
_conv_index = {}  # {turn_num: (hash, summary, size)}
_state = {"turn_counter": 0}
_git_cache = {}  # {ts, summary}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}


# ── Shared utilities ──────────────────────────────────────────
def _reset_turn_counter():
    _state["turn_counter"] = 0


def _increment_turn():
    _state["turn_counter"] += 1
    return _state["turn_counter"]


def _get_turn_counter():
    return _state["turn_counter"]


def _fmt_size(b):
    if b >= 1_000_000:
        return f"{b / 1_000_000:.1f}MB"
    if b >= 1000:
        return f"{b / 1000:.1f}KB"
    return f"{b}B"


def _inline_clear():
    """Clear the inline store (called on session reset)."""
    _inline_store.clear()
