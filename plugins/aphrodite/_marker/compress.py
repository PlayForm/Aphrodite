"""Proxy compression — keep-alive HTTP connection pool."""

import contextlib
import http.client
import json
import logging
import threading
import time

_log = logging.getLogger("aphrodite")

# ── Thread-local connection pool (keep-alive reuse) ──────────
_tls = threading.local()


def _get_conn(port: int) -> http.client.HTTPConnection:
    """Get or create a keep-alive HTTPConnection for the given port (thread-local).

    Connections idle for >60s are evicted and recreated.
    """
    if not hasattr(_tls, "conns"):
        _tls.conns = {}
        _tls.conn_ts = {}
    now = time.time()
    if port in _tls.conns and now - _tls.conn_ts.get(port, 0) > 60:
        with contextlib.suppress(Exception):
            _tls.conns[port].close()
        del _tls.conns[port]
        del _tls.conn_ts[port]
    if port not in _tls.conns:
        _tls.conns[port] = http.client.HTTPConnection("127.0.0.1", port, timeout=0.5)
    return _tls.conns[port]


def _put_conn(port: int) -> None:
    """Close and remove a connection (recovery after error)."""
    if hasattr(_tls, "conns") and port in _tls.conns:
        with contextlib.suppress(Exception):
            _tls.conns[port].close()
        del _tls.conns[port]
        if hasattr(_tls, "conn_ts") and port in _tls.conn_ts:
            del _tls.conn_ts[port]


def _compress_via_proxy(content, target_port, headers=None):
    """Compress content through proxy CCR. Returns (hash, compressed_size) or None.

    Uses a thread-local keep-alive HTTP connection pool to avoid TCP handshake
    overhead on repeated calls to the same proxy port.

    Sends raw bytes with Content-Type: application/octet-stream to skip
    JSON serialization overhead — the proxy reads the request body directly.
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
        _put_conn(target_port)
        return None
