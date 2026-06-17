"""aphrodite — live container: async tool result via CCR markers.

When a read_file (or similar file-reading tool) produces output above
threshold, we return a CCR marker instead of full content. The LLM sees
a tiny marker and continues reasoning. When it actually needs the content,
it polls via aphrodite_retrieve(hash).

Pattern: same as terminal(background=true) — return handle immediately,
content loads behind the scenes.
"""

import json
import logging

from .._core import (
    INLINE_THRESHOLD,
    PORTS,
    _detect_model_family,
    _inline_store_put,
    _recent_markers,
    _state,
)
from .._marker import (
    _ccr_marker,
    _classify_content,
    _compress_via_proxy,
    _make_ccr_preview,
)
from .._proxy import _alive_cached, _headroom_context

_log = logging.getLogger("aphrodite.hooks.live")

# Tools that get live container treatment
_LIVE_TOOLS: frozenset = frozenset({
    "read_file",
    "search_files",
})


def _is_live_tool(tool_name: str) -> bool:
    """Check if this tool should get live container treatment."""
    return tool_name in _LIVE_TOOLS


def _wrap_as_live_container(
    tool_name: str,
    result: str,
    args: dict | None = None,
) -> str | None:
    """Wrap tool result in a live container CCR marker.

    Returns None if result is too small or tool shouldn't be intercepted.
    Returns the CCR marker string if live container applied.
    """
    if not result or not isinstance(result, str) or not result.strip():
        return None

    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive
    if not proxy_available:
        return None

    result_len = len(result)
    if result_len < INLINE_THRESHOLD:
        return None

    # Already a CCR marker — don't double-wrap
    if result.startswith("<<<CCR:") or "<<<CCR:" in result[:100]:
        return None

    target = PORTS["token"] if token_alive else PORTS["cache"]
    label = "token" if token_alive else "cache"

    klass = _classify_content(result)
    preview = _make_ccr_preview(result, klass=klass, model_family=_detect_model_family())

    ccr = _compress_via_proxy(result, target, headers=_headroom_context or None)
    if not ccr:
        return None

    h, _sz = ccr
    _inline_store_put(h, result)

    path_hint = ""
    if args and isinstance(args, dict):
        p = args.get("path", args.get("paths", ""))
        if p:
            path_hint = f";path={str(p)[:80]}"

    marker = _ccr_marker(
        h, "live", result_len, label, preview,
        headroom_budget=_headroom_context.get("x-headroom-budget"),
    )

    _recent_markers.append({
        "hash": h, "type": "live", "size": result_len,
        "preview": preview, "turn": _state.get("turn_counter", 0),
    })

    _log.info(
        "live_container: %s → CCR:%s size=%s %s",
        tool_name, h, result_len, path_hint,
    )

    return marker
