"""aphrodite - CCR resolution (retrieve + recursive unpacking)."""

import json
from concurrent.futures import ThreadPoolExecutor, as_completed

from ._core import _CCR_RE, PORTS, RECURSIVE_DEPTH, _inline_store_put
from ._inline import _inline_retrieve
from ._marker import _get_conn, _put_conn  # reuse keep-alive connection pool

# Shared executor for concurrent proxy lookups - avoids per-call creation overhead
_EXECUTOR = ThreadPoolExecutor(max_workers=2)


def _filter_lines(content: str, query: str) -> str:
    """Filter content to lines containing the query string (case-insensitive).

    Returns filtered joined lines, or the original content if no lines match.
    """
    if not query:
        return content
    lines = [l for l in content.splitlines() if query.lower() in l.lower()]
    return "\n".join(lines) if lines else content


def _resolve_one(hash_val, timeout=4, query=""):
    """Resolve a single CCR hash. Checks inline store first, then tries both proxies
    concurrently using ThreadPoolExecutor for reduced latency.

    Resolves exactly ONE hash - does NOT unpack nested <<<CCR:...>>> markers.
    Returns the content string on success, or None if the hash cannot be resolved
    from any source (inline store, token proxy, cache proxy).

    Use _resolve_recursive when the content may contain nested markers.
    Use _resolve_one when you only need the raw content for a single hash."""
    # i: prefix hashes are inline-only - skip proxy entirely
    if hash_val.startswith("i:"):
        content = _inline_retrieve(hash_val)
        if content is not None:
            return _filter_lines(content, query)
        return None
    content = _inline_retrieve(hash_val)
    if content is not None:
        return _filter_lines(content, query)
    # Try both proxies concurrently
    payload = {"hash": hash_val}
    if query:
        payload["query"] = query
    futures = {}
    for port in (PORTS["token"], PORTS["cache"]):
        futures[_EXECUTOR.submit(_proxy_lookup, port, payload, timeout)] = port
    for future in as_completed(futures):
        try:
            result = future.result()
            if result is not None:
                return result
        except Exception:
            continue
    return None


def _proxy_lookup(port: int, payload: dict, timeout: int = 4) -> str | None:
    """Try a single proxy port and return the content if found.

    Uses thread-local keep-alive HTTP connection from _marker.py to avoid
    TCP handshake overhead on repeated calls to the same proxy port.
    Broken connections are evicted and recreated on the next call.
    """
    try:
        data = json.dumps(payload).encode()
        conn = _get_conn(port)
        conn.request(
            "POST",
            "/retrieve",
            body=data,
            headers={"Content-Type": "application/json", "Connection": "keep-alive"},
        )
        r = conn.getresponse()
        result = json.loads(r.read())
        r.close()
        if result.get("found"):
            return result["content"]
    except Exception:
        _put_conn(port)  # ditch broken connection, reopen fresh next time
    return None


def _resolve_recursive(hash_val, depth=0, resolved=None, _visited=None):
    """Resolve a CCR hash and recursively unpack all nested <<<CCR:...>>> markers.

    Calls _resolve_one to fetch the content for the top-level hash, then scans
    the result for nested <<<CCR:...>>> markers and resolves each one recursively
    up to RECURSIVE_DEPTH levels deep (default 5). Uses ``_visited`` set to prevent
    infinite recursion on self-referential or circular markers, and ``resolved``
    dict to cache already-resolved hashes so they are not re-fetched.

    Returns the fully resolved content string, or None if the hash was not
    resolved (e.g. max depth exceeded with no cached result). When a hash
    cannot be resolved from any source, the unresolved marker is preserved
    as-is: ``<<<CCR:hash|unresolved>>>``.

    Use _resolve_one when you only need the raw content for a single hash and
    do NOT need to unpack nested markers."""
    if resolved is None:
        resolved = {}
    if _visited is None:
        _visited = set()
    if hash_val in _visited:
        return resolved.get(hash_val)
    _visited.add(hash_val)
    if depth >= RECURSIVE_DEPTH or hash_val in resolved:
        return resolved.get(hash_val)
    content = _resolve_one(hash_val)
    if content is None:
        return f"<<<CCR:{hash_val}|unresolved>>>"
    resolved[hash_val] = content
    # Use finditer to get full match strings (group(0)) plus capture groups
    nested = list(_CCR_RE.finditer(content))
    if not nested:
        return content
    replacements = {}
    for match in nested:
        full_marker = match.group(0)
        parts = match.group(1).split("|")
        if len(parts) >= 1 and parts[0] not in resolved:
            nested_hash = parts[0]
            nested_content = _resolve_recursive(nested_hash, depth + 1, resolved)
            replacements[full_marker] = nested_content
    for marker_str, replacement in replacements.items():
        content = content.replace(marker_str, replacement)
    # Guard re-store: skip if expanded content exceeds 512KB to avoid ballooning inline store
    if len(content) <= 524288:
        _inline_store_put(hash_val, content)
    return content
