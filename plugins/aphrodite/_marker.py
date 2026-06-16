"""aphrodite - marker formatting and proxy compression."""

import base64
import json
import logging
import urllib.request

from ._core import _CCR_RE

_log = logging.getLogger("aphrodite")


def _ccr_marker(hash_val, ccr_type, size, mode="", preview=""):
    """Build a standard CCR marker string."""
    parts = [hash_val, ccr_type, str(size)]
    if mode:
        parts.append(mode)
    if preview:
        preview_b64 = base64.urlsafe_b64encode(preview.encode()).decode()
        parts.append(f"preview={preview_b64}")
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
    """Parse <<<CCR:hash|type|size|mode>>> markers from text. Returns list of dicts."""
    markers = []
    for match in _CCR_RE.finditer(text):
        m = match.group(1)
        parts = m.split("|")
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                marker = {"hash": parts[0], "type": parts[1], "size": sz}
                if len(parts) >= 4:
                    marker["mode"] = parts[3]
                markers.append(marker)
            except (ValueError, IndexError):
                marker_text = match.group(0)[:100]
                _log.warning("Malformed CCR marker ignored: %r", marker_text)
    return markers
