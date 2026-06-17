"""aphrodite — conversation turn diff handler."""

import json
import logging

from .._core import _conv_index

_log = logging.getLogger("aphrodite.hooks.diff")


def _fmt_diff(data: dict) -> str:
    """Format diff JSON into a readable markdown summary."""
    turns = data.get("recent", [])
    total = data.get("turns", 0)

    lines = [f"Turn History: {total} turns"]
    if turns:
        lines.append("")
        for t in turns[:10]:
            tnum = t.get("turn", "?")
            summary = (t.get("summary", "") or "")[:100]
            lines.append(f"T{tnum}: {summary}")
    else:
        lines.append("No turn history yet.")

    return "\n".join(lines)


def _diff_handler(args=None, **kwargs):
    """Show conversation turn diffs — what was discussed in recent turns."""
    if not _conv_index:
        return json.dumps({"turns": 0, "hint": "No turn history yet"})
    turns = []
    for tnum in sorted(_conv_index.keys(), reverse=True)[:10]:
        h, summary, size = _conv_index[tnum]
        turns.append({"turn": tnum, "hash": h, "summary": summary, "size": size})
    return json.dumps({"turns": len(_conv_index), "recent": turns})


DIFF_SCHEMA = {
    "name": "aphrodite_diff",
    "description": "Show conversation turn history — what was discussed, compressed, "
    "and stored across turns. Use to understand context evolution.",
    "parameters": {"type": "object", "properties": {}},
}
