"""aphrodite - tool handlers and schemas."""

import hashlib
import json
import logging
import urllib.request

from ._core import PORTS, _inline_store, _inline_store_put
from ._proxy import _alive
from ._resolve import _resolve_recursive

_log = logging.getLogger("aphrodite")

# ── Tools ─────────────────────────────────────────────────────


def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers with recursive depth. Scans for nested markers."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "").strip()
    # Defensive: if the user passes a full <<<CCR:hash|type|size>>> marker, extract just the hash
    if "|" in hash_val:
        hash_val = hash_val.split("|")[0].strip()
    query = args.get("query", "")
    if not hash_val:
        return '{"error": "missing hash parameter"}'
    try:
        content = _resolve_recursive(hash_val)
        if content is not None and not content.startswith("<<<CCR:"):
            if query:
                lines = [l for l in content.splitlines() if query.lower() in l.lower()]
                if lines:
                    return json.dumps({"content": "\n".join(lines), "hash": hash_val, "size": len("\n".join(lines))})
                return json.dumps({"content": content, "hash": hash_val, "size": len(content)})
            return json.dumps({"content": content, "hash": hash_val, "size": len(content)})
        return f'{{"error": "CCR entry not found: {hash_val}"}}'
    except Exception as e:
        return f'{{"error": "retrieve failed: {str(e)}"}}'


def _compress_handler(args=None, **kwargs):
    """Compress content into CCR via aphrodite proxy. Content-addressable:
    checks local cache first, only hits proxy on miss."""
    args = args if isinstance(args, dict) else {}
    content = args.get("content", "")
    # Guard: non-string content (e.g. dict/list) must be serialized
    if not isinstance(content, str):
        try:
            content = json.dumps(content)
        except Exception:
            return '{"error": "content must be a string or JSON-serializable"}'
    type_hint = args.get("type", "text")
    if not content:
        return '{"error": "missing content parameter"}'

    # Pop the API: check local cache first (content-addressable store)
    full_h = hashlib.sha256(content.encode("utf-8")).hexdigest()
    h = full_h[:16]
    if h in _inline_store or full_h in _inline_store:
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "source": "cache_hit", "compression_ratio": None, "note": "already in store"}
        )

    try:
        data = json.dumps({"content": content}).encode()
        target = PORTS["token"] if _alive(PORTS["token"]) else PORTS["cache"]
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create", data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        h = result.get("hash", h)
        if h:
            _inline_store_put(h, content)  # mirror in inline store for aphrodite_search
            _inline_store_put(full_h, content)  # also store under full hash
            _inline_store.move_to_end(h)
            _inline_store.move_to_end(full_h)
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "compression_ratio": result.get("compression_ratio")}
        )
    except Exception:
        # Fallback: store inline anyway (both short and full hash)
        _inline_store_put(h, content)
        _inline_store_put(full_h, content)
        _inline_store.move_to_end(h)
        _inline_store.move_to_end(full_h)
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
            "hash": {"type": "string", "description": "CCR marker hash to retrieve. Extract from <<<CCR:hash|type|size>>> markers - the hash is the first pipe-delimited segment."},
            "query": {
                "type": "string",
                "description": "Optional: filter retrieved content to lines containing this query string",
            },
            "path": {"type": "string", "description": "Optional: file path to read directly (bypasses CCR)"},
        },
        "required": ["hash"],
    },
}
