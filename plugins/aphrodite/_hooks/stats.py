"""aphrodite — statistics handler."""

import json
import logging
import urllib.request

from .._core import (
    PORTS,
    _inline_bytes,
    _inline_store,
)
from .._engine import get_engine
from .._marker import _parse_errors

_log = logging.getLogger("aphrodite.hooks.stats")


def _fmt_stats(data: dict) -> str:
    """Format stats JSON into a readable markdown summary."""
    lines = ["Aphrodite Stats", ""]

    proxy = data.get("proxy", {})
    lines.append("proxy:")
    for name in ["token", "cache"]:
        p = proxy.get(name, {})
        if p.get("alive"):
            lines.append(
                f"  {name}: on {p.get('ccr_created', 0)} created "
                f"{p.get('ccr_hits', 0)} hits {p.get('tokens_saved', 0)} tokens saved"
            )
        else:
            lines.append(f"  {name}: off")

    eng = data.get("engine", {})
    lines.append("")
    if eng.get("active"):
        lines.append(
            f"engine: on {eng.get('threshold_tokens', 0)} threshold "
            f"{eng.get('compressions', 0)} compressions "
            f"{eng.get('protect_first_n', 0)}/{eng.get('protect_last_n', 0)} protect"
        )
    else:
        lines.append("engine: off")

    inline = data.get("inline_store", {})
    entries = inline.get("entries", 0)
    total_bytes = inline.get("total_bytes", 0)
    bytes_str = f"{total_bytes / 1024:.1f}KB" if total_bytes >= 1024 else f"{total_bytes}B"
    lines.append(f"inline: {entries} entries {bytes_str}")

    return "\n".join(lines)


def _stats_handler(args=None, **kwargs):
    """Return proxy health, CCR stats, engine status, inline store size."""
    result = {
        "proxy": {},
        "engine": {},
        "inline_store": {
            "entries": len(_inline_store),
            "total_bytes": _inline_bytes,
        },
    }

    for name, port in PORTS.items():
        try:
            r = urllib.request.urlopen(f"http://127.0.0.1:{port}/stats", timeout=2)
            data = json.loads(r.read())
            ccr = data.get("ccr", {})
            result["proxy"][name] = {
                "alive": True,
                "ccr_created": ccr.get("created", 0),
                "ccr_hits": ccr.get("hits", 0),
                "ccr_misses": ccr.get("misses", 0),
                "ccr_entries": ccr.get("entries", "?"),
                "tokens_saved": data.get("tokens_saved", 0),
                "requests_total": data.get("requests", {}).get("total", 0),
                "requests_compressed": data.get("requests", {}).get("compressed", 0),
                "compressions_by_type": data.get("compressions_by_type", {}),
            }
        except Exception:
            result["proxy"][name] = {"alive": False}

    eng = get_engine()
    if eng:
        result["engine"] = {
            "active": True,
            "compressions": eng.compression_count,
            "marker_parse_errors": _parse_errors,
            "threshold_tokens": eng.threshold_tokens,
            "last_prompt_tokens": eng.last_prompt_tokens,
            "context_length": eng.context_length,
            "protect_first_n": eng.protect_first_n,
            "protect_last_n": eng.protect_last_n,
            "last_compression": eng.last_compression,
            "session_id": eng.session_id,
        }
    else:
        result["engine"] = {"active": False}

    return json.dumps(result)


STATS_SCHEMA = {
    "name": "aphrodite_stats",
    "description": "Check aphrodite proxy health, CCR stats, engine compression status. "
    "Use when debugging compression or checking if proxy is alive.",
    "parameters": {"type": "object", "properties": {}},
}
