"""aphrodite - proxy health checks and version/headroom queries."""

import http.client
import json
import logging
import time

from .._core import DEBUG_LOGGING

_log = logging.getLogger("aphrodite")

# ── Alive cache (5-second TTL) ──────────────────────────────
_alive_cache: dict[int, tuple[bool, float]] = {}  # {port: (result, timestamp)}

# ── Turn-scoped alive cache (refreshed by pre_llm_hook each turn) ──
_alive_turn_cache: dict[int, bool] = {}  # {port: bool}

# ── Proxy version (queried from /health after launch/rebuild) ──
_proxy_version: str = ""  # "v0.5.115" from binary


def _query_proxy_version(port: int, timeout: float = 2.0) -> str:
    """Query proxy /health for binary version. Caches in _proxy_version."""
    global _proxy_version
    try:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
        conn.request("GET", "/health", headers={"Connection": "close"})
        resp = conn.getresponse()
        body = resp.read().decode().strip()
        conn.close()
        if body:
            data = json.loads(body)
            ver = data.get("version", "")
            if ver:
                _proxy_version = ver
                return ver
    except Exception as exc:
        _log.debug("proxy version query failed on :%d: %s", port, exc)
    return _proxy_version


# ── Headroom session context (tracked at session start, refreshed per-turn) ──
_headroom_context: dict[str, str] = {}  # {"x-headroom-budget": "...", "x-headroom-fill": "..."}


def _update_headroom_context(headers: dict | None) -> None:
    """Update _headroom_context from an LLM-call headers dict.

    Called each turn by ``_pre_llm_hook`` so compression calls in
    ``_transform_tool_result`` and ``_transform_terminal_hook`` inherit
    the same session context.  Only ``x-headroom-*`` keys are kept;
    ``x-headroom-bypass`` is explicitly excluded.
    """
    global _headroom_context
    if not headers:
        return
    fresh = {}
    for k, v in headers.items():
        kl = k.lower()
        if kl.startswith("x-headroom-") and kl != "x-headroom-bypass":
            fresh[k] = str(v)
    if fresh:
        _headroom_context.update(fresh)


def _query_and_set_headroom_budget(port: int, timeout: float = 2.0) -> None:
    """Query proxy /health for fill_pct and set x-headroom-budget in _headroom_context.

    Lower fill_pct → more headroom → higher budget (less aggressive compression).
    Higher fill_pct → less headroom → lower budget (more aggressive compression).
    Budget is clamped to [5, 99].
    """
    global _headroom_context
    try:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
        conn.request("GET", "/health", headers={"Connection": "close"})
        resp = conn.getresponse()
        body = resp.read().decode().strip()
        conn.close()
        if body:
            data = json.loads(body)
            fill_pct = data.get("fill_pct")
            if fill_pct is not None:
                fill_pct = float(fill_pct)
                # budget = 100 - fill_pct, clamped [5, 99]
                budget = max(5, min(99, int(100.0 - fill_pct)))
                _headroom_context["x-headroom-budget"] = str(budget)
                if DEBUG_LOGGING:
                    _log.debug(
                        "headroom: fill_pct=%.1f%% budget=%d (from %s)",
                        fill_pct, budget, port,
                    )
    except Exception as exc:
        _log.debug("headroom query failed on :%d: %s", port, exc)


def _alive(port: int, timeout: int = 3) -> bool:
    """Check proxy health with 5-second caching to avoid socket overhead."""
    now = time.time()
    if port in _alive_cache:
        result, ts = _alive_cache[port]
        if now - ts < 5:
            return result
    try:
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
        conn.request("GET", "/health", headers={"Connection": "close"})
        resp = conn.getresponse()
        body = resp.read().decode().strip()
        conn.close()
        if not body:
            result = True
        else:
            try:
                data = json.loads(body)
                result = data.get("status") in ("healthy", "ok", "degraded")
            except Exception:
                result = body.strip() == "ok"
    except Exception:
        result = False
    _alive_cache[port] = (result, now)
    return result


def _alive_cached(port: int) -> bool:
    """Check turn-scoped cache first, fall through to _alive()."""
    if port in _alive_turn_cache:
        return _alive_turn_cache[port]
    return _alive(port)
