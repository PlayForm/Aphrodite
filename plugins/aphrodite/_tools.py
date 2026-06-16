"""aphrodite - tool handlers and schemas."""

import hashlib
import json
import logging
import os
import urllib.request

from ._core import PORTS, _hash_alias, _inline_store, _inline_store_put
from ._proxy import _alive_cached
from ._resolve import _filter_lines, _resolve_recursive

_log = logging.getLogger("aphrodite")

# ── Workspace boundary for path-mode file reads ────────────────
_WORKSPACE_ROOT = os.path.realpath(".")
_MAX_PATH_READ = 10_485_760  # 10MB cap

# ── Tools ─────────────────────────────────────────────────────


def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers with recursive depth. Scans for nested markers."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "").strip()
    # Defensive: if the user passes a full <<<CCR:hash|type|size>>> marker, extract just the hash
    if hash_val.startswith("<<<CCR:"):
        hash_val = hash_val.removeprefix("<<<CCR:").removesuffix(">>>").strip()
    if "|" in hash_val:
        hash_val = hash_val.split("|")[0].strip()
    query = args.get("query", "")
    path = args.get("path", "").strip()
    if not hash_val and not path:
        return json.dumps({"error": "missing hash or path parameter"})
    try:
        if path:
            resolved = os.path.realpath(path)
            if not resolved.startswith(_WORKSPACE_ROOT):
                return json.dumps({"error": f"path outside workspace boundary: {path}"})
            if not os.path.isfile(resolved):
                return json.dumps({"error": f"not a file: {path}"})
            with open(resolved, "r") as f:
                content = f.read(_MAX_PATH_READ)
                remainder = f.read(1)
                if remainder:
                    content += "\n... [truncated at 10MB]"
            if query:
                content = _filter_lines(content, query)
            return json.dumps({"content": content, "path": path, "size": len(content)})
        content = _resolve_recursive(hash_val)
        if content is not None and not content.startswith("<<<CCR:"):
            if query:
                content = _filter_lines(content, query)
            return json.dumps({"content": content, "hash": hash_val, "size": len(content)})
        return json.dumps({"error": f"CCR entry not found: {hash_val}"})
    except MemoryError:
        raise
    except PermissionError:
        return json.dumps({"error": f"permission denied: {path}"})
    except IsADirectoryError:
        return json.dumps({"error": f"is a directory, not a file: {path}"})
    except FileNotFoundError:
        return json.dumps({"error": f"file not found: {path}"})
    except Exception as e:
        return json.dumps({"error": f"retrieve failed: {e}"})


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
    h = full_h[:24]
    canonical = _hash_alias.get(full_h, h)
    if canonical in _inline_store:
        return json.dumps(
            {"hash": canonical, "type": type_hint, "size": len(content), "source": "cache_hit", "compression_ratio": 1.0, "note": "already in store"}
        )

    try:
        data = content.encode("utf-8")
        target = PORTS["token"] if _alive_cached(PORTS["token"]) else PORTS["cache"]
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create", data=data, headers={"Content-Type": "application/octet-stream"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        h = result.get("hash", h)
        if h:
            _hash_alias[full_h] = h
            _inline_store_put(h, content)  # mirror in inline store (canonical key)
        return json.dumps(
            {"hash": h, "type": type_hint, "size": len(content), "compression_ratio": result.get("compression_ratio")}
        )
    except Exception:
        # Fallback: store inline under canonical short hash
        _hash_alias[full_h] = h
        _inline_store_put(h, content)
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
        "required": [],
    },
}
