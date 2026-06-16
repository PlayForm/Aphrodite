"""aphrodite - marker formatting and proxy compression."""

import base64
import json
import logging
import urllib.request

from ._core import _CCR_RE

_log = logging.getLogger("aphrodite")


def _ccr_marker(hash_val, ccr_type, size, mode="", preview="", headers=None):
    """Build a standard CCR marker string.

    Args:
        hash_val: CCR hash string.
        ccr_type: Type label (tool, terminal, etc.).
        size: Original size in bytes.
        mode: Proxy mode (token, cache, inline, etc.).
        preview: Optional text preview (base64-encoded).
        headers: Optional dict of extra key=value pairs embedded in the marker.
    """
    parts = [hash_val, ccr_type, str(size)]
    if mode:
        parts.append(mode)
    if preview:
        preview_b64 = base64.urlsafe_b64encode(preview.encode()).decode()
        parts.append(f"preview={preview_b64}")
    if headers:
        for k, v in headers.items():
            parts.append(f"{k}={v}")
    return f"<<<CCR:{'|'.join(parts)}>>>"


def _compress_via_proxy(content, target_port):
    """Compress content through proxy CCR. Returns (hash, compressed_size) or None."""
    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target_port}/ccr/create", data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        return ccr["hash"], len(content)
    except Exception:
        return None


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|mode>>> markers from text. Returns list of dicts with preview."""
    markers = []
    for match in _CCR_RE.finditer(text):
        m = match.group(1)
        parts = m.split("|")
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                # Extract preview text after the >>> terminator
                marker_end = match.end()  # position right after >>>
                preview = text[marker_end:].strip()[:200] if marker_end < len(text) else ""
                markers.append(
                    {
                        "hash": str(parts[0]) if parts[0] else "",
                        "type": str(parts[1]),
                        "size": sz,
                        "mode": str(parts[3]) if len(parts) > 3 else "?",
                        "preview": preview,
                    }
                )
            except ValueError:
                _log.debug("_parse_ccr_markers: malformed marker skipped in %d-char text", len(text) if isinstance(text, str) else 0)
    # Filter: real CCR hashes are hex (0-9,a-f), >=8 chars
    return [
        m
        for m in markers
        if m["hash"] and len(m["hash"]) >= 8 and all(c in "0123456789abcdef" for c in m["hash"].lower())
    ]
