"""aphrodite — terminal output compression via CCR."""

import hashlib
import logging
import time

from .._core import (
    _CCR_RE,
    _DEV,
    DEBUG_LOGGING,
    INLINE_THRESHOLD,
    PORTS,
    TERMINAL_THRESHOLD,
    _detect_model_family,
    _hash_alias,
    _inline_store_put,
    _recent_markers,
)
from .._inline import _inline_compress
from .._marker import (
    _classify_content,
    _compress_via_proxy,
    _make_ccr_preview,
)
from .._proxy import _alive_cached, _headroom_context
from .classify import _classifier_says_skip

_log = logging.getLogger("aphrodite.hooks.terminal")


def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
    """Compress terminal output via CCR on-the-fly. Proxy first, inline fallback.
    Build output gets smart summarization — repeated patterns collapsed."""
    _t0 = time.time()
    if _DEV:
        return output
    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    out_len = len(output)
    orig_len = out_len
    if out_len < TERMINAL_THRESHOLD:
        if DEBUG_LOGGING:
            _log.debug(
                "terminal_hook: BELOW size=%s < threshold=%s %.1fms (cmd: %s)",
                out_len, TERMINAL_THRESHOLD, (time.time() - _t0) * 1000, command[:60],
            )
        return output

    if _CCR_RE.search(output):
        if DEBUG_LOGGING:
            _log.debug(
                "terminal_hook: GUARD has existing CCR marker %.1fms (cmd: %s)",
                (time.time() - _t0) * 1000, command[:60],
            )
        return output

    # Build output detection: collapse repeated lines
    first_line = output.split("\n", 1)[0].strip() if output else ""
    is_build = any(
        first_line.startswith(p)
        for p in (
            "Compiling ", "   Compiling ", "Finished ", "error:",
            "warning:", "Running ", "PASSED", "FAILED", "test result:",
        )
    )
    if is_build:
        lines = output.splitlines()
        if len(lines) <= 20:
            pass
        else:
            unique = []
            counts = {}
            prev = None
            for line in lines:
                stripped = line.strip()
                if stripped == prev:
                    counts[stripped] = counts.get(stripped, 1) + 1
                else:
                    if stripped not in counts:
                        unique.append(stripped)
                    counts[stripped] = counts.get(stripped, 0) + 1
                prev = stripped

            errors = [l for l in unique if "error" in l.lower() and l not in ("error:", "error")]
            warnings = [l for l in unique if "warning" in l.lower() and "warning:" not in l]
            summary = f"[build: {len(lines)} lines, {len(unique)} unique patterns]"
            if errors:
                summary += f" | errors: {'; '.join(errors[:5])}"
            if warnings:
                summary += f" | warnings: {'; '.join(warnings[:3])}"

            if not errors and not warnings:
                if DEBUG_LOGGING:
                    _log.debug("terminal_hook: clean build — inline summary, no CCR")
                return summary
            out_len = len(summary)
            if DEBUG_LOGGING:
                _log.debug(
                    "terminal_hook: BUILD collapse %d→%d lines (cmd: %s)",
                    len(lines), len(summary.split("\n")), command[:60],
                )
            if proxy_available:
                target = PORTS["token"] if token_alive else PORTS["cache"]
                ccr = _compress_via_proxy(output, target, headers=_headroom_context or None)
                if ccr:
                    h, _ = ccr
                    _inline_store_put(h, output)
                    full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
                    _hash_alias[full_sha] = h
                    if DEBUG_LOGGING:
                        _log.debug("terminal_hook: BUILD-CCR %s:%s", "token" if token_alive else "cache", h)
                    _recent_markers.append({"hash": h, "type": "build", "size": len(output), "preview": summary})
                    return f"<<<CCR:{h}|build|{len(output)}>>> {summary}"
            h, _ = _inline_compress(output)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            _recent_markers.append({"hash": h, "type": "build", "size": len(output), "preview": summary})
            return f"<<<CCR:{h}|build|{len(output)}>>> {summary}…(use aphrodite_retrieve)"

    # Classifier poll: clean terminal outputs skip CCR
    klass = _classify_content(output)
    if _classifier_says_skip(klass):
        return _make_ccr_preview(output, klass=klass, model_family=_detect_model_family())

    preview = _make_ccr_preview(output, model_family=_detect_model_family())

    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(output, target)
        if ccr:
            h, _ = ccr
            _inline_store_put(h, output)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                ratio = out_len / max(len(h), 1)
                _log.debug(
                    "terminal_hook: CCR %s:%s size=%s ratio=%.1fx",
                    "token" if token_alive else "cache", h, orig_len, ratio,
                )
            _recent_markers.append({"hash": h, "type": "terminal", "size": orig_len, "preview": preview})
            return f"<<<CCR:{h}|terminal|{orig_len}>>> {preview}"
        elif DEBUG_LOGGING:
            _log.debug("terminal_hook: PROXY FAIL - returned no hash (cmd: %s)", command[:60])

    if orig_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(output)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE hash=%s size=%s", h, orig_len)
            _recent_markers.append({"hash": h, "type": "terminal", "size": orig_len, "preview": preview})
            return f"<<<CCR:{h}|terminal|{orig_len}>>> {preview}"
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE FAIL (cmd: %s)", command[:60])
    if DEBUG_LOGGING:
        _log.debug("terminal_hook: PASSTHROUGH size=%s %.1fms", out_len, (time.time() - _t0) * 1000)
    return output
