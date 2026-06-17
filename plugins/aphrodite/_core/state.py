"""aphrodite — session state: turn counter, caches, conv index, CCR regex."""

import re
from collections import OrderedDict

# ── CCR regex (shared) ───────────────────────────────────────
_CCR_RE = re.compile(r'(?:\[|<<<|⫷)CCR:([^|\\>⫸]+)(?:\|[^\\\]]*?)?(?:\]|>>>|⫸)')

# ── Hash alias: maps full SHA256 hash → short 16-char hash ──
_hash_alias: dict = {}  # {full_sha256: short_hash}

# ── Shared session state ──────────────────────────────────────
_referenced_files: OrderedDict = OrderedDict()  # {filepath: last_tool_name} LRU via move_to_end
_conv_index = {}  # {turn_num: (hash, summary, size)}
_state = {"turn_counter": 0}
_scanned_msg_idx = 0  # for incremental marker scan in pre_llm_hook
_git_cache = {}  # {ts, summary}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}


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
