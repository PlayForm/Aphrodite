"""aphrodite — tool handlers and schemas."""
import os, json, time, hashlib, urllib.request
import logging
from ._core import (_inline_store, _CCR_RE, RECURSIVE_DEPTH, PLUGIN_VERSION,
    PORTS, BINARY, DEBUG_LOGGING, _DEV, TERMINAL_THRESHOLD, INLINE_THRESHOLD,
    TOOL_THRESHOLD_TOKEN, TOOL_THRESHOLD_CACHE, ENGINE_THRESHOLD_PCT,
    ENGINE_PROTECT_FIRST, ENGINE_PROTECT_LAST, ENGINE_MIN_MSGS, CATALOG_MODE)
from ._proxy import _alive
from ._resolve import _resolve_one, _resolve_recursive
from ._marker import _compress_via_proxy, _ccr_marker, _parse_ccr_markers
from ._inline import _inline_compress, _inline_retrieve

_log = logging.getLogger("aphrodite")
_referenced_files = {}
_recent_markers = []
_conv_index = {}
_turn_counter = 0
_git_cache = {}
_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}

# ── Tools ─────────────────────────────────────────────────────

def _resolve_recursive(hash_val, depth=0, resolved=None):
    """Recursively resolve CCR markers in content, up to max depth.
    
    After retrieving content, scans for nested <<<CCR:...>>> markers
    and resolves them in parallel, replacing markers with resolved content.
    """
    if resolved is None:
        resolved = {}
    
    if depth >= RECURSIVE_DEPTH or hash_val in resolved:
        return resolved.get(hash_val, "")
    
    content = _resolve_one(hash_val)
    if content is None:
            return f'<<<CCR:{hash_val}|unresolved>>>'
    
    resolved[hash_val] = content
    
    # Find nested CCR markers
    nested = _CCR_RE.findall(content)
    if not nested:
        return content
    
    # Resolve nested markers in parallel (sequential for simplicity)
    replacements = {}
    for marker in nested:
        parts = marker.split('|')
        if len(parts) >= 1 and parts[0] not in resolved:
            nested_hash = parts[0]
            nested_content = _resolve_recursive(nested_hash, depth + 1, resolved)
            replacements[f'<<<CCR:{marker}>>>'] = nested_content
    
    # Replace markers with resolved content
    for marker_str, replacement in replacements.items():
        content = content.replace(marker_str, replacement)
    
    return content


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
    h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
    if h in _inline_store:
        return json.dumps({"hash": h, "type": type_hint, "size": len(content), 
                           "source": "cache", "compression_ratio": 0})
    
    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        h = result.get("hash", h)
        if h:
            _inline_store[h] = content  # mirror in inline store for aphrodite_search
        return json.dumps({
            "hash": h,
            "type": type_hint,
            "size": len(content),
            "compression_ratio": result.get("compression_ratio")
        })
    except Exception as e:
        # Fallback: store inline anyway
        _inline_store[h] = content
        return json.dumps({"hash": h, "type": type_hint, "size": len(content), 
                           "source": "inline_fallback", "compression_ratio": 0})


COMPRESS_SCHEMA = {
    "name": "aphrodite_compress",
    "description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to compress and store in CCR"},
            "type": {"type": "string", "description": "Optional: content type hint - code, log, diff, error, json, build_output, text"}
        },
        "required": ["content"]
    }
}
RETRIEVE_SCHEMA = {
    "name": "aphrodite_retrieve",
    "description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads. Recursively resolves nested CCR markers up to 3 levels deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve"},
            "query": {"type": "string", "description": "Optional: filter retrieved content to lines containing this query string"},
            "path": {"type": "string", "description": "Optional: file path to read directly (bypasses CCR)"}
        },
        "required": []
    }
}


