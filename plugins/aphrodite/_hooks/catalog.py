"""aphrodite — compression catalog handler and table-of-contents builder."""

import json
import logging

from .._core import _conv_index, _fmt_size, _recent_markers, _referenced_files

_log = logging.getLogger("aphrodite.hooks.catalog")


def _fmt_catalog(data: dict) -> str:
    """Format catalog JSON into a readable markdown table."""
    items = data.get("items", [])
    total_saved = data.get("total_saved", 0)
    conv_turns = data.get("conv_turns", 0)
    ref_files = data.get("referenced_files", 0)

    saved_str = f"{total_saved / 1024:.1f}KB" if total_saved >= 1024 else f"{total_saved}B"

    lines = [
        f"Catalog: {len(items)} items {saved_str} saved {conv_turns} turns {ref_files} files"
    ]

    if items:
        by_type = data.get("by_type", {})
        if by_type:
            type_summary = " ".join(
                f"{t}({v['count']})" for t, v in sorted(by_type.items())
            )
            lines.append(f"Types: {type_summary}")

        lines.append("")
        lines.append("| Hash | Type | Size | Preview |")
        lines.append("|------|------|------|---------|")
        for item in items:
            h = item.get("hash", "")[:10]
            t = item.get("type", "")
            s = item.get("size", 0)
            sz = f"{s / 1024:.0f}KB" if s >= 1024 else f"{s}B"
            p = (item.get("preview", "") or "")[:80].replace("|", "\\|")
            lines.append(f"| {h} | {t} | {sz} | {p} |")
    else:
        lines.append("No compressed items yet.")

    return "\n".join(lines)


def _catalog_handler(args=None, **kwargs):
    """Return compression catalog. Mode 'toc' returns compact table-of-contents."""
    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "full")

    if mode == "toc":
        return _build_toc()

    items = []
    for m in _recent_markers:
        items.append({
            "hash": m["hash"], "type": m["type"], "size": m["size"],
            "preview": m.get("preview", "")[:120],
        })
    by_type = {}
    for item in items:
        by_type.setdefault(item["type"], []).append(item["hash"])
    result = {
        "total_items": len(items),
        "total_saved": sum(m["size"] for m in _recent_markers),
        "by_type": {
            t: {"count": len(hashes), "hashes": hashes[:10]}
            for t, hashes in sorted(by_type.items())
        },
        "items": items,
        "conv_turns": len(_conv_index),
        "referenced_files": len(_referenced_files),
    }
    return json.dumps(result, indent=2)


CATALOG_SCHEMA = {
    "name": "aphrodite_catalog",
    "description": "Return full compression catalog with hashes, sizes, types, previews. "
    "Mode 'toc' for compact table-of-contents with Retrieve? recommendations. "
    "Use toc BEFORE retrieving to avoid wasted round-trips.",
    "parameters": {
        "type": "object",
        "properties": {
            "mode": {
                "type": "string",
                "description": "Optional: 'toc' for compact table-of-contents, default full catalog",
            }
        },
    },
}


def _build_toc() -> str:
    """Build a compact table-of-contents for the agent to quickly scan before retrieving.

    Shows every CCR entry with hash, type, size, preview, and a 'Retrieve?'
    recommendation (NO if the preview tells the full story, YES if retrieval
    would add useful content).
    """
    markers = list(_recent_markers)
    if not markers:
        return "Catalog: 0 items"

    lines = [
        f"Catalog: {len(markers)} items, {sum(m['size'] for m in markers)}B saved",
        "",
        "| Hash    | Type           | Size  | Preview                          | Retrieve? |",
        "|---------|----------------|-------|----------------------------------|-----------|",
    ]

    for m in reversed(markers[-20:]):
        h = m["hash"][:12]
        t = m["type"][:14]
        s = _fmt_size(m["size"])
        p = (m.get("preview", "") or "")[:45].replace("|", "/")
        retrieve = "NO"
        if t in ("build_output", "build_error"):
            pl = p.lower()
            if "0e" not in pl and "0w" not in pl:
                retrieve = "YES"
        elif (
            t == "terminal" and "exit=0" not in p
            or t in ("grep", "search_files", "search_results") and "0 matches" not in p and "0m" not in p
            or t not in ("build_output", "build_error", "terminal") and "0E 0W" not in p
        ):
            retrieve = "YES"

        lines.append(f"| {h:<7} | {t:<14} | {s:>5} | {p:<45} | {retrieve:<9} |")

    lines.extend(["", "Retrieve? = NO means the preview is sufficient — skip retrieval."])
    return "\n".join(lines)
