"""CCR marker parsing — extract hash/type/size/meta from text."""

import logging

from .._core import _CCR_RE
from .marker import _is_valid_ccr_hash

_log = logging.getLogger("aphrodite")
_parse_errors = 0  # count of malformed CCR markers silently skipped


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|key=value|...>>> markers from text.

    Returns list of dicts each containing:
        hash, type, size, mode, preview, meta

    ``meta`` is a dict with all key=value pairs found past parts[3],
    including ``preview`` (which is stored both in ``preview`` and
    ``meta["preview"]`` for backward compat).
    """
    global _parse_errors
    markers = []
    for match in _CCR_RE.finditer(text):
        full = match.group(0)
        inner = full.split("CCR:", 1)[1]
        for suffix in (">>>", "]", "⫸"):
            if inner.endswith(suffix):
                inner = inner[: -len(suffix)]
                break
        parts = inner.split("|")
        h = parts[0]
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                mode = "?"
                meta = {}
                for part in parts[3:]:
                    if "=" in part:
                        k, _, v = part.partition("=")
                        meta[str(k)] = str(v)
                    elif mode == "?":
                        mode = str(part)
                    else:
                        pass
                preview = meta.pop("preview", "")
                parsed_meta = meta if meta else {}
                markers.append(
                    {
                        "hash": h,
                        "type": str(parts[1]),
                        "size": sz,
                        "mode": mode,
                        "preview": preview,
                        "meta": parsed_meta,
                    }
                )
            except ValueError:
                _parse_errors += 1
                _log.debug("_parse_ccr_markers: malformed marker skipped in %d-char text", len(text) if isinstance(text, str) else 0)
    return [m for m in markers if _is_valid_ccr_hash(m["hash"])]
