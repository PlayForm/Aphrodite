"""aphrodite — retroactive reclassification of CCR entries with structured metadata."""

import contextlib
import json
import logging
import time as _time

from .._core import _inline_store, _recent_markers
from .._marker import _classify_content
from .._resolve import _resolve_one

_log = logging.getLogger("aphrodite.hooks.reclassify")


RECLASSIFY_SCHEMA = {
    "name": "aphrodite_reclassify",
    "description": "Retroactively classify/metadata-enrich all CCR entries lacking "
    "structured metadata. Scans _recent_markers, retrieves content, runs "
    "_classify_content, writes meta field. Safe, non-destructive.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": "Optional: reclassify a single CCR entry by hash. "
                "If omitted (or action='all'), reclassifies all entries lacking meta.",
            },
            "action": {
                "type": "string",
                "description": "Set to 'all' to reclassify all entries lacking meta. "
                "Ignored if hash is provided.",
                "default": "all",
            },
        },
        "required": [],
    },
}


def _aphrodite_reclassify_handler(args=None, **kwargs):
    """Retroactively enrich all CCR entries with structured metadata.

    For each entry in _recent_markers that lacks a non-empty ``meta`` dict,
    retrieve the original content and run ``_classify_content`` on it, then
    write the result to the entry's ``meta`` field.

    Non-destructive: only adds the ``meta`` field, never removes or alters
    existing fields. Skips entries where content cannot be retrieved.

    Returns JSON with classification stats.
    """
    args = args if isinstance(args, dict) else {}
    target_hash = args.get("hash", "").strip()
    action = args.get("action", "all")

    if target_hash:
        candidates = [m for m in _recent_markers if m.get("hash") == target_hash]
        if not candidates:
            return json.dumps({"error": f"hash not found: {target_hash}", "reclassified": 0})
    elif action == "all":
        candidates = [m for m in _recent_markers if not m.get("meta") or m["meta"] == {}]
    else:
        return json.dumps({"error": f"unknown action: {action}", "reclassified": 0})

    if not candidates:
        return json.dumps({
            "reclassified": 0, "type_distribution": {},
            "note": "all entries already have metadata",
        })

    type_counts = {}
    reclassified = 0
    skipped_no_content = 0
    errors = 0
    t0 = _time.time()

    for m in candidates:
        h = m.get("hash", "")
        if not h:
            continue
        if m.get("meta") and m["meta"] != {}:
            continue

        content = None
        h_bare = h[2:] if h.startswith("i:") else h
        if h_bare in _inline_store:
            content = _inline_store[h_bare]
        else:
            with contextlib.suppress(Exception):
                content = _resolve_one(h, timeout=2)

        if content is None:
            skipped_no_content += 1
            continue

        try:
            klass = _classify_content(content)
            if not m.get("meta") or m["meta"] == {}:
                m["meta"] = klass
            ctype = klass.get("type", "text")
            type_counts[ctype] = type_counts.get(ctype, 0) + 1
            reclassified += 1
        except Exception:
            errors += 1
            continue

    elapsed = _time.time() - t0
    total_with_meta = sum(1 for m in _recent_markers if m.get("meta") and m["meta"] != {})

    return json.dumps({
        "reclassified": reclassified,
        "skipped_no_content": skipped_no_content,
        "errors": errors,
        "elapsed_ms": round(elapsed * 1000, 1),
        "total_with_meta": total_with_meta,
        "total_entries": len(_recent_markers),
        "type_distribution": dict(sorted(type_counts.items())),
        "note": f"{reclassified} entries enriched with retroactive metadata",
    }, indent=2)
