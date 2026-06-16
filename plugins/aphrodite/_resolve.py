"""aphrodite - CCR resolution (retrieve + recursive unpacking)."""

import json
import urllib.request

from ._core import _CCR_RE, RECURSIVE_DEPTH, _inline_store
from ._inline import _inline_retrieve


def _resolve_one(hash_val, timeout=4, query=""):
    """Resolve a single CCR hash. Checks inline store first, then tries both proxies.

    Resolves exactly ONE hash - does NOT unpack nested <<<CCR:...>>> markers.
    Returns the content string on success, or None if the hash cannot be resolved
    from any source (inline store, token proxy, cache proxy).

    Use _resolve_recursive when the content may contain nested markers.
    Use _resolve_one when you only need the raw content for a single hash."""
    content = _inline_retrieve(hash_val)
    if content is not None:
        if query:
            lines = [l for l in content.splitlines() if query.lower() in l.lower()]
            return "\n".join(lines) if lines else content
        return content
    payload = {"hash": hash_val}
    if query:
        payload["query"] = query
    for port in (9798, 9797):
        try:
            data = json.dumps(payload).encode()
            req = urllib.request.Request(
                f"http://127.0.0.1:{port}/retrieve", data=data, headers={"Content-Type": "application/json"}
            )
            with urllib.request.urlopen(req, timeout=timeout) as r:
                result = json.loads(r.read())
            if result.get("found"):
                content = result["content"]
                _inline_store[hash_val] = content
                return content
        except Exception:
            continue
    return None


def _resolve_recursive(hash_val, depth=0, resolved=None):
    """Resolve a CCR hash and recursively unpack all nested <<<CCR:...>>> markers.

    Calls _resolve_one to fetch the content for the top-level hash, then scans
    the result for nested <<<CCR:...>>> markers and resolves each one recursively
    up to RECURSIVE_DEPTH levels deep (default 5). Also guards against circular
    references via the ``resolved`` dict (content from previously seen hash_vals
    is reused, not re-fetched).

    Returns the fully resolved content string. When a hash cannot be resolved,
    the unresolved marker is preserved as-is: ``<<<CCR:hash|unresolved>>>``.

    Use _resolve_one when you only need the raw content for a single hash and
    do NOT need to unpack nested markers."""
    if resolved is None:
        resolved = {}
    if depth >= RECURSIVE_DEPTH or hash_val in resolved:
        return resolved.get(hash_val, "")
    content = _resolve_one(hash_val)
    if content is None:
        return f"<<<CCR:{hash_val}|unresolved>>>"
    resolved[hash_val] = content
    nested = _CCR_RE.findall(content)
    if not nested:
        return content
    replacements = {}
    for marker in nested:
        parts = marker.split("|")
        if len(parts) >= 1 and parts[0] not in resolved:
            nested_hash = parts[0]
            nested_content = _resolve_recursive(nested_hash, depth + 1, resolved)
            replacements[f"<<<CCR:{marker}>>>"] = nested_content
    for marker_str, replacement in replacements.items():
        content = content.replace(marker_str, replacement, 1)
    _inline_store[hash_val] = content
    return content
