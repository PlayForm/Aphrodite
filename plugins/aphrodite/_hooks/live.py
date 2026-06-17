"""aphrodite — live container bubble.

Self-contained, zero-dependency module. Invisible until activated.
When APHRODITE_LIVE_CONTAINER=1, wraps read_file results in CCR markers.
Communicates directly with the aphrodite proxy at :9798.
No imports from the aphrodite plugin — works as a standalone patch.

Apply to Hermes core by adding to file_tools.py:
    from live_container import wrap_read_result
    return wrap_read_result(result_json)
"""

import json
import os
import urllib.request

# ── Bubble config (self-contained, no TOML) ──────────────────────
_PROXY_URL = "http://127.0.0.1:9798"
_MIN_BYTES = 2048  # Only wrap results above this size
_TIMEOUT = 3       # Proxy request timeout


def _is_active() -> bool:
    """Check if live container mode is enabled."""
    return os.environ.get("APHRODITE_LIVE_CONTAINER") == "1"


def _is_live_tool(tool_name: str) -> bool:
    """Check if a tool supports live container wrapping."""
    if not _is_active():
        return False
    return tool_name in ("read_file",)


def _wrap_as_live_container(content: str, tool_name: str) -> str:
    """Wrap tool output in a live container CCR marker."""
    if not _is_live_tool(tool_name):
        return content
    if len(content) <= _MIN_BYTES:
        return content
    h = _store(content)
    if not h:
        return content
    size = len(content)
    return (
        f"<<<CCR:{h}|live|{size}>>>\n"
        f"Live container — content stored. "
        f"Use aphrodite_retrieve({h}) to fetch when needed. "
        f"Continue reasoning without waiting."
    )


def _store(content: str) -> str | None:
    """Store content in CCR proxy, return hash or None."""
    try:
        data = content.encode()
        req = urllib.request.Request(
            f"{_PROXY_URL}/ccr/create",
            data=data,
            headers={"Content-Type": "application/octet-stream"},
        )
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as r:
            ccr = json.loads(r.read())
        return ccr.get("hash")
    except Exception:
        return None


def wrap_read_result(result_json: str) -> str:
    """Wrap a read_file result in a live container CCR marker.

    Call this right before returning from read_file_tool.
    If live container is inactive or content is too small,
    returns the original result unchanged.

    Usage in file_tools.py:
        result_json = json.dumps(result_dict, ensure_ascii=False)
        return wrap_read_result(result_json)
    """
    if not _is_active():
        return result_json

    if len(result_json) <= _MIN_BYTES:
        return result_json

    h = _store(result_json)
    if not h:
        return result_json

    size = len(result_json)
    return (
        f"<<<CCR:{h}|live|{size}>>>\n"
        f"Live container — content stored. "
        f"Use aphrodite_retrieve({h}) to fetch when needed. "
        f"Continue reasoning without waiting."
    )
