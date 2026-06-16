"""aphrodite core - constants, thresholds, CCR regex, inline store."""

import logging
import os
import re
from collections import OrderedDict, deque

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.5.86"  # binary download version (must match Cargo.toml)
PLUGIN_VERSION = "1.62.14"  # plugin version
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


ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 65)
# Semantics: -1 = always compress, 0 = disabled, >0 = fill% threshold
# Coding-tuned: compress later (65% vs old 50%) to preserve tool chains + semantics
ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 5)
ENGINE_PROTECT_LAST = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 5)
# Coding-tuned: 5 head + 5 tail messages kept raw to avoid tool_call↔result splits
ENGINE_MIN_MSGS = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 12)
# Coding-tuned: don't compress sessions under 12 messages — tool chains need room
TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 1024)
TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 8192)
TERMINAL_THRESHOLD = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 2048)
INLINE_THRESHOLD = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)
# HEADROOM_SSE_BUFFER_MAX_BYTES check: if set, bump INLINE_THRESHOLD to 1MB
# so headroom's SSE buffer isn't overwhelmed by small inline compressions
if os.environ.get("HEADROOM_SSE_BUFFER_MAX_BYTES"):
    INLINE_THRESHOLD = max(INLINE_THRESHOLD, 1_048_576)
RECURSIVE_DEPTH = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)
AUTO_EXPAND_LIMIT = _cfg_int("APHRODITE_AUTO_EXPAND_LIMIT", 51200)
DEBUG_LOGGING = os.environ.get("APHRODITE_DEBUG", "") == "1"
CATALOG_MODE = os.environ.get("APHRODITE_CATALOG", "compact")

# Big-payload guard: skip compression entirely for payloads exceeding this
MAX_REQUEST_BODY_SIZE = _cfg_int("APHRODITE_MAX_REQUEST_BODY_SIZE", 104_857_600)  # 100MB default

_DEV = os.environ.get("APHRODITE_PASSTHROUGH", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"

if _DEV:
    _log.warning("aphrodite PASSTHROUGH MODE - plugin disabled, use cargo watch for proxies")
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
_CCR_RE = re.compile(r'(?:\[|<<<|⫷)CCR:([^|\\>⫸]+)(?:\|[^\\\]]*?)?(?:\]|>>>|⫸)')

# ── Hash alias: maps full SHA256 hash → short 16-char hash ──
_hash_alias: dict = {}  # {full_sha256: short_hash}

# ── Inline compression store + trigram index (session-scoped, capped at 500) ──
class _CappedStore(OrderedDict):
    """OrderedDict that auto-evicts oldest entries when exceeding MAX_STORE."""

    MAX_STORE = 500

    def __setitem__(self, key, value):
        super().__setitem__(key, value)
        if len(self) > self.MAX_STORE:
            self.popitem(last=False)

    def popitem(self, last=True):
        key, value = super().popitem(last=last)
        global _inline_bytes
        _inline_bytes -= len(value) if value else 0
        if _inline_index_enabled:
            _remove_trigram_index(key)
        return key, value


_inline_store: _CappedStore = _CappedStore()
_inline_index: dict = {}  # {trigram: set_of_hashes} for O(1) search
_inline_bytes: int = 0  # tracked byte count (avoids sum(len(v) for v ...))
_inline_index_enabled: bool = False  # lazily enabled on first index build
_hash_to_trigrams: dict = {}  # {hash: set_of_trigrams} reverse index for O(1) eviction

# ── Shared session state ──────────────────────────────────────
_referenced_files: OrderedDict = OrderedDict()  # {filepath: last_tool_name} LRU via move_to_end
_recent_markers: deque = deque(maxlen=_cfg_int("APHRODITE_RECENT_MARKERS_MAX", 500))  # [{hash, type, size, preview, turn}] deque auto-evicts oldest; tuned for bursty workflows
_conv_index = {}  # {turn_num: (hash, summary, size)}
_state = {"turn_counter": 0}
_scanned_msg_idx = 0  # for incremental marker scan in pre_llm_hook
_git_cache = {}  # {ts, summary}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}


# ── Shared utilities ──────────────────────────────────────────
def _reset_scanned_msg_idx():
    global _scanned_msg_idx
    _scanned_msg_idx = 0


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
    global _inline_bytes, _inline_index_enabled
    _inline_store.clear()
    _inline_index.clear()
    _hash_to_trigrams.clear()
    _inline_bytes = 0
    _inline_index_enabled = False


def _init_trigram_index():
    """Build trigram index from all inline store entries (one-time)."""
    global _inline_index_enabled
    _inline_index.clear()
    for h, content in _inline_store.items():
        _index_trigrams(h, content)
    _inline_index_enabled = True


def _index_trigrams(h, content):
    """Split content into trigrams and index under hash. Populates both
    forward (_inline_index) and reverse (_hash_to_trigrams) indices."""
    lower = content.lower()
    trigrams = {lower[i : i + 3] for i in range(len(lower) - 2)}
    _hash_to_trigrams[h] = trigrams
    for tri in trigrams:
        _inline_index.setdefault(tri, set()).add(h)


def _remove_trigram_index(h):
    """Remove all index entries for a given hash (O(1) via reverse index)."""
    trigrams = _hash_to_trigrams.pop(h, ())
    for tri in trigrams:
        s = _inline_index.get(tri)
        if s:
            s.discard(h)
            if not s:
                del _inline_index[tri]


def _inline_store_put(h, content):
    """Store content in inline store with LRU eviction at MAX=500.

    __setitem__ handles ordering and eviction automatically. On update,
    old trigrams are unindexed before re-indexing the new content.
    Returns True if the entry was newly added, False if updated.
    """
    global _inline_bytes
    is_new = h not in _inline_store
    if not is_new:
        old_len = len(_inline_store[h])
        _inline_bytes -= old_len
        if _inline_index_enabled:
            _remove_trigram_index(h)
    _inline_store[h] = content
    _inline_bytes += len(content)
    # Index trigrams for search
    if _inline_index_enabled:
        _index_trigrams(h, content)
    return is_new
