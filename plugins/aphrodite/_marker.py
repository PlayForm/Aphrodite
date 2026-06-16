"""aphrodite - marker formatting and proxy compression."""

import http.client
import json
import logging
import re
import threading
import time

from ._core import _CCR_RE

_log = logging.getLogger("aphrodite")

# Compiled regex for valid CCR hash validation
_VALID_HASH_RE = re.compile(r"^(?:[0-9a-f]{24,}|i:[0-9a-f]{6,})$")
_parse_errors = 0  # count of malformed CCR markers silently skipped

# ── Thread-local connection pool (keep-alive reuse) ──────────
_tls = threading.local()


def _get_conn(port: int) -> http.client.HTTPConnection:
    """Get or create a keep-alive HTTPConnection for the given port (thread-local).

    Connections idle for >60s are evicted and recreated.
    """
    if not hasattr(_tls, "conns"):
        _tls.conns = {}
        _tls.conn_ts = {}
    # Evict idle connections (>60s since last use)
    now = time.time()
    if port in _tls.conns:
        if now - _tls.conn_ts.get(port, 0) > 60:
            try:
                _tls.conns[port].close()
            except Exception:
                pass
            del _tls.conns[port]
            del _tls.conn_ts[port]
    if port not in _tls.conns:
        _tls.conns[port] = http.client.HTTPConnection("127.0.0.1", port, timeout=0.5)
    return _tls.conns[port]


def _put_conn(port: int) -> None:
    """Close and remove a connection (recovery after error)."""
    if hasattr(_tls, "conns") and port in _tls.conns:
        try:
            _tls.conns[port].close()
        except Exception:
            pass
        del _tls.conns[port]
        if hasattr(_tls, "conn_ts") and port in _tls.conn_ts:
            del _tls.conn_ts[port]


def _ccr_marker(hash_val, ccr_type, size, mode="", preview=""):
    """Build a standard CCR marker string.

    Args:
        hash_val: CCR hash string.
        ccr_type: Type label (tool, terminal, etc.).
        size: Original size in bytes.
        mode: Proxy mode (token, cache, inline, etc.).
        preview: Optional text preview (pipe-safe, control-char-stripped).
    """
    parts = [hash_val, ccr_type, str(size)]
    if mode:
        parts.append(mode)
    if preview:
        safe = preview.replace("|", "-").replace("\n", " ").replace("\r", " ").strip()
        safe = "".join(c if c >= " " else " " for c in safe)
        parts.append(f"preview={safe}")
    return f"<<<CCR:{'|'.join(parts)}>>>"


def _compress_via_proxy(content, target_port, headers=None):
    """Compress content through proxy CCR. Returns (hash, compressed_size) or None.

    Uses a thread-local keep-alive HTTP connection pool to avoid TCP handshake
    overhead on repeated calls to the same proxy port.

    Sends raw bytes with ``Content-Type: application/octet-stream`` to skip
    JSON serialization overhead - the proxy reads the request body directly.
    """
    try:
        data = content.encode("utf-8")
        conn = _get_conn(target_port)
        hdrs = {"Content-Type": "application/octet-stream", "Connection": "keep-alive"}
        if headers:
            for k, v in headers.items():
                if k.lower().startswith("x-headroom-"):
                    hdrs[k] = str(v)
        conn.request(
            "POST",
            "/ccr/create",
            body=data,
            headers=hdrs,
        )
        r = conn.getresponse()
        ccr = json.loads(r.read(4096))
        r.close()
        _tls.conn_ts[target_port] = time.time()
        if "error" in ccr:
            _log.debug("_compress_via_proxy: proxy returned error on port %s - %s", target_port, ccr.get("error"))
            return None
        return ccr["hash"], len(content)
    except Exception:
        _put_conn(target_port)  # ditch broken connection, reopen fresh next time
        return None


def _is_valid_ccr_hash(h):
    """Check if h is a valid CCR hash.

    Fast-reject short strings before regex; the regex enforces >=24 hex chars
    for proxy hashes (>=6 hex for i: inline hashes).
    """
    if not h or len(h) < 8:
        return False
    return bool(_VALID_HASH_RE.match(h.lower()))


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|mode|preview=TEXT>>> markers from text. Returns list of dicts with preview."""
    global _parse_errors
    markers = []
    for match in _CCR_RE.finditer(text):
        full = match.group(0)
        # Extract inner content between CCR: and the closing delimiter
        inner = full.split("CCR:", 1)[1]
        for suffix in (">>>", "]", "⫸"):
            if inner.endswith(suffix):
                inner = inner[: -len(suffix)]
                break
        parts = inner.split("|")
        h = parts[0]  # hash from first pipe-delimited field
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                # Parse embedded preview=TEXT from marker parts (parts[3:])
                preview = ""
                for part in parts[3:]:
                    if part.startswith("preview="):
                        preview = part[len("preview="):]
                        break
                markers.append(
                    {
                        "hash": h,  # from parts[0], the single source of truth
                        "type": str(parts[1]),
                        "size": sz,
                        "mode": str(parts[3]) if len(parts) > 3 else "?",
                        "preview": preview,
                    }
                )
            except ValueError:
                _parse_errors += 1
                _log.debug("_parse_ccr_markers: malformed marker skipped in %d-char text", len(text) if isinstance(text, str) else 0)
    # Filter: real CCR hashes are hex (0-9,a-f), >=8 chars,
    # or start with "i:" followed by pure hex.
    return [m for m in markers if _is_valid_ccr_hash(m["hash"])]
