"""aphrodite — ContextEngine for Hermes compression pipeline."""

import json
import logging
import urllib.request

from agent.context_engine import ContextEngine

from ._core import (
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    PLUGIN_VERSION,
    PORTS,
    _conv_index,
    _fmt_size,
    _inline_clear,
    _recent_markers,
    _referenced_files,
)
from ._inline import _inline_compress
from ._proxy import _alive

_log = logging.getLogger("aphrodite")
_engine = None


def _set_engine(eng):
    global _engine
    _engine = eng


def get_engine():
    """Return the aphrodite context engine instance, or None.

    Other plugins can call this to access the engine and its stats.
    """
    return _engine


def _fire_hook(name, **kwargs):
    """Fire a Hermes hook so other plugins can listen to engine events."""
    try:
        from hermes_cli.plugins import invoke_hook

        invoke_hook(name, **kwargs)
    except Exception:
        pass


class AphroditeContextEngine(ContextEngine):
    """CCR-based context compression engine for Hermes.

    Replaces built-in summarization compressor with CCR offloading.
    Extensible via Hermes hooks - other plugins can listen to:
      - ``aphrodite_engine_compressed`` - fired after each compression

    Set ``context.engine: aphrodite`` in config.yaml to activate.
    Works with proxy (token/cache) or inline fallback (zlib).
    """

    @property
    def name(self) -> str:
        return "aphrodite"

    threshold_percent = ENGINE_THRESHOLD_PCT
    protect_first_n = ENGINE_PROTECT_FIRST
    protect_last_n = ENGINE_PROTECT_LAST
    min_messages_to_compress = ENGINE_MIN_MSGS

    def __init__(self):
        self.last_prompt_tokens = 0
        self.last_completion_tokens = 0
        self.last_total_tokens = 0
        self.threshold_tokens = 0
        self.context_length = 1000000
        self.compression_count = 0
        self.last_compression = {}
        self.session_id = ""
        _set_engine(self)

    def update_from_response(self, usage):
        self.last_prompt_tokens = usage.get("prompt_tokens", 0) or usage.get("input_tokens", 0)
        self.last_completion_tokens = usage.get("completion_tokens", 0) or usage.get("output_tokens", 0)
        self.last_total_tokens = usage.get("total_tokens", 0)
        if self.context_length:
            self.threshold_tokens = int(self.context_length * self.threshold_percent / 100)

    def should_compress(self, prompt_tokens=None):
        if self.threshold_percent == 0:
            return False
        tokens = prompt_tokens or self.last_prompt_tokens
        if not tokens:
            return False
        if not self.context_length:
            return False
        return (tokens / self.context_length) * 100 >= self.threshold_percent

    def compress(self, messages, current_tokens=None, focus_topic=None):
        if len(messages) <= self.min_messages_to_compress:
            return messages
        head_n = self.protect_first_n
        tail_n = self.protect_last_n

        is_editing = False
        for msg in messages[-10:]:
            content = str(msg.get("content", ""))
            if msg.get("role") == "tool" and any(
                kw in content.lower()
                for kw in ("wrote", "patched", "modified", "created", "deleted", "successfully", "written")
            ):
                is_editing = True
                break
        if is_editing:
            tail_n = max(tail_n, 8)

        if len(messages) > tail_n:
            boundary = len(messages) - tail_n
            while boundary < len(messages) and messages[boundary].get("role") == "tool":
                boundary += 1
                tail_n += 1
            if boundary > 0 and messages[boundary - 1].get("role") == "assistant":
                tool_calls = messages[boundary - 1].get("tool_calls", [])
                if tool_calls:
                    boundary -= 1
                    tail_n += 1
            tail_n = min(tail_n, len(messages) - head_n)

        head = messages[:head_n]
        middle = messages[head_n:-tail_n] if tail_n > 0 else messages[head_n:]
        tail = messages[-tail_n:] if tail_n > 0 else []

        if len(middle) < 3:
            return messages

        packed = json.dumps(
            [
                {
                    "role": m.get("role", ""),
                    "content": str(m.get("content", "")),
                    "tool_call_id": m.get("tool_call_id", ""),
                }
                for m in middle
            ]
        )
        if len(packed) < 200:
            return messages

        hash_val = None
        size_str = _fmt_size(len(packed))

        token_alive = _alive(PORTS["token"])
        cache_alive = _alive(PORTS["cache"])
        if token_alive or cache_alive:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            try:
                data = json.dumps({"content": packed}).encode()
                req = urllib.request.Request(
                    f"http://127.0.0.1:{target}/ccr/create",
                    data=data,
                    headers={"Content-Type": "application/json"},
                )
                with urllib.request.urlopen(req, timeout=5) as r:
                    ccr = json.loads(r.read())
                hash_val = ccr["hash"]
            except Exception:
                pass

        if not hash_val:
            try:
                hash_val, _ = _inline_compress(packed)
                size_str = _fmt_size(len(packed))
            except Exception:
                return messages

        marker = (
            f"[CONTEXT COMPRESSED: {len(middle)} messages → "
            f"CCR:{hash_val}|{size_str}]\n"
            f"These messages were offloaded to reduce context. "
            f"Retrieve with: aphrodite_retrieve({hash_val}).\n"
            f"The {self.protect_last_n} messages below are your active context."
        )
        self.compression_count += 1
        _log.info("context_engine: compressed %d msgs → CCR:%s (%s)", len(middle), hash_val, size_str)
        self._notify_compressed(len(packed), len(middle), hash_val)
        return head + [{"role": "system", "content": marker}] + tail

    def _notify_compressed(self, packed_len, middle_len, hash_val):
        self.last_compression = {
            "messages_compressed": middle_len,
            "packed_size": packed_len,
            "hash": hash_val,
            "count": self.compression_count,
        }
        _fire_hook("aphrodite_engine_compressed", engine=self, stats=self.last_compression)

    def get_status(self):
        return {
            "last_prompt_tokens": self.last_prompt_tokens,
            "threshold_tokens": self.threshold_tokens,
            "context_length": self.context_length,
            "usage_percent": (
                min(100, self.last_prompt_tokens / self.context_length * 100) if self.context_length else 0
            ),
            "compression_count": self.compression_count,
        }

    def update_model(self, model="", context_length=0, base_url="", api_key="", provider="", api_mode="", **kw):
        if context_length:
            self.context_length = context_length
            self.threshold_tokens = 1

    def on_session_reset(self):
        global _turn_counter
        self.last_prompt_tokens = 0
        self.last_completion_tokens = 0
        self.last_total_tokens = 0
        self.compression_count = 0
        self.last_compression = {}
        _inline_clear()
        _conv_index.clear()
        _turn_counter = 0
        _referenced_files.clear()
        _recent_markers.clear()
        _log.info("aphrodite v%s: session reset - inline store + memory cleared", PLUGIN_VERSION)

    def on_session_start(self, session_id="", **kw):
        self.session_id = session_id
        _log.info("context_engine: session %s started", session_id[:16] if session_id else "?")
