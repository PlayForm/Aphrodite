"""aphrodite — tool output formatting for readable LLM consumption."""

import hashlib
import json
import logging
import os
import re
import time

from .._core import (
    _CCR_RE,
    _DEV,
    DEBUG_LOGGING,
    INLINE_THRESHOLD,
    MAX_REQUEST_BODY_SIZE,
    PORTS,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _detect_model_family,
    _hash_alias,
    _inline_store_put,
    _recent_markers,
    _state,
)
from .._inline import _inline_compress
from .._marker import (
    _ccr_marker,
    _classify_content,
    _compress_via_proxy,
    _make_ccr_preview,
)
from .._proxy import _alive_cached, _headroom_context
from .catalog import _fmt_catalog
from .classify import _classifier_says_skip
from .diff import _fmt_diff
from .files import _fmt_files, _track_file_refs
from .stats import _fmt_stats

_log = logging.getLogger("aphrodite.hooks.transform")

_ESSENTIAL_TOOLS: frozenset = frozenset({
    "aphrodite_catalog", "aphrodite_compress", "aphrodite_diff",
    "aphrodite_files", "aphrodite_rebuild", "aphrodite_reclassify",
    "aphrodite_retrieve", "aphrodite_search", "aphrodite_stats",
    "aphrodite_test",
})


def _format_aphrodite_output(tool_name: str, result: str) -> str:
    """Format aphrodite tool JSON output into rich markdown."""
    try:
        data = json.loads(result)
    except (json.JSONDecodeError, TypeError):
        return result

    if tool_name == "aphrodite_catalog":
        return _fmt_catalog(data)
    elif tool_name == "aphrodite_stats":
        return _fmt_stats(data)
    elif tool_name == "aphrodite_diff":
        return _fmt_diff(data)
    elif tool_name == "aphrodite_files":
        return _fmt_files(data)

    return result


def _extract_tool_metadata(tool_name, args, result):
    """Extract structured metadata dict from tool_name, args, and result.

    Returns a dict suitable for ``_ccr_marker(meta=...)``, or None
    if no metadata can be extracted.  Soft-fails on any exception.
    """
    try:
        args = args if isinstance(args, dict) else {}
        if tool_name == "read_file":
            path = args.get("path", args.get("file", ""))
            if path and isinstance(path, str):
                fn = os.path.basename(path)
                _, ext = os.path.splitext(fn)
                meta = {"fn": fn}
                if ext:
                    meta["ext"] = ext.lstrip(".")
                lines = result.splitlines()
                meta["lines"] = str(len(lines))
                names = []
                for line in lines:
                    m = re.match(r"^\s*(?:class|struct|enum|trait|fn)\s+(\w+)", line)
                    if m:
                        names.append(m.group(1))
                if names:
                    meta["names"] = ",".join(names[:5])
                return meta

        elif tool_name == "search_files":
            pattern = args.get("pattern", "")
            if pattern:
                meta = {"q": str(pattern)[:40]}
                try:
                    data = json.loads(result)
                    if isinstance(data, list):
                        meta["files"] = str(len(data))
                    elif isinstance(data, dict):
                        for key in ("matches", "files", "results"):
                            val = data.get(key)
                            if isinstance(val, list):
                                meta["files"] = str(len(val))
                                break
                        else:
                            meta["files"] = str(data.get("total_count", data.get("count", "?")))
                except (json.JSONDecodeError, ValueError):
                    line_count = 0
                    for line in result.splitlines():
                        line = line.strip()
                        if line and not line.startswith((">", "<", "-", "+")):
                            line_count += 1
                    if line_count:
                        meta["files"] = str(line_count)
                return meta

        elif tool_name == "terminal":
            exit_code = args.get("exit_code", args.get("returncode", ""))
            if not exit_code:
                for line in result.splitlines():
                    m = re.match(r"exit code[:\s]+(\d+)", line.strip(), re.IGNORECASE)
                    if m:
                        exit_code = m.group(1)
                        break
            meta = {}
            if exit_code:
                meta["exit"] = str(exit_code)
            last_line = ""
            for line in result.splitlines():
                stripped = line.strip()
                if stripped:
                    last_line = stripped
            if last_line:
                meta["last"] = last_line[:60]
            return meta if meta else None

        return None
    except Exception:
        if DEBUG_LOGGING:
            _log.debug("_extract_tool_metadata: failed for %s", tool_name[:40])
        return None


def _transform_tool_result(
    tool_name="",
    args=None,
    result="",
    tool_call_id="",
    task_id="",
    session_id="",
    turn_id="",
    api_request_id="",
    duration_ms=0,
    status="",
    error_type="",
    error_message="",
    **kwargs,
):
    """Compress tool outputs via CCR. Proxy first, inline fallback when proxy down."""
    _t0 = time.time()
    if not result or not isinstance(result, str) or not result.strip():
        return result

    _track_file_refs(tool_name, args)
    if _DEV:
        return result
    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    marker_type = "aphrodite" if tool_name.startswith("aphrodite_") else "tool"
    if tool_name in _ESSENTIAL_TOOLS:
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: SKIP %s %.1fms (in skip list)",
                tool_name[:40], (time.time() - _t0) * 1000,
            )
        return _format_aphrodite_output(tool_name, result)

    threshold = TOOL_THRESHOLD_TOKEN if token_alive else TOOL_THRESHOLD_CACHE if cache_alive else INLINE_THRESHOLD
    result_len = len(result)
    if result_len > MAX_REQUEST_BODY_SIZE:
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: SKIP %s size=%s > MAX_REQUEST_BODY_SIZE=%s",
                tool_name[:40], result_len, MAX_REQUEST_BODY_SIZE,
            )
        return result
    if result_len < threshold:
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: BELOW %s size=%s < threshold=%s %.1fms",
                tool_name[:40], result_len, threshold, (time.time() - _t0) * 1000,
            )
        return result

    if _CCR_RE.search(result):
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: GUARD %s has existing CCR marker %.1fms",
                tool_name[:40], (time.time() - _t0) * 1000,
            )
        return result

    klass = _classify_content(result)
    if _classifier_says_skip(klass):
        if proxy_available:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            _compress_via_proxy(result, target, headers=_headroom_context or None)
        return _make_ccr_preview(result, klass=klass, model_family=_detect_model_family())

    preview = _make_ccr_preview(result, klass=klass, model_family=_detect_model_family())
    metadata = _extract_tool_metadata(tool_name, args, result)

    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(result, target, headers=_headroom_context or None)
        if ccr:
            h, sz = ccr
            _inline_store_put(h, result)
            full_sha = hashlib.sha256(result.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            label = "token" if token_alive else "cache"
            if DEBUG_LOGGING:
                ratio = result_len / max(len(h), 1)
                _log.debug(
                    "transform_tool_result: CCR %s %s:%s size=%s ratio=%.1fx %.1fms",
                    tool_name[:40], label, h, result_len, ratio, (time.time() - _t0) * 1000,
                )
            _recent_markers.append(
                {"hash": h, "type": marker_type, "size": result_len,
                 "preview": preview, "turn": _state["turn_counter"],
                 "meta": metadata or {}}
            )
            _inline_store_put(h, result)
            return _ccr_marker(
                h, marker_type, result_len, label, preview,
                headroom_budget=_headroom_context.get("x-headroom-budget"),
                meta=metadata,
            )
        elif DEBUG_LOGGING:
            _log.debug("transform_tool_result: PROXY FAIL %s - proxy returned no hash", tool_name[:40])

    if result_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(result)
            full_sha = hashlib.sha256(result.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                _log.debug(
                    "transform_tool_result: INLINE %s hash=%s size=%s %.1fms",
                    tool_name[:40], h, result_len, (time.time() - _t0) * 1000,
                )
            _recent_markers.append(
                {"hash": h, "type": marker_type, "size": result_len,
                 "preview": preview, "turn": _state["turn_counter"],
                 "meta": metadata or {}}
            )
            return _ccr_marker(
                h, marker_type, result_len, "inline", preview,
                headroom_budget=_headroom_context.get("x-headroom-budget"),
                meta=metadata,
            )
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("transform_tool_result: INLINE FAIL %s", tool_name[:40])
    if DEBUG_LOGGING:
        _log.debug(
            "transform_tool_result: PASSTHROUGH %s size=%s %.1fms",
            tool_name[:40], result_len, (time.time() - _t0) * 1000,
        )
    return result
