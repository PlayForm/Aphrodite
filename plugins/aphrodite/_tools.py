"""aphrodite — tool handlers and schemas."""

import hashlib
import json
import logging
import urllib.request

from ._core import _inline_store
from ._resolve import _resolve_recursive

_log = logging.getLogger("aphrodite")

# ── Tools ─────────────────────────────────────────────────────


def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers with recursive depth. Scans for nested markers."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "")
    query = args.get("query", "")
    if not hash_val:
        return '{"error": "missing hash parameter"}'
    try:
        content = _resolve_recursive(hash_val)
        if content and not content.startswith("<<<CCR:"):
            if query:
                lines = [l for l in content.splitlines() if query.lower() in l.lower()]
                if lines:
                    return "\n".join(lines)
                return content  # no matches, return full content
            return content
        return f'{{"error": "CCR entry not found: {hash_val}"}}'
    except Exception as e:
        return f'{{"error": "retrieve failed: {str(e)}"}}'


def _compress_handler(args=None, **kwargs):
    """Compress content into CCR via aphrodite proxy. Content-addressable:
    checks local cache first, only hits proxy on miss."""
    args = args if isinstance(args, dict) else {}
    content = args.get("content", "")
    type_hint = args.get("type", "text")
    if not content:
        return '{"error": "missing content parameter"}'

    # Pop the API: check local cache first (content-addressable store)
    h = hashlib.sha256(content.encode("utf-8")).hexdigest()[:16]
    if h in _inline_store:
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "source": "cache", "compression_ratio": 0}
        )

    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/ccr/create", data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        h = result.get("hash", h)
        if h:
            _inline_store[h] = content  # mirror in inline store for aphrodite_search
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "compression_ratio": result.get("compression_ratio")}
        )
    except Exception:
        # Fallback: store inline anyway
        _inline_store[h] = content
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "source": "inline_fallback", "compression_ratio": 0}
        )


COMPRESS_SCHEMA = {
    "name": "aphrodite_compress",
    "description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to compress and store in CCR"},
            "type": {
                "type": "string",
                "description": "Optional: content type hint - code, log, diff, error, json, build_output, text",
            },
        },
        "required": ["content"],
    },
}
RETRIEVE_SCHEMA = {
    "name": "aphrodite_retrieve",
    "description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads. Recursively resolves nested CCR markers up to 3 levels deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve"},
            "query": {
                "type": "string",
                "description": "Optional: filter retrieved content to lines containing this query string",
            },
            "path": {"type": "string", "description": "Optional: file path to read directly (bypasses CCR)"},
        },
        "required": [],
    },
}
