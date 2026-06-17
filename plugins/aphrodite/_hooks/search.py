"""aphrodite — CCR search handler with trigram indexing."""

import json
import logging

from .._core import (
    _conv_index,
    _init_trigram_index,
    _inline_index,
    _inline_index_enabled,
    _inline_store,
    _recent_markers,
)

_log = logging.getLogger("aphrodite.hooks.search")


def _search_handler(args=None, **kwargs):
    """Search across compressed items by type or content pattern (trigram-indexed)."""
    args = args if isinstance(args, dict) else {}
    query = args.get("query", "").lower()
    ccr_type = args.get("type", "")

    if query and len(query) < 3:
        return json.dumps({
            "query": query,
            "type_filter": ccr_type,
            "matches": 0,
            "error": "query too short - minimum 3 characters required",
            "results": [],
        })

    if not _inline_index_enabled and _inline_store:
        _init_trigram_index()

    results = []

    # Search conversation turn index
    for tnum, (h, summary, size) in sorted(_conv_index.items(), reverse=True):
        if query and query not in summary.lower():
            continue
        results.append({"source": "turn", "turn": tnum, "hash": h, "summary": summary, "size": size})

    # Search inline store via trigram index
    if query:
        trigrams = {query[i:i + 3] for i in range(len(query) - 2)}
        candidates = set()
        if trigrams and _inline_index:
            for tri in trigrams:
                candidates |= _inline_index.get(tri, set())
        if not candidates and not _inline_index:
            candidates = set(_inline_store.keys())
        for h in candidates:
            content = _inline_store.get(h)
            if content is None:
                continue
            if query not in content.lower():
                continue
            preview = content[:200].replace("\n", " ").strip()
            results.append({"source": "inline", "hash": h, "preview": preview, "size": len(content)})
    else:
        for h, content in list(_inline_store.items()):
            preview = content[:200].replace("\n", " ").strip()
            results.append({"source": "inline", "hash": h, "preview": preview, "size": len(content)})

    # Search recent marker catalog
    for m in _recent_markers:
        if query and query not in m.get("preview", "").lower():
            continue
        results.append({
            "source": "marker",
            "hash": m["hash"],
            "type": m.get("type", "?"),
            "size": m.get("size", 0),
            "preview": m.get("preview", "")[:200],
        })

    # Deduplicate by hash
    seen = set()
    unique = []
    for r in results:
        h = r.get("hash", "")
        if h and h not in seen:
            seen.add(h)
            unique.append(r)
    results = unique

    if ccr_type:
        results = [
            r for r in results
            if ccr_type in r.get("type", "") or ccr_type in r.get("summary", "") + r.get("preview", "")
        ]

    return json.dumps({
        "query": query,
        "type_filter": ccr_type,
        "matches": len(results),
        "hint": "Use aphrodite_retrieve(hash) to expand any result hash.",
        "results": results[:20],
    })


SEARCH_SCHEMA = {
    "name": "aphrodite_search",
    "description": "Search across CCR entries — find compressed content by keyword or type. "
    "Use to locate previously compressed context without knowing the hash.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search keyword or phrase to find in compressed content",
            },
            "type": {
                "type": "string",
                "description": "Optional: filter by CCR type (tool, terminal, code, error, etc.)",
            },
        },
        "required": ["query"],
    },
}
