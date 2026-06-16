"""aphrodite core - constants, thresholds, CCR regex, inline store."""

import logging
import os
import re
from collections import OrderedDict, deque

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.5.112"  # binary download version (must match Cargo.toml)
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


ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 55)
# Coding-optimized: compress at 55% — more headroom for new code. LLM retrieves on demand.
ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 3)
ENGINE_PROTECT_LAST = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 8)
# Coding: 3 head (less old context) + 8 tail (keep recent tool chains visible)
ENGINE_MIN_MSGS = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 12)
TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 1024)
TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 8192)
TERMINAL_THRESHOLD = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 2048)
INLINE_THRESHOLD = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)
# HEADROOM_SSE_BUFFER_MAX_BYTES check: if set, bump INLINE_THRESHOLD to 1MB
# so headroom's SSE buffer isn't overwhelmed by small inline compressions
if os.environ.get("HEADROOM_SSE_BUFFER_MAX_BYTES"):
    INLINE_THRESHOLD = max(INLINE_THRESHOLD, 1_048_576)
RECURSIVE_DEPTH = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)
# Auto-expand: OFF by default — LLM sees raw CCR markers and retrieves on demand.
# Set APHRODITE_AUTO_EXPAND=1 to enable auto-expansion (resolves markers inline).
# Set APHRODITE_AUTO_EXPAND_LIMIT=N to cap what gets auto-expanded (bytes).
AUTO_EXPAND_LIMIT = _cfg_int("APHRODITE_AUTO_EXPAND_LIMIT", 0)
if os.environ.get("APHRODITE_AUTO_EXPAND") == "1":
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


# ── Model-aware template dispatch ──────────────────────────────────────────
# Different LLM families process structured vs code-excerpt previews
# differently. The model family selects a preview strategy:
#   compact    — [type:key=val]  (Claude, default)
#   code_first — code excerpts before metadata  (DeepSeek, coding models)
#   balance    — metadata + short excerpt  (GPT, general-purpose)

MODEL_FAMILY_MAP: dict[str, str] = {
    "claude": "compact",
    "deepseek": "code_first",
    "gpt": "balance",
    "gemini": "balance",
    "llama": "compact",
    "mistral": "compact",
    "mixtral": "compact",
    "qwen": "code_first",
}


def _detect_model_family(model_name: str | None = None) -> str:
    """Detect model family from model name, env var, or state.

    Priority: explicit arg → APHRODITE_MODEL env → _state["model"] → "compact"
    """
    name = model_name or os.environ.get("APHRODITE_MODEL", "") or _state.get("model", "")
    if not name:
        return "compact"
    name_lower = name.lower().replace("-", "").replace("_", "")
    for prefix, family in MODEL_FAMILY_MAP.items():
        if name_lower.startswith(prefix):
            return family
    return "compact"


def _set_session_model(model_name: str) -> None:
    """Record the active model name in session state for preview dispatch."""
    _state["model"] = model_name


# ── Code structure extractor (regex-based, no tree-sitter) ─────────────────

import re as _re

_CODE_PATTERNS: dict[str, dict[str, _re.Pattern]] = {
    "rust": {
        "fn": _re.compile(
            r'^\s*(?:pub(?:\s*\(\s*crate\s*\))?\s+)?(?:async\s+)?fn\s+(\w+(?:::\w+)*)\s*\(([^)]*)\)(?:\s*->\s*(\S+(?:\s*\+\s*\S+)*))?',
            _re.MULTILINE,
        ),
        "struct": _re.compile(r'^\s*(?:pub\s+)?struct\s+(\w+)', _re.MULTILINE),
        "impl": _re.compile(
            r'^\s*impl(?:\s*<\s*\w+(?:\s*,\s*\w+)*\s*>)?\s+(\w+(?:::\w+)*(?:\s*<\s*\w+(?:\s*,\s*\w+)*\s*>)?)',
            _re.MULTILINE,
        ),
        "trait": _re.compile(r'^\s*(?:pub\s+)?trait\s+(\w+)', _re.MULTILINE),
        "mod": _re.compile(r'^\s*(?:pub\s+)?mod\s+(\w+)', _re.MULTILINE),
    },
    "python": {
        "def": _re.compile(r'^\s*(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)', _re.MULTILINE),
        "class": _re.compile(r'^\s*class\s+(\w+)', _re.MULTILINE),
    },
    "go": {
        "func": _re.compile(
            r'^\s*func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(([^)]*)\)', _re.MULTILINE
        ),
        "type": _re.compile(r'^\s*type\s+(\w+)\s+struct', _re.MULTILINE),
        "interface": _re.compile(r'^\s*type\s+(\w+)\s+interface', _re.MULTILINE),
    },
    "js": {
        "function": _re.compile(
            r'(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)|\b(\w+)\s*=\s*(?:async\s*)?\(([^)]*)\)\s*=>',
            _re.MULTILINE,
        ),
        "class": _re.compile(r'class\s+(\w+)', _re.MULTILINE),
    },
}


def _extract_code_structure(content: str, language: str = "") -> dict:
    """Extract function/class/struct signatures from source code.

    Returns a dict with keys like 'fns', 'structs', 'impls', 'classes', etc.
    Each value is a list of short (<60 char) signature strings.
    Total output ≤ 300 chars to stay within preview budget.
    """
    # Auto-detect language
    if not language:
        if "fn " in content[:500] and "->" in content[:500]:
            language = "rust"
        elif "def " in content[:500] and ":" in content[:500]:
            language = "python"
        elif "func " in content[:500] and "{" in content[:500]:
            language = "go"
        elif "function " in content[:500] or "=>" in content[:500]:
            language = "js"
        else:
            return {}

    pats = _CODE_PATTERNS.get(language)
    if not pats:
        return {}

    result: dict[str, list[str]] = {}
    budget = 300

    def _sig(kind: str, text: str) -> str:
        """Truncate a signature to fit preview budget."""
        s = f"{kind} {text}".strip()
        return s[:60]

    # Collect function signatures (most important for navigation)
    if "fn" in pats:
        fns = []
        for m in pats["fn"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            ret = m.group(3) if m.lastindex and m.lastindex >= 3 and m.group(3) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            ret_str = f" -> {ret.strip()}" if ret else ""
            s = f"fn {name}({params}){ret_str}"
            if len(s) > 60:
                s = s[:57] + "..."
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    if "def" in pats:
        fns = []
        for m in pats["def"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            s = f"def {name}({params})"
            s = s[:60]
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    if "func" in pats:
        fns = []
        for m in pats["func"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            s = f"func {name}({params})"
            s = s[:60]
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    # Collect structs/types/classes
    for kind, key in [("struct", "structs"), ("class", "classes"), ("type", "types")]:
        if kind in pats:
            items = []
            for m in pats[kind].finditer(content):
                s = f"{kind} {m.group(1)}"
                s = s[:60]
                items.append(s)
                budget -= len(s) + 1
                if budget < 0:
                    break
            if items:
                result.setdefault(key, items)

    if budget <= 0:
        return result

    # Collect impls (Rust)
    if "impl" in pats:
        items = []
        for m in pats["impl"].finditer(content):
            s = f"impl {m.group(1)}"
            s = s[:60]
            items.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if items:
            result["impls"] = items

    return result
