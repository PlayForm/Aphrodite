"""aphrodite - hook handlers for Hermes tool/terminal/LLM calls."""

import hashlib
import json
import logging
import os
import re
import subprocess
import time
import urllib.request

from ._core import (
    _CCR_RE,
    _DEV,
    _FILE_TOOLS,
    AUTO_EXPAND_LIMIT,
    BINARY,
    CATALOG_MODE,
    DEBUG_LOGGING,
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    INLINE_THRESHOLD,
    MAX_REQUEST_BODY_SIZE,
    PLUGIN_VERSION,
    PORTS,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _conv_index,
    _fmt_size,
    _git_cache,
    _hash_alias,
    _increment_turn,
    _init_trigram_index,
    _inline_bytes,
    _inline_index,
    _inline_bytes,
    _inline_store,
    _inline_store_put,
    _recent_markers,
    _reset_scanned_msg_idx,
    _scanned_msg_idx,
    _state,
    _detect_model_family,
    _render_prompt_tmpl,
    CLASSIFIER_POLL,
    MODEL_FAMILY,
    CATALOG_INTENT_HINTS,
    CONTEXT_ENGINE,
)
from ._engine import get_engine
from ._inline import _inline_compress, _inline_retrieve
from ._marker import _ccr_marker, _classify_content, _compress_via_proxy, _make_ccr_preview, _parse_ccr_markers, _parse_errors
from ._proxy import (
    _alive,
    _alive_cache,
    _alive_cached,
    _alive_turn_cache,
    _expand_guidance,
    _headroom_context,
    _query_and_set_headroom_budget,
    _update_headroom_context,
)
from ._resolve import _resolve_one
from ._tools import _compress_handler, _retrieve_handler

_last_user_msg = ""  # cached by pre_llm_hook for store_conversation_turn
_catalog_injected_this_turn: bool = False  # guard vs double-inject on LLM retry
_session_instruction_injected: bool = False  # guard: only inject session intro once


# These are defined within this module (extracted from original monolithic file)
# _track_file_refs, _fmt_size, _extract_preview, _group_into_turns are defined below
# _referenced_files, _recent_markers, _conv_index, _FILE_TOOLS, _git_cache are module-level

_log = logging.getLogger("aphrodite")

__all__ = [
    "_transform_tool_result", "_transform_terminal_hook",
    "_pre_llm_hook", "_store_conversation_turn",
    "_rebuild_handler", "_stats_handler", "_files_handler",
    "_diff_handler", "_search_handler", "_test_handler", "_catalog_handler",
    "_aphrodite_reclassify_handler",
    "REBUILD_SCHEMA", "STATS_SCHEMA", "FILES_SCHEMA", "DIFF_SCHEMA",
    "SEARCH_SCHEMA", "TEST_SCHEMA", "CATALOG_SCHEMA", "RECLASSIFY_SCHEMA",
]

# ── Module-level frozenset constants ───────────────────────────
# Only aphrodite_* tools are protected from double-compression
# (they already compress at the proxy level via tool_relay).
# Everything else — read_file, skill_view, session_search, etc. —
# flows through the normal compression pipeline.
_ESSENTIAL_TOOLS: frozenset = frozenset({
    "aphrodite_catalog", "aphrodite_compress", "aphrodite_diff",
    "aphrodite_files", "aphrodite_rebuild", "aphrodite_reclassify",
    "aphrodite_retrieve", "aphrodite_search", "aphrodite_stats",
    "aphrodite_test",
})
_READ_KEYWORDS: frozenset = frozenset({
    "read", "show", "view", "get", "cat", "display", "retrieve",
    "fetch", "look", "see", "open", "inspect", "check", "print",
    "dump", "output",
})

# ── Tool output formatting ─────────────────────────────────────
# Transforms raw JSON from aphrodite meta-tools into readable
# markdown tables for the LLM, preserving all data.


def _format_aphrodite_output(tool_name: str, result: str) -> str:
    """Format aphrodite tool JSON output into rich markdown.

    Called from _transform_tool_result for tools in _ESSENTIAL_TOOLS.
    Returns formatted string, or original result if unparseable/unknown.
    """
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


def _fmt_catalog(data: dict) -> str:
    items = data.get("items", [])
    total_saved = data.get("total_saved", 0)
    conv_turns = data.get("conv_turns", 0)
    ref_files = data.get("referenced_files", 0)

    if total_saved >= 1024:
        saved_str = f"{total_saved / 1024:.1f}KB"
    else:
        saved_str = f"{total_saved}B"

    lines = [
        f"Catalog: {len(items)} items {saved_str} saved {conv_turns} turns {ref_files} files"
    ]

    if items:
        by_type = data.get("by_type", {})
        if by_type:
            type_summary = " ".join(
                f"{t}({v['count']})" for t, v in sorted(by_type.items())
            )
            lines.append(f"Types: {type_summary}")

        lines.append("")
        lines.append("| Hash | Type | Size | Preview |")
        lines.append("|------|------|------|---------|")
        for item in items:
            h = item.get("hash", "")[:10]
            t = item.get("type", "")
            s = item.get("size", 0)
            sz = f"{s / 1024:.0f}KB" if s >= 1024 else f"{s}B"
            p = (item.get("preview", "") or "")[:80].replace("|", "\\|")
            lines.append(f"| {h} | {t} | {sz} | {p} |")
    else:
        lines.append("No compressed items yet.")

    return "\n".join(lines)


def _fmt_stats(data: dict) -> str:
    lines = ["Aphrodite Stats", ""]

    # Proxy health
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

    # Engine
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

    # Inline store
    inline = data.get("inline_store", {})
    entries = inline.get("entries", 0)
    total_bytes = inline.get("total_bytes", 0)
    if total_bytes >= 1024:
        bytes_str = f"{total_bytes / 1024:.1f}KB"
    else:
        bytes_str = f"{total_bytes}B"
    lines.append(f"inline: {entries} entries {bytes_str}")

    return "\n".join(lines)


def _fmt_diff(data: dict) -> str:
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


def _fmt_files(data: dict) -> str:
    files = data.get("files", [])
    count = data.get("count", len(files) if isinstance(files, list) else 0)

    if count == 0:
        return "Referenced Files: 0 files"

    lines = [f"Referenced Files: {count} files", ""]

    if isinstance(files, dict):
        for tool, paths in files.items():
            lines.append(f"{tool}:")
            for p in paths[:20]:
                lines.append(f"  {p}")
    elif isinstance(files, list):
        for f in files[:30]:
            if isinstance(f, dict):
                lines.append(f"  {f.get('path', '')} ({f.get('tool', '')})")
            else:
                lines.append(f"  {f}")

    return "\n".join(lines)


# ── Hooks ─────────────────────────────────────────────────────

# Layer 1: inject session-start instruction on first _pre_llm_hook call
def _inject_session_instruction(conversation_history):
    """Inject ephemeral system message with aphrodite version + proxy info.

    Called once per session from ``_pre_llm_hook`` on its first invocation.
    Injects a single ``💋`` system message so every session starts
    with the agent aware of the CCR toolchain and proxy state.
    """
    # Determine engine threshold display
    threshold = ENGINE_THRESHOLD_PCT
    if threshold == 0:
        thresh_str = "disabled (0)"
    elif threshold == -1:
        thresh_str = "always (-1)"
    else:
        thresh_str = f"{threshold}%"
    token_alive = _alive_cached(PORTS["token"])
    # Build the instruction
    lines = [
        f"💋 aphrodite v{PLUGIN_VERSION} active.",
    ]
    if token_alive:
        lines.append(
            f"  Token proxy :9798 active | engine threshold={thresh_str} | "
            f"tools auto-expand inline (<{_fmt_size(AUTO_EXPAND_LIMIT)})"
        )
    else:
        lines.append(
            "  Token proxy :9798 offline | inline fallback active | "
            f"engine threshold={thresh_str}"
        )
    lines.append(_render_prompt_tmpl("session_inject"))
    lines.append(
        "  ─ Layer 2: per-turn catalog injected below each turn ─"
    )
    lines.append(
        "  ─ Layer 3: load aphrodite-tool-guide skill for full tool reference ─"
    )
    instruction = "\n".join(lines)
    conversation_history.append(
        {"role": "system", "content": instruction, "ephemeral": True}
    )
    _log.info("injected session instruction v%s", PLUGIN_VERSION)
    global _session_instruction_injected
    _session_instruction_injected = True


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
                # Count lines, extract def/class/fn names from result
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
                # Try to count files from JSON result
                try:
                    data = json.loads(result)
                    if isinstance(data, list):
                        meta["files"] = str(len(data))
                    elif isinstance(data, dict):
                        # Check common keys: "matches", "files", "results", "count"
                        for key in ("matches", "files", "results"):
                            val = data.get(key)
                            if isinstance(val, list):
                                meta["files"] = str(len(val))
                                break
                        else:
                            meta["files"] = str(data.get("total_count", data.get("count", "?")))
                except (json.JSONDecodeError, ValueError):
                    # Not JSON  -  count lines matching grep output format
                    line_count = 0
                    for line in result.splitlines():
                        line = line.strip()
                        if line and not line.startswith((">", "<", "-", "+")):
                            line_count += 1
                    if line_count:
                        meta["files"] = str(line_count)
                return meta

        elif tool_name == "terminal":
            # Extract exit code from kwargs (passed through Hermes terminal hook)
            exit_code = args.get("exit_code", args.get("returncode", ""))
            if not exit_code:
                # Try to find it in the result itself
                for line in result.splitlines():
                    m = re.match(r"exit code[:\s]+(\d+)", line.strip(), re.IGNORECASE)
                    if m:
                        exit_code = m.group(1)
                        break
            meta = {}
            if exit_code:
                meta["exit"] = str(exit_code)
            # Last non-empty line of output
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


def _classifier_says_skip(klass: dict) -> bool:
    """Classifier poll: does the content have nothing worth retrieving?

    If the classifier signals clean/inert output (0E/0W build, exit=0 terminal,
    0 match search, etc.), we skip CCR marker emission. The preview IS the
    complete story — creating a `<<<CCR:hash>>>` marker just baits the LLM
    into a wasteful retrieval round-trip.

    The content IS still stored in CCR for search/history. We just don't
    show the marker to the LLM.
    """
    if not CLASSIFIER_POLL:
        return False
    ctype = klass.get("type", "")
    if ctype in ("build_output", "build_error"):
        if klass.get("errors", "0") in ("0", "") and klass.get("warnings", "0") in ("0", ""):
            return True
    if ctype == "terminal" and klass.get("exit") == "0":
        return True
    if ctype in ("search_files", "search_results") and klass.get("total", "0") in ("0", ""):
        return True
    return False


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
    """Compress tool outputs via CCR. Proxy first, inline fallback when proxy down.

    Dual-mode: proxy CCR (token >1KB, cache >8KB) with inline fallback (>4KB).
    Works without proxy - no provider switch required.
    """
    _t0 = time.time()
    if not result or not isinstance(result, str) or not result.strip():
        return result

    # Track file references for aphrodite_files tool (before dev guard to always track)
    _track_file_refs(tool_name, args)
    if _DEV:
        return result  # dev mode: passthrough
    token_alive = _alive_cached(9798)
    cache_alive = _alive_cached(9797)
    proxy_available = token_alive or cache_alive

    # Determine marker type: aphrodite meta-tools get auto-expanded
    # so the LLM always sees navigation/aid info inline; regular
    # tool results stay wrapped as CCR markers to save context.
    marker_type = "aphrodite" if tool_name.startswith("aphrodite_") else "tool"
    skip = _ESSENTIAL_TOOLS
    if tool_name in skip:
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: SKIP %s %.1fms (in skip list)", tool_name[:40], (time.time() - _t0) * 1000
            )
        # Format aphrodite tool outputs as rich markdown for readability
        return _format_aphrodite_output(tool_name, result)

    threshold = TOOL_THRESHOLD_TOKEN if token_alive else TOOL_THRESHOLD_CACHE if cache_alive else INLINE_THRESHOLD
    result_len = len(result)
    # Big-payload guard: skip compression for payloads exceeding MAX_REQUEST_BODY_SIZE
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
                tool_name[:40],
                result_len,
                threshold,
                (time.time() - _t0) * 1000,
            )
        return result

    # Don't re-compress content that already has CCR markers (retrieved/compressed)
    if _CCR_RE.search(result):
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: GUARD %s has existing CCR marker %.1fms",
                tool_name[:40],
                (time.time() - _t0) * 1000,
            )
        return result

    # ── Trust the classifier: skip CCR for clean/uninteresting outputs ──
    # The classifier poll ("real poll intent") determines if content is
    # worth an LLM retrieval. Clean outputs get stored for history but
    # the CCR marker is omitted — the preview IS the complete story.
    klass = _classify_content(result)
    if _classifier_says_skip(klass):
        # Store silently via proxy for search/history, but suppress CCR marker.
        # The LLM sees ONLY the preview — no bait to retrieve.
        if proxy_available:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            _compress_via_proxy(result, target, headers=_headroom_context or None)
        return _make_ccr_preview(result, klass=klass, model_family=_detect_model_family())

    preview = _make_ccr_preview(result, klass=klass, model_family=_detect_model_family())
    metadata = _extract_tool_metadata(tool_name, args, result)

    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(result, target, headers=_headroom_context or None)
        if ccr:
            h, sz = ccr
            # Bridge hash formats: map full SHA-256 → canonical proxy hash
            # so _compress_handler can find inline-cached content (#51)
            full_sha = hashlib.sha256(result.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            label = "token" if token_alive else "cache"
            if DEBUG_LOGGING:
                ratio = result_len / max(len(h), 1)
                _log.debug(
                    "transform_tool_result: CCR %s %s:%s size=%s ratio=%.1fx %.1fms",
                    tool_name[:40],
                    label,
                    h,
                    result_len,
                    ratio,
                    (time.time() - _t0) * 1000,
                )
            _recent_markers.append(
                {"hash": h, "type": marker_type, "size": result_len, "preview": preview, "turn": _state["turn_counter"], "meta": metadata or {}}
            )
            _inline_store_put(h, result)
            return _ccr_marker(
                h, marker_type, result_len, label, preview,
                headroom_budget=_headroom_context.get("x-headroom-budget"),
                meta=metadata,
            )
        elif DEBUG_LOGGING:
            _log.debug("transform_tool_result: PROXY FAIL %s - proxy returned no hash", tool_name[:40])

    # Fallback: inline compression (works without proxy)
    if result_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(result)
            # Bridge hash formats: map full SHA-256 → canonical inline hash
            # so _compress_handler can find inline-cached content (#51)
            full_sha = hashlib.sha256(result.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                _log.debug(
                    "transform_tool_result: INLINE %s hash=%s size=%s %.1fms",
                    tool_name[:40],
                    h,
                    result_len,
                    (time.time() - _t0) * 1000,
                )
            _recent_markers.append(
                {"hash": h, "type": marker_type, "size": result_len, "preview": preview, "turn": _state["turn_counter"], "meta": metadata or {}}
            )
            return _ccr_marker(
                h, marker_type, result_len, "inline", preview,
                headroom_budget=_headroom_context.get("x-headroom-budget"),
                meta=metadata,
            )
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("transform_tool_result: INLINE FAIL %s", tool_name[:40])
            pass
    if DEBUG_LOGGING:
        _log.debug(
            "transform_tool_result: PASSTHROUGH %s size=%s %.1fms",
            tool_name[:40],
            result_len,
            (time.time() - _t0) * 1000,
        )
    return result  # soft-fail


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate, kill running proxies, replace binary, restart."""
    import shutil

    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "aphrodite"],
        cwd=repo,
        capture_output=True,
        text=True,
        timeout=300,
        env={**os.environ, "PATH": f"{os.path.expanduser('~/.cargo/bin')}:{os.environ.get('PATH', '')}"},
    )
    if result.returncode != 0:
        return f'{{"error": "build failed: {result.stderr[-200:]}"}}'

    src = os.path.join(repo, "target/release/aphrodite")
    if not os.path.exists(src):
        return '{"error": "binary not found after build"}'

    # ── Kill running proxies ──────────────────────────────────────
    killed = []
    for port in PORTS.values():
        try:
            r = subprocess.run(["lsof", "-ti", f":{port}"], capture_output=True, text=True, timeout=5)
            if r.stdout.strip():
                for pid in r.stdout.strip().split("\n"):
                    try:
                        os.kill(int(pid), 9)
                        killed.append(f":{port}({pid})")
                    except (OSError, ProcessLookupError):
                        pass
        except FileNotFoundError:
            killed.append(f":{port}(lsof-missing)")
        except Exception:
            pass

    # ── Replace binary ───────────────────────────────────────────
    shutil.copy2(src, BINARY)
    os.chmod(BINARY, 0o755)

    # ── Restart proxies ──────────────────────────────────────────
    import time as _time
    _time.sleep(0.3)  # let ports release
    restarted = []
    from ._proxy import _start as _proxy_start, _query_proxy_version
    for name in ("cache", "token"):
        try:
            _proxy_start(name, os.environ.copy())
            restarted.append(name)
        except Exception:
            pass

    # ── Query proxy version after restart ────────────────────────
    _time.sleep(0.3)
    proxy_ver = _query_proxy_version(PORTS["token"]) or "?"

    return json.dumps({
        "ok": True,
        "size": os.path.getsize(BINARY),
        "path": BINARY,
        "killed": killed,
        "restarted": restarted,
        "proxy_version": proxy_ver,
    })


REBUILD_SCHEMA = {
    "name": "aphrodite_rebuild",
    "description": "Rebuild aphrodite crate from source and install binary. Use after code changes.",
    "parameters": {"type": "object", "properties": {}},
}


# ── Conversation Memory via CCR ─────────────────────────────────────


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    if not conversation_history or assistant_response is None:
        return

    if _DEV:
        return
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]
    tnum = _increment_turn()

    # Use cached last user message from pre_llm_hook (avoids scanning full history)
    last_user = _last_user_msg

    summary = f"T{tnum}: {last_user}… → {str(assistant_response)[:200]}"
    # Tag by file type for better retrieval
    if _referenced_files:
        exts = {}
        for path in list(_referenced_files)[-10:]:  # recent files
            ext = os.path.splitext(path)[1] or "noext"
            exts[ext] = exts.get(ext, 0) + 1
        top_exts = sorted(exts.items(), key=lambda x: x[1], reverse=True)[:3]
        file_tag = " ".join(f"{ext}({n})" for ext, n in top_exts)
        summary += f" [{file_tag}]"

    try:
        # Cap stored assistant response at 4096 chars to bound turn index size
        capped_resp = str(assistant_response)[:4096]
        data = json.dumps(
            {
                "turn": tnum,
                "user": last_user,
                "assistant": capped_resp,
            }
        ).encode()
        store_headers = {"Content-Type": "application/octet-stream"}
        if _headroom_context:
            store_headers.update(_headroom_context)
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create", data=data, headers=store_headers
        )
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())

        # Pop oldest if at capacity before inserting (prevents transient >100)
        if len(_conv_index) >= 100:
            oldest = next(iter(_conv_index))
            del _conv_index[oldest]
        _conv_index[tnum] = (ccr["hash"], summary, len(capped_resp))

        _log.debug("conv-cache: stored T%d → %s (%d total)", tnum, ccr["hash"], len(_conv_index))
    except Exception as exc:
        _log.debug("_store_conversation_turn: %s", exc)


def _git_summary(cwd: str | None = None):
    """Get cached git diff --stat summary. Returns string or None.
    ``cwd`` defaults to the session's current working directory."""
    if cwd is None:
        cwd = os.getcwd()
    now = time.time()
    if _git_cache.get("ts", 0) > now - 30:
        return _git_cache.get("summary")
    try:
        import subprocess

        r = subprocess.run(["git", "diff", "--stat"], capture_output=True, text=True, timeout=3, cwd=cwd)
        if r.returncode == 0 and r.stdout.strip():
            summary = r.stdout.strip().split("\n")[-1] if r.stdout.strip() else None
            _git_cache["ts"] = now
            _git_cache["summary"] = summary
            return summary
    except Exception as exc:
        _log.debug("_git_summary: %s", exc)
    return None


def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
    """Before LLM call: build navigable compression catalog.

    CANNOT mutate conversation_history (Hermes passes a copy). Instead:

    WRAPPING PATTERN visible to LLM:
    ┌─ Last ~10 messages: raw, fully in context
    ├─ Tool/terminal outputs >1KB: <<<CCR:hash|type|size>>> markers inline
    ├─ Old turn summaries: compressed to CCR, cataloged here
    └─ Everything else: raw user/assistant text (Hermes keeps it)

    STRATEGY: Provide catalog so LLM uses aphrodite_retrieve(hash)
    instead of scanning 300+ raw messages. Each CCR item below is
    retrievable - the LLM should fetch only what's relevant.
    """
    if _DEV:
        return
    if not conversation_history or not isinstance(conversation_history, list):
        return

    quiet_mode = os.environ.get("QUIET", "") == "1"
    if quiet_mode and DEBUG_LOGGING:
        _log.debug("pre_llm_hook: quiet_mode=1, catalog injection suppressed")

    # Refresh turn-scoped alive cache for consistent proxy state within this turn
    global _scanned_msg_idx, _last_user_msg, _catalog_injected_this_turn
    _alive_cache.clear()  # purge 5s cache so turn-probes below are fresh
    _alive_turn_cache.clear()
    _alive_turn_cache[PORTS["token"]] = _alive(PORTS["token"])
    _alive_turn_cache[PORTS["cache"]] = _alive(PORTS["cache"])
    _last_user_msg = user_message or ""  # cache for _store_conversation_turn
    _catalog_injected_this_turn = False  # reset per-turn, allows re-injection on fresh calls

    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None
    ctx_len = len(conversation_history)

    # ── 0a. Headroom feedback loop: query fill_pct from proxy, set headroom_budget ──
    if target and proxy_available:
        _query_and_set_headroom_budget(target)

    # ── 0b. Inject session-start instruction (once per session) ──
    if not _session_instruction_injected and not _DEV:
        _inject_session_instruction(conversation_history)

    # ── 0b. Pass through x-headroom-* headers (skip bypass) ──
    headroom_hdrs = {}
    headers = kwargs.get("headers")
    if headers:
        _update_headroom_context(dict(headers))
        for k, v in headers.items():
            kl = k.lower()
            if kl.startswith("x-headroom-") and kl != "x-headroom-bypass":
                headroom_hdrs[k] = v

    # ── 1. Scan for CCR markers (incremental - only tool/system messages + "CCR:" fast-check) ──
    markers = []
    total_bytes = 0
    start_idx = max(0, _scanned_msg_idx)
    for msg in conversation_history[start_idx:]:
        role = msg.get("role", "")
        if role not in ("tool", "system"):  # user/assistant messages never carry CCR markers
            continue
        content = msg.get("content", "")
        if isinstance(content, str) and "CCR:" in content:  # fast substring check before full regex
            for m in _parse_ccr_markers(content):
                total_bytes += m["size"]
                markers.append(m)
    _scanned_msg_idx = ctx_len  # advance past current messages
    # Merge with existing: keep markers not found in new scan, cap at 200
    seen_hashes = {m["hash"] for m in markers if "hash" in m}
    for old_m in _recent_markers:
        if old_m.get("hash") not in seen_hashes:
            markers.append(old_m)
            seen_hashes.add(old_m["hash"])
    # Only append new markers (hash-set-diff) instead of full clear+extend
    if seen_hashes != {m["hash"] for m in _recent_markers}:
        current_hashes = {m["hash"] for m in _recent_markers}
        for m in markers:
            if m["hash"] not in current_hashes:
                _recent_markers.append(m)
    if DEBUG_LOGGING and markers:
        _log.debug(
            "pre_llm_hook: scanned %d CCR markers across %d msgs, %s total compressed",
            len(markers),
            ctx_len,
            _fmt_size(total_bytes),
        )

    # ── 1.4 Auto-classify new entries lacking metadata ──────────
    # For any marker without a non-empty ``meta`` field, attempt to
    # classify its content retroactively. This enriches old entries
    # that were created before structured metadata was introduced.
    # Only processes markers accessible via inline store (fast path);
    # proxy-resolved classification is opt-in via aphrodite_reclassify.
    classified_this_turn = 0
    for m in _recent_markers:
        if m.get("meta") and m["meta"] != {}:
            continue
        h = m.get("hash", "")
        if not h:
            continue
        h_bare = h[2:] if h.startswith("i:") else h
        content = _inline_store.get(h_bare)
        if content is None:
            continue
        try:
            klass = _classify_content(content)
            m["meta"] = klass
            classified_this_turn += 1
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("pre_llm_hook: auto-classify failed for %s", h[:12])
    if classified_this_turn and DEBUG_LOGGING:
        _log.debug(
            "pre_llm_hook: auto-classified %d entries lacking metadata",
            classified_this_turn,
        )

    # ── 1.5 Auto-expand small tool CCR markers ──────────────────
    # For each tool-type marker small enough to inline, resolve and
    # replace in conversation_history. Context/terminal markers stay
    # as markers - the LLM retrieves them via catalog hints.
    _expanded_hashes: set = set()  # hashes resolved and replaced inline
    if AUTO_EXPAND_LIMIT > 0:
        expanded_count = 0
        for msg in conversation_history:
            role = msg.get("role", "")
            if role not in ("tool", "system"):
                continue
            content = msg.get("content", "")
            if not isinstance(content, str) or "CCR:" not in content:
                continue

            replacements = {}  # full_marker_str -> resolved_content
            for match in _CCR_RE.finditer(content):
                full_marker = match.group(0)
                h = match.group(1)
                # Extract inner content between CCR: and closing delimiter
                inner = full_marker.split("CCR:", 1)[1]
                for suffix in (">>>", "]", "⫸"):
                    if inner.endswith(suffix):
                        inner = inner[: -len(suffix)]
                        break
                parts = inner.split("|")
                if len(parts) < 3:
                    continue
                marker_type = str(parts[1])
                if marker_type != "aphrodite":
                    continue  # only expand aphrodite meta-tool markers
                try:
                    marker_size = int(parts[2])
                except ValueError:
                    continue
                if marker_size >= AUTO_EXPAND_LIMIT:
                    continue  # too large, leave as marker

                # Attempt resolution - inline store first, then proxy
                resolved = _resolve_one(h, timeout=2)
                if resolved is not None and len(resolved) < AUTO_EXPAND_LIMIT:
                    replacements[full_marker] = resolved
                    _expanded_hashes.add(h)
                elif DEBUG_LOGGING:
                    _log.debug(
                        "pre_llm_hook: auto-expand skip %s size=%s (unresolved or oversized)",
                        h[:12], marker_size,
                    )

            if replacements:
                new_content = content
                for old, new in replacements.items():
                    new_content = new_content.replace(old, new, 1)
                msg["content"] = new_content
                n = len(replacements)
                expanded_count += n
                if DEBUG_LOGGING:
                    _log.debug(
                        "pre_llm_hook: auto-expanded %d tool marker(s) in %s message (%d→%d chars)",
                        n, role, len(content), len(new_content),
                    )
        if expanded_count:
            _log.debug(
                "pre_llm_hook: auto-expanded %d tool CCR markers total (limit=%s)",
                expanded_count, _fmt_size(AUTO_EXPAND_LIMIT),
            )

    # Filter out auto-expanded markers from catalog - already visible inline to LLM
    if _expanded_hashes:
        before = len(markers)
        markers = [m for m in markers if m["hash"] not in _expanded_hashes]
        if DEBUG_LOGGING:
            _log.debug(
                "pre_llm_hook: filtered %d expanded markers from catalog (%d → %d)",
                before - len(markers), before, len(markers),
            )

    # CATALOG_MODE "tool" early-return when no markers - nothing to catalog
    if CATALOG_MODE == "tool" and not markers:
        return None

    # ── 2. Compress old turns to CCR (skip already-compressed) ──
    compress_hint = ""
    if proxy_available and target and ctx_len > 30:
        turns = _group_into_turns(conversation_history)
        if len(turns) > 6:
            old_turns = turns[:-6]
            # Filter out turns already in _conv_index (prevents re-compression)
            old_turns = [t for t in old_turns if t["id"] not in _conv_index]
            if not old_turns:
                compress_hint = ""  # already compressed, skip
            else:
                try:
                    summaries = []
                    for t in old_turns:
                        summaries.append(
                            {
                                "turn": t["id"],
                                "user": t.get("user", "")[:1000],
                                "assistant": t.get("assistant", "(tool calls)")[:1000],
                            }
                        )
                    packed = json.dumps(summaries)
                    if len(packed) > 500:
                        data = packed.encode()
                        archive_headers = {"Content-Type": "application/octet-stream"}
                        if _headroom_context:
                            archive_headers.update(_headroom_context)
                        req = urllib.request.Request(
                            f"http://127.0.0.1:{target}/ccr/create",
                            data=data,
                            headers=archive_headers,
                        )
                        with urllib.request.urlopen(req, timeout=3) as r:
                            ccr = json.loads(r.read())
                        kept = len(turns) - len(old_turns)
                        compress_hint = (
                            f"  [TURN ARCHIVE] CCR:{ccr['hash']} | "
                            f"turns T{turns[0]['id']}-T{old_turns[-1]['id']} "
                            f"({len(old_turns)} turns compressed, last {kept} raw)\n"
                        )
                        # Store sentinel to prevent re-compression on LLM failure
                        for t in old_turns:
                            _conv_index[t["id"]] = (ccr["hash"], f"turn {t['id']}", 0)
                except Exception as exc:
                    _log.debug("_pre_llm_hook turn archive: %s", exc)

    # ── 3. Build the catalog (mode-aware) ─────────────────────
    parts = []
    if markers or _conv_index or compress_hint or len(_referenced_files) > 5 or DEBUG_LOGGING or _expand_guidance:
        parts.append("💋")

        # Auto-expand guidance - reminds LLM that tool CCR markers are
        # resolved inline and only context/terminal markers need retrieval
        if _expand_guidance:
            parts.append(f"  {_expand_guidance}")

        # ── Auto line: build status + uncommitted changes + proxy health ─
        from ._automation import _auto_build_watch, _auto_commit_reminder

        auto_parts = []
        build_info = _auto_build_watch()
        if build_info:
            auto_parts.append(build_info.replace("  ", ""))
        commit_info = _auto_commit_reminder()
        if commit_info:
            auto_parts.append(commit_info.replace("  ", ""))
        up_ports = [str(port) for name, port in PORTS.items() if _alive_turn_cache.get(port)]
        if up_ports:
            auto_parts.append(f"proxy: {','.join(up_ports)} up")
        else:
            auto_parts.append("proxy: none ⚠")
        if auto_parts:
            parts.append("  [AUTO] " + " | ".join(auto_parts))

        # Debug banner (only in DEBUG mode or full catalog)
        if DEBUG_LOGGING or CATALOG_MODE == "full":
            parts.append(
                f"  ⚙ v{PLUGIN_VERSION} | engine={'on' if CONTEXT_ENGINE else 'off'} | dev={'on' if _DEV else 'off'}"
            )
            parts.append(
                f"  ⚙ thresholds: term={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_tok={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE} engine_pct={ENGINE_THRESHOLD_PCT}% prot={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min={ENGINE_MIN_MSGS}"
            )

        # Git diff summary (skip in tool-only mode)
        if CATALOG_MODE != "tool":
            git_info = _git_summary()
            if git_info:
                parts.append(f"  git: {git_info}")

        # Compression wrapping summary (compact by type in compact/tool mode)
        if proxy_available:
            mode = "token" if token_alive else "cache"
            if CATALOG_MODE == "tool":
                parts.append(f"  {len(markers)} items compressed")
            elif CATALOG_MODE == "compact":
                # Group by type for compact display
                by_type = {}
                for m in markers:
                    by_type.setdefault(m["type"], []).append(m)
                if by_type:
                    type_parts = " ".join(f"{len(items)} [{ctype}]" for ctype, items in sorted(by_type.items()))
                    parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved) - {type_parts}")
                else:
                    parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved)")
            else:  # full
                parts.append(f"  mode={mode} | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
        elif CATALOG_MODE != "tool":
            parts.append(f"  mode=inline | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")

        # Auto-expand guidance
        if markers:
            parts.append(
                "  ⚡ Tool outputs auto-expand before you see them - full content is inline. "
                "Context/terminal markers require aphrodite_retrieve(hash) to fetch."
            )

        # Layer 2: per-turn hint with counts and cross-reference
        if markers or len(_expanded_hashes) > 0 or not _session_instruction_injected:
            hint = [
                f"  [{len(markers)} markers available | {len(_expanded_hashes)} tool outputs auto-expanded this turn]",
                "  Call aphrodite_catalog to list all entries, aphrodite_retrieve(hash) to fetch.",
                "  For full tool reference, load aphrodite-tool-guide skill (skill_view).",
            ]
            parts.extend(hint)

        # Engine stats
        engine = get_engine()
        if engine and engine.compression_count > 0:
            parts.append(
                f"  engine: {engine.compression_count} compressions | last: {engine.last_compression.get('messages_compressed', '?')} msgs → CCR:{engine.last_compression.get('hash', '?')[:8]}"
            )

        # Turn archive
        if compress_hint:
            parts.append(compress_hint)

        # Full CCR catalog: grouped by type with previews (full mode only)
        if CATALOG_MODE == "full" and markers:
            live = [m for m in markers if m["hash"] in _inline_store or _inline_retrieve(m["hash"])]
            if not live and markers:
                live = markers

            # Auto-expand: resolve small cached items inline in a single pass,
            # building preview_cache to avoid double read and mutation of shared markers
            preview_cache = {}
            expanded = []
            for m in live:
                h = m.get("hash", "")
                if m.get("size", 0) < 10240 and h in _inline_store:
                    content = _inline_store[h]
                    preview = content[:200].replace("\n", " ").strip()
                    preview_cache[h] = preview
                else:
                    preview_cache[h] = m.get("preview", "")
                expanded.append({**m, "preview": preview_cache[h]})
            live = expanded

            # Deduplicate by hash - keep first occurrence only
            seen = set()
            deduped = []
            for m in live:
                if m["hash"] not in seen:
                    seen.add(m["hash"])
                    deduped.append(m)
            live = deduped

            by_type = {}
            for m in live:
                by_type.setdefault(m["type"], []).append(m)

            parts.append(f"  catalog ({len(markers)} items):")
            for ctype, items in sorted(by_type.items()):
                visible = min(len(items), 3)
                parts.append(f"    [{ctype}] {len(items)} items:")
                for i, m in enumerate(items[:visible]):
                    preview = preview_cache.get(m["hash"], "")
                    h = str(m.get("hash", "")).strip()
                    if len(h) < 4 or h in ("{}", "?", "None", "null", "undefined"):
                        continue
                    meta = m.get("meta", {}) or {}
                    if meta:
                        kvs = ", ".join(f"{k}={v}" for k, v in sorted(meta.items()))
                        parts.append(f"      {h[:12]} - {m.get('type', '?')} [{kvs}] ({_fmt_size(m['size'])})")
                    else:
                        parts.append(f"      CCR:{h} | {_fmt_size(m['size'])} | {preview}")
                if len(items) > visible:
                    parts.append(f"      ... +{len(items) - visible} more")

            parts.append("  ⚡ Markers include structured metadata - use hints to decide retrieval.")

        # Conversation memory (full mode only - already in system prompt)
        if CATALOG_MODE == "full" and _conv_index:
            recent = sorted(_conv_index.items(), reverse=True)[:3]
            parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in recent))

        # File tree: compact in non-full modes
        if len(_referenced_files) > 5:
            if CATALOG_MODE == "full":
                by_dir = {}
                for path in sorted(_referenced_files):
                    d = os.path.dirname(path) or "."
                    by_dir.setdefault(d, []).append(os.path.basename(path))
                parts.append(f"  files: {len(_referenced_files)} referenced:")
                for d, files in sorted(by_dir.items())[:8]:
                    parts.append(f"    {d}/ {', '.join(files[:6])}")
                    if len(files) > 6:
                        parts.append(f"      ... +{len(files) - 6} more")
                if len(by_dir) > 8:
                    parts.append(f"    ... +{len(by_dir) - 8} more dirs")
            else:
                parts.append(f"  files: {len(_referenced_files)} referenced")

        # Context hint (skip in tool mode)
        if CATALOG_MODE != "tool" and ctx_len > 20:
            if ctx_len > 100:
                parts.append(_render_prompt_tmpl("catalog_context_warn", {"ctx": ctx_len}))
            else:
                parts.append(f"  context={ctx_len} msgs")

        # Read-intent detection (skip in tool mode)
        if CATALOG_MODE != "tool":
            last_user = user_message or ""
            if not last_user and isinstance(conversation_history, list):
                for msg in reversed(conversation_history):
                    if msg.get("role") == "user":
                        last_user = str(msg.get("content", ""))[:200].lower()
                        break
            words = set(last_user.lower().split())
            has_read_intent = bool(words & _READ_KEYWORDS)
            if has_read_intent and markers:
                recent_markers = markers[-3:]
                hashes = " ".join(m['hash'][:12] for m in recent_markers)
                parts.append(
                    f"  intent=read | recent CCRs: {hashes}"
                )

    if quiet_mode:
        if DEBUG_LOGGING:
            _log.debug("pre_llm_hook: quiet_mode=1, catalog skipped")
        return None

    if parts:
        if _catalog_injected_this_turn:
            if DEBUG_LOGGING:
                _log.debug("pre_llm_hook: SKIP catalog inject (already injected this turn)")
            return None

        catalog = "\n".join(parts)
        if DEBUG_LOGGING:
            _log.debug(
                "pre_llm_hook: catalog (%d lines, %d markers, %d files)",
                len(parts),
                len(markers),
                len(_referenced_files),
            )
            _log.debug(
                "pre_llm_hook: %d markers parsed, %d skipped (empty/bad hash)",
                len(markers),
                sum(1 for m in markers if len(str(m.get("hash", ""))) < 4),
            )
        # Inject catalog as ephemeral system message - Hermes expects None from pre_llm_call hooks
        conversation_history.append({"role": "system", "content": catalog, "ephemeral": True})
        _catalog_injected_this_turn = True
    return None


def _group_into_turns(conversation_history):
    """Group messages into turns (user → assistant → tools)."""
    turns = []
    current = None
    turn_num = 0
    for msg in conversation_history:
        role = msg.get("role", "")
        content = msg.get("content", "")
        if role == "user":
            if current:
                turns.append(current)
            turn_num += 1
            current = {"id": turn_num, "user": str(content)[:1000]}
        elif role == "assistant" and current:
            current["assistant"] = str(content)[:1000]
        elif role == "tool" and current:
            # Store first 200 chars of tool content for turn summary
            raw = str(content)[:200] if content else ""
            if raw:
                current.setdefault("tools", []).append(raw)
    if current:
        turns.append(current)
    return turns


def _extract_preview(marker, conversation_history):
    """Extract a short preview for a CCR marker from conversation history.

    NOTE: No longer called by catalog loop - replaced by O(1) preview_cache.
    Kept as fallback for manual/inline use outside pre_llm_hook.
    """
    h = marker["hash"]
    for msg in conversation_history:
        c = msg.get("content", "")
        if isinstance(c, str) and h in c:
            idx = c.find(h)
            after = c[idx + len(h) :].strip()
            if ">>>" in after:
                after = after.split(">>>", 1)[-1].strip()
            return after[:80].strip()
    return ""


def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
    """Compress terminal output via CCR on-the-fly. Proxy first, inline fallback.
    Build output gets smart summarization - repeated patterns collapsed."""
    _t0 = time.time()
    if _DEV:
        return output  # dev mode: passthrough
    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    out_len = len(output)
    orig_len = out_len  # saved before possible build mutation below
    if out_len < TERMINAL_THRESHOLD:  # use configured threshold
        if DEBUG_LOGGING:
            _log.debug(
                "terminal_hook: BELOW size=%s < threshold=%s %.1fms (cmd: %s)",
                out_len,
                TERMINAL_THRESHOLD,
                (time.time() - _t0) * 1000,
                command[:60],
            )
        return output

    # Don't re-compress content that already has CCR markers (retrieved/compressed)
    if _CCR_RE.search(output):
        if DEBUG_LOGGING:
            _log.debug(
                "terminal_hook: GUARD has existing CCR marker %.1fms (cmd: %s)",
                (time.time() - _t0) * 1000,
                command[:60],
            )
        return output

    # ── Build output detection: collapse repeated lines ──────────────
    first_line = output.split("\n", 1)[0].strip() if output else ""
    is_build = any(
        first_line.startswith(p)
        for p in (
            "Compiling ",
            "   Compiling ",
            "Finished ",
            "error:",
            "warning:",
            "Running ",
            "PASSED",
            "FAILED",
            "test result:",
        )
    )
    if is_build:
        lines = output.splitlines()
        if len(lines) <= 20:
            pass  # short build - passthrough to regular handling below
        else:
            # Count unique patterns, deduplicate consecutive repeats
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

            # Build summary: unique error/warning lines + total
            errors = [l for l in unique if "error" in l.lower() and l not in ("error:", "error")]
            warnings = [l for l in unique if "warning" in l.lower() and "warning:" not in l]
            summary = f"[build: {len(lines)} lines, {len(unique)} unique patterns]"
            if errors:
                summary += f" | errors: {'; '.join(errors[:5])}"
            if warnings:
                summary += f" | warnings: {'; '.join(warnings[:3])}"

            # ── Clean build: no CCR needed, return summary inline ─────
            if not errors and not warnings:
                if DEBUG_LOGGING:
                    _log.debug("terminal_hook: clean build — inline summary, no CCR")
                return summary
            out_len = len(summary)
            if DEBUG_LOGGING:
                _log.debug(
                    "terminal_hook: BUILD collapse %d→%d lines (cmd: %s)",
                    len(lines),
                    len(summary.split("\n")),
                    command[:60],
                )
            # Store full output in CCR, return summary
            if proxy_available:
                target = PORTS["token"] if token_alive else PORTS["cache"]
                ccr = _compress_via_proxy(output, target, headers=_headroom_context or None)
                if ccr:
                    h, _ = ccr
                    # Bridge hash formats for _compress_handler cache hits (#51)
                    full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
                    _hash_alias[full_sha] = h
                    if DEBUG_LOGGING:
                        _log.debug("terminal_hook: BUILD-CCR %s:%s", "token" if token_alive else "cache", h)
                    _recent_markers.append({"hash": h, "type": "build", "size": len(output), "preview": summary})
                    return f"<<<CCR:{h}|build|{len(output)}>>> {summary}"
            # Inline fallback
            h, _ = _inline_compress(output)
            # Bridge hash formats for _compress_handler cache hits (#51)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            _recent_markers.append({"hash": h, "type": "build", "size": len(output), "preview": summary})
            return f"<<<CCR:{h}|build|{len(output)}>>> {summary}…(use aphrodite_retrieve)"

    # ── Classifier poll: clean terminal outputs skip CCR ───────────────
    klass = _classify_content(output)
    if _classifier_says_skip(klass):
        return _make_ccr_preview(output, klass=klass, model_family=_detect_model_family())

    preview = _make_ccr_preview(output, model_family=_detect_model_family())

    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(output, target)
        if ccr:
            h, _ = ccr
            # Bridge hash formats for _compress_handler cache hits (#51)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                ratio = out_len / max(len(h), 1)
                _log.debug(
                    "terminal_hook: CCR %s:%s size=%s ratio=%.1fx",
                    "token" if token_alive else "cache",
                    h,
                    orig_len,
                    ratio,
                )
            _recent_markers.append({"hash": h, "type": "terminal", "size": orig_len, "preview": preview})
            return f"<<<CCR:{h}|terminal|{orig_len}>>> {preview}"
        elif DEBUG_LOGGING:
            _log.debug("terminal_hook: PROXY FAIL - returned no hash (cmd: %s)", command[:60])

    # Fallback: inline compression
    if orig_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(output)
            # Bridge hash formats for _compress_handler cache hits (#51)
            full_sha = hashlib.sha256(output.encode("utf-8")).hexdigest()
            _hash_alias[full_sha] = h
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE hash=%s size=%s", h, orig_len)
            _recent_markers.append({"hash": h, "type": "terminal", "size": orig_len, "preview": preview})
            return f"<<<CCR:{h}|terminal|{orig_len}>>> {preview}"
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE FAIL (cmd: %s)", command[:60])
            pass
    if DEBUG_LOGGING:
        _log.debug("terminal_hook: PASSTHROUGH size=%s %.1fms", out_len, (time.time() - _t0) * 1000)
    return output


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

    # Proxy health
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

    # Engine status
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
    "description": "Check aphrodite proxy health, CCR stats, engine compression status. Use when debugging compression or checking if proxy is alive.",
    "parameters": {"type": "object", "properties": {}},
}

# ── File tracking (for aphrodite_files tool) ──────────────────


def _track_file_refs(tool_name, args):
    """Track file paths referenced by tool calls. Uses OrderedDict LRU eviction."""
    if tool_name not in _FILE_TOOLS:
        return
    args = args if isinstance(args, dict) else {}
    path = args.get("path", args.get("file", ""))
    if path and isinstance(path, str) and len(path) < 500:
        _referenced_files[path] = tool_name  # set value (new or overwrite)
        _referenced_files.move_to_end(path)  # promote to most recently used
        if len(_referenced_files) > 200:
            _referenced_files.popitem(last=False)  # evict oldest (first inserted)


def _files_handler(args=None, **kwargs):
    """List all files referenced in the current session."""
    if not _referenced_files:
        return json.dumps({"files": [], "count": 0, "hint": "No file operations yet"})
    by_tool = {}
    for path, tool in sorted(_referenced_files.items()):
        by_tool.setdefault(tool, []).append(path)
    return json.dumps(
        {
            "count": len(_referenced_files),
            "by_tool": {t: sorted(paths) for t, paths in sorted(by_tool.items())},
            "all": sorted(_referenced_files.keys()),
        }
    )


FILES_SCHEMA = {
    "name": "aphrodite_files",
    "description": "List all file paths referenced in the current session. Grouped by tool type. Use to see what files have been touched before making decisions.",
    "parameters": {"type": "object", "properties": {}},
}


def _diff_handler(args=None, **kwargs):
    """Show conversation turn diffs - what was discussed in recent turns."""
    if not _conv_index:
        return json.dumps({"turns": 0, "hint": "No turn history yet"})
    turns = []
    for tnum in sorted(_conv_index.keys(), reverse=True)[:10]:
        h, summary, size = _conv_index[tnum]
        turns.append({"turn": tnum, "hash": h, "summary": summary, "size": size})
    return json.dumps({"turns": len(_conv_index), "recent": turns})


DIFF_SCHEMA = {
    "name": "aphrodite_diff",
    "description": "Show conversation turn history - what was discussed, compressed, and stored across turns. Use to understand context evolution.",
    "parameters": {"type": "object", "properties": {}},
}


def _catalog_handler(args=None, **kwargs):
    """Return compression catalog. Mode 'toc' returns compact table-of-contents."""
    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "full")

    if mode == "toc":
        return _build_toc()

    items = []
    for m in _recent_markers:
        items.append({"hash": m["hash"], "type": m["type"], "size": m["size"], "preview": m.get("preview", "")[:120]})
    by_type = {}
    for item in items:
        by_type.setdefault(item["type"], []).append(item["hash"])
    result = {
        "total_items": len(items),
        "total_saved": sum(m["size"] for m in _recent_markers),
        "by_type": {t: {"count": len(hashes), "hashes": hashes[:10]} for t, hashes in sorted(by_type.items())},
        "items": items,
        "conv_turns": len(_conv_index),
        "referenced_files": len(_referenced_files),
    }
    return json.dumps(result, indent=2)


def _build_toc() -> str:
    """Build a compact table-of-contents for the agent to quickly scan before retrieving.

    Shows every CCR entry with hash, type, size, preview, and a 'Retrieve?'
    recommendation (NO if the preview tells the full story, YES if retrieval
    would add useful content).
    """
    markers = list(_recent_markers)
    if not markers:
        return "Catalog: 0 items"

    lines = [
        f"Catalog: {len(markers)} items, {sum(m['size'] for m in markers)}B saved",
        "",
        "| Hash    | Type           | Size  | Preview                          | Retrieve? |",
        "|---------|----------------|-------|----------------------------------|-----------|",
    ]

    for m in reversed(markers[-20:]):  # last 20, newest first
        h = m["hash"][:12]
        t = m["type"][:14]
        s = _fmt_size(m["size"])
        p = (m.get("preview", "") or "")[:45].replace("|", "/")
        # Retrieve recommendation: NO for clean outputs, YES otherwise
        retrieve = "NO"
        if t in ("build_output", "build_error"):
            preview_lower = p.lower()
            if "0e" not in preview_lower and "0w" not in preview_lower:
                retrieve = "YES"
        elif t == "terminal" and "exit=0" not in p:
            retrieve = "YES"
        elif t in ("grep", "search_files", "search_results") and "0 matches" not in p and "0m" not in p:
            retrieve = "YES"
        elif t not in ("build_output", "build_error", "terminal") and "0E 0W" not in p:
            retrieve = "YES"

        lines.append(f"| {h:<7} | {t:<14} | {s:>5} | {p:<45} | {retrieve:<9} |")

    lines.extend(["", "Retrieve? = NO means the preview is sufficient — skip retrieval."])
    return "\n".join(lines)


CATALOG_SCHEMA = {
    "name": "aphrodite_catalog",
    "description": "Return full compression catalog with hashes, sizes, types, previews. Mode 'toc' for compact table-of-contents with Retrieve? recommendations. Use toc BEFORE retrieving to avoid wasted round-trips.",
    "parameters": {
        "type": "object",
        "properties": {"mode": {"type": "string", "description": "Optional: 'toc' for compact table-of-contents, default full catalog"}},
    },
}


def _search_handler(args=None, **kwargs):
    """Search across compressed items by type or content pattern (trigram-indexed)."""
    args = args if isinstance(args, dict) else {}
    query = args.get("query", "").lower()
    ccr_type = args.get("type", "")

    # Guard: require minimum query length for meaningful search
    if query and len(query) < 3:
        return json.dumps(
            {
                "query": query,
                "type_filter": ccr_type,
                "matches": 0,
                "error": "query too short - minimum 3 characters required",
                "results": [],
            }
        )

    # Lazily initialize trigram index on first search
    if not _inline_index_enabled and _inline_store:
        _init_trigram_index()

    results = []

    # ── Search conversation turn index ──
    for tnum, (h, summary, size) in sorted(_conv_index.items(), reverse=True):
        if query and query not in summary.lower():
            continue
        results.append({"source": "turn", "turn": tnum, "hash": h, "summary": summary, "size": size})

    # ── Search inline store (trigram-indexed when index enabled) ──
    if query:
        # Use trigram index for candidate lookup
        trigrams = {query[i : i + 3] for i in range(len(query) - 2)}
        candidates = set()
        if trigrams and _inline_index:
            for tri in trigrams:
                candidates |= _inline_index.get(tri, set())
        # Fallback: full scan only if index is truly empty (nothing indexed yet).
        # If index exists but produced no candidates, the query has no matches
        # - no need to scan, the trigram index is completeness-guaranteed.
        if not candidates and not _inline_index:
            candidates = set(_inline_store.keys())
        for h in candidates:
            content = _inline_store.get(h)
            if content is None:
                continue
            if query not in content.lower():
                continue
            preview = content[:200].replace("\n", " ").strip()
            results.append({"source": "inline", "hash": h, "preview": preview, "size": len(content)})
    else:
        # No query - list all inline entries
        for h, content in list(_inline_store.items()):
            if query and query not in content.lower():
                continue
            preview = content[:200].replace("\n", " ").strip()
            results.append({"source": "inline", "hash": h, "preview": preview, "size": len(content)})

    # Search recent marker catalog (from pre_llm_hook)
    for m in _recent_markers:
        if query and query not in m.get("preview", "").lower():
            continue
        results.append(
            {
                "source": "marker",
                "hash": m["hash"],
                "type": m.get("type", "?"),
                "size": m.get("size", 0),
                "preview": m.get("preview", "")[:200],
            }
        )

    # Deduplicate by hash before filtering
    seen = set()
    unique = []
    for r in results:
        h = r.get("hash", "")
        if h and h not in seen:
            seen.add(h)
            unique.append(r)
    results = unique

    if ccr_type:
        results = [
            r
            for r in results
            if ccr_type in r.get("type", "") or ccr_type in r.get("summary", "") + r.get("preview", "")
        ]

    return json.dumps(
        {
            "query": query,
            "type_filter": ccr_type,
            "matches": len(results),
            "hint": "Use aphrodite_retrieve(hash) to expand any result hash.",
            "results": results[:20],
        }
    )


def _test_handler(args=None, **kwargs):
    """Full smoke test suite - exercises all tools, hooks, compression, search, retrieve."""
    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "quick")  # quick, full, matrix
    report = {"suite": "aphrodite_smoke", "version": PLUGIN_VERSION, "mode": mode, "tests": []}

    def test(name, fn):
        try:
            t0 = time.time()
            result = fn()
            elapsed = (time.time() - t0) * 1000
            report["tests"].append({"name": name, "status": "PASS", "elapsed_ms": round(elapsed, 1), "result": result})
        except Exception as e:
            report["tests"].append({"name": name, "status": "FAIL", "error": str(e)})

    # ── Tool smoke tests ─────────────────────────────────
    test("compress_json", lambda: json.loads(_compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"})))
    test(
        "compress_code",
        lambda: json.loads(_compress_handler(args={"content": "def foo():\n    return 42\n", "type": "code"})),
    )
    test(
        "compress_cache_hit", lambda: _compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"})
    )  # should hit cache

    test(
        "retrieve_roundtrip",
        lambda: (
            (h := json.loads(_compress_handler(args={"content": "def foo():\n    return 42\n", "type": "code"}))["hash"])
            and "def foo"
            in _retrieve_handler(
                args={
                    "hash": h
                }
            )
        ),
    )

    test("stats", lambda: json.loads(_stats_handler())["proxy"])

    test("files_empty", lambda: json.loads(_files_handler())["count"] == 0)

    test("diff_empty", lambda: json.loads(_diff_handler())["turns"] == 0)

    # ── Proxy health ─────────────────────────────────────
    test("proxy_health", lambda: _alive(9798))
    test("proxy_metrics", lambda: _alive(9797))

    # ── Full mode: heavy compression test ────────────────
    if mode in ("full", "matrix"):
        big_payload = json.dumps(
            {"data": list(range(1000)), "nested": {"deep": {"values": [i * i for i in range(200)]}}}
        )
        test(
            "compress_large",
            lambda: json.loads(_compress_handler(args={"content": big_payload, "type": "json"}))["size"] > 1000,
        )
        test("search_find", lambda: json.loads(_search_handler(args={"query": "deep"}))["matches"] >= 1)
        test("terminal_threshold", lambda: TERMINAL_THRESHOLD > 0)
        test("inline_threshold", lambda: INLINE_THRESHOLD > 0)

    # ── Matrix mode: settings sweep ──────────────────────
    if mode == "matrix":
        settings = {"results": {}}
        for pct in (0, 25, 50, 75, 100):
            for protect in (2, 5, 10):
                key = f"pct={pct}_protect={protect}"
                settings["results"][key] = {
                    "threshold_pct": pct,
                    "protect_last": protect,
                    "compresses_always": pct == 0,
                    "compresses_never": pct >= 100,
                }
        report["settings_matrix"] = settings

    # ── Pipeline mode: full + matrix + feature toggles ─────
    if mode == "pipeline":
        # Feature toggle: test with/without debug, with/without compression
        toggles = {
            "debug_on": {"APHRODITE_DEBUG": "1"},
            "debug_off": {"APHRODITE_DEBUG": "0"},
            "engine_on": {"APHRODITE_CONTEXT_ENGINE": "1"},
            "engine_off": {"APHRODITE_CONTEXT_ENGINE": "0"},
        }
        feature_results = {}
        for name, env_overrides in toggles.items():
            saved = {k: os.environ.get(k, "") for k in env_overrides}
            try:
                for k, v in env_overrides.items():
                    os.environ[k] = v
                feature_results[name] = {
                    "env": env_overrides,
                    "proxy_alive": _alive(9798),
                    "cache_alive": _alive(9797),
                    "thresholds": {
                        "terminal": TERMINAL_THRESHOLD,
                        "inline": INLINE_THRESHOLD,
                        "tool_token": TOOL_THRESHOLD_TOKEN,
                        "tool_cache": TOOL_THRESHOLD_CACHE,
                    },
                    "engine_threshold": ENGINE_THRESHOLD_PCT,
                }
            finally:
                for k, orig in saved.items():
                    if orig:
                        os.environ[k] = orig
                    else:
                        os.environ.pop(k, None)
        report["feature_toggles"] = feature_results

    report["summary"] = {
        "total": len(report["tests"]),
        "passed": sum(1 for t in report["tests"] if t["status"] == "PASS"),
        "failed": sum(1 for t in report["tests"] if t["status"] == "FAIL"),
    }

    # ── Save results for regression comparison ─────────────
    try:
        results_path = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite", ".test-results.json")
        prev = {}
        if os.path.exists(results_path):
            with open(results_path) as f:
                prev = json.load(f)
        with open(results_path, "w") as f:
            json.dump(report, f, indent=2)
        if prev:
            prev_passed = prev.get("summary", {}).get("passed", 0)
            curr_passed = report["summary"]["passed"]
            report["regression"] = {
                "previous_passed": prev_passed,
                "current_passed": curr_passed,
                "delta": curr_passed - prev_passed,
                "status": "DEGRADED" if curr_passed < prev_passed else "OK",
            }
    except Exception:
        pass
    return json.dumps(report, indent=2)


TEST_SCHEMA = {
    "name": "aphrodite_test",
    "description": "Run full smoke test suite - compress, retrieve, search, stats, files, diff, proxy health. Modes: quick, full, matrix, pipeline.",
    "parameters": {
        "type": "object",
        "properties": {"mode": {"type": "string", "description": "Test mode: quick (default), full, or matrix"}},
    },
}

SEARCH_SCHEMA = {
    "name": "aphrodite_search",
    "description": "Search across CCR entries - find compressed content by keyword or type. Use to locate previously compressed context without knowing the hash.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search keyword or phrase to find in compressed content"},
            "type": {
                "type": "string",
                "description": "Optional: filter by CCR type (tool, terminal, code, error, etc.)",
            },
        },
        "required": ["query"],
    },
}

RECLASSIFY_SCHEMA = {
    "name": "aphrodite_reclassify",
    "description": "Retroactively classify/metadata-enrich all CCR entries lacking structured metadata. Scans _recent_markers, retrieves content, runs _classify_content, writes meta field. Safe, non-destructive, never modifies original content.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "Optional: reclassify a single CCR entry by hash. If omitted (or action='all'), reclassifies all entries lacking meta."},
            "action": {"type": "string", "description": "Set to 'all' to reclassify all entries lacking meta. Ignored if hash is provided.", "default": "all"},
        },
        "required": [],
    },
}


def _aphrodite_reclassify_handler(args=None, **kwargs):
    """Retroactively enrich all CCR entries with structured metadata.

    For each entry in _recent_markers that lacks a non-empty ``meta`` dict,
    retrieve the original content and run ``_classify_content`` on it, then
    write the result to the entry's ``meta`` field.

    Non-destructive: only adds the ``meta`` field, never removes or alters
    existing fields. Skips entries where content cannot be retrieved.

    Returns JSON with classification stats.
    """
    import time

    args = args if isinstance(args, dict) else {}
    from ._marker import _classify_content
    from ._resolve import _resolve_one
    target_hash = args.get("hash", "").strip()
    action = args.get("action", "all")

    # Build candidates: either a single hash or all entries lacking meta
    if target_hash:
        candidates = [m for m in _recent_markers if m.get("hash") == target_hash]
        if not candidates:
            return json.dumps({"error": f"hash not found: {target_hash}", "reclassified": 0})
    elif action == "all":
        candidates = [m for m in _recent_markers if not m.get("meta") or m["meta"] == {}]
    else:
        return json.dumps({"error": f"unknown action: {action}", "reclassified": 0})

    if not candidates:
        return json.dumps({"reclassified": 0, "type_distribution": {}, "note": "all entries already have metadata"})

    type_counts = {}
    reclassified = 0
    skipped_no_content = 0
    errors = 0
    t0 = time.time()

    for m in candidates:
        h = m.get("hash", "")
        if not h:
            continue
        if m.get("meta") and m["meta"] != {}:
            continue

        # Retrieve content - inline store first, then proxy
        content = None
        h_bare = h[2:] if h.startswith("i:") else h
        if h_bare in _inline_store:
            content = _inline_store[h_bare]
        else:
            with contextlib.suppress(Exception):
                content = _resolve_one(h, timeout=2)

        if content is None:
            skipped_no_content += 1
            continue

        try:
            klass = _classify_content(content)
            if not m.get("meta") or m["meta"] == {}:
                m["meta"] = klass
            ctype = klass.get("type", "text")
            type_counts[ctype] = type_counts.get(ctype, 0) + 1
            reclassified += 1
        except Exception:
            errors += 1
            continue

    elapsed = time.time() - t0
    total_with_meta = sum(1 for m in _recent_markers if m.get("meta") and m["meta"] != {})

    return json.dumps(
        {
            "reclassified": reclassified,
            "skipped_no_content": skipped_no_content,
            "errors": errors,
            "elapsed_ms": round(elapsed * 1000, 1),
            "total_with_meta": total_with_meta,
            "total_entries": len(_recent_markers),
            "type_distribution": dict(sorted(type_counts.items())),
            "note": f"{reclassified} entries enriched with retroactive metadata",
        },
        indent=2,
    )


def _prefetch_handler(args=None, **kwargs):
    """Background file read + compress — returns CCR markers instantly."""
    import threading

    args = args if isinstance(args, dict) else {}
    paths_raw = args.get("paths", args.get("path", ""))
    if isinstance(paths_raw, str):
        paths = [p.strip() for p in paths_raw.split(",") if p.strip()]
    elif isinstance(paths_raw, list):
        paths = [str(p).strip() for p in paths_raw if str(p).strip()]
    else:
        return json.dumps({"error": "paths required — string or list of file paths"})

    if not paths:
        return json.dumps({"error": "no valid paths provided"})

    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    markers = []
    lock = threading.Lock()

    def _read_and_compress(path: str):
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
        except FileNotFoundError:
            with lock: markers.append({"path": path, "error": "file not found"})
            return
        except PermissionError:
            with lock: markers.append({"path": path, "error": "permission denied"})
            return
        except Exception as e:
            with lock: markers.append({"path": path, "error": str(e)[:100]})
            return

        size = len(content)
        klass = _classify_content(content)
        preview = _make_ccr_preview(content, klass=klass, model_family=_detect_model_family())

        if proxy_available:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            ccr = _compress_via_proxy(content, target, headers=_headroom_context or None)
            if ccr:
                h, _ = ccr
                _inline_store_put(h, content)
                with lock:
                    _recent_markers.append({
                        "hash": h, "type": klass.get("type", "text"),
                        "size": size, "preview": preview,
                        "turn": _state.get("turn_counter", 0),
                        "meta": {"path": path},
                    })
                    markers.append({"hash": h, "path": path, "type": klass.get("type"), "size": size})
                return
        try:
            from ._inline import _inline_compress
            h, _ = _inline_compress(content)
            _inline_store_put(h, content)
            with lock:
                _recent_markers.append({
                    "hash": h, "type": klass.get("type", "text"),
                    "size": size, "preview": preview,
                    "turn": _state.get("turn_counter", 0),
                    "meta": {"path": path},
                })
                markers.append({"hash": h, "path": path, "type": klass.get("type"), "size": size})
        except Exception as e:
            with lock: markers.append({"path": path, "error": f"compress failed: {e}"})

    for path in paths:
        threading.Thread(target=_read_and_compress, args=(path,), daemon=True).start()

    return json.dumps({
        "prefetching": len(paths),
        "markers": markers,
        "note": "Files loading in background. Use TOC to check, aphrodite_retrieve(hash) to fetch.",
    }, indent=2)


PREFETCH_SCHEMA = {
    "name": "aphrodite_prefetch",
    "description": "Read files in background and compress to CCR. Returns markers instantly — agent continues while files load. Use aphrodite_retrieve(hash) when content is needed. Essential for parallelizing large reads.",
    "parameters": {
        "type": "object",
        "properties": {
            "paths": {
                "type": "array",
                "items": {"type": "string"},
                "description": "List of file paths to prefetch (or single string path)",
            }
        },
        "required": ["paths"],
    },
}


# ── Prefetch registry (live schedule for background file loads) ─

_prefetch_registry: dict = {}  # {path: {status, eta_s, hash, size, error}}


def _prefetch_status_handler(args=None, **kwargs):
    """Return the live prefetch schedule — what's loading, what's ready, ETAs."""
    if not _prefetch_registry:
        return "No active prefetches."

    pending = [(p, r) for p, r in _prefetch_registry.items() if r.get("status") == "loading"]
    ready = [(p, r) for p, r in _prefetch_registry.items() if r.get("status") == "ready"]
    errors = [(p, r) for p, r in _prefetch_registry.items() if r.get("status") == "error"]

    lines = [f"Prefetch schedule: {len(ready)} ready, {len(pending)} loading, {len(errors)} errors"]
    lines.append("")
    lines.append("| Status  | Path                          | Size    | ETA    | Hash      |")
    lines.append("|---------|-------------------------------|---------|--------|-----------|")

    for path, r in ready:
        lines.append(f"| READY   | {path[:40]:<40} | {r.get('size', 0):>6}B | {r.get('elapsed_s', 0):>4.1f}s | {r.get('hash', '')[:10]:<10} |")
    for path, r in pending:
        lines.append(f"| LOADING | {path[:40]:<40} | {r.get('size', 0):>6}B | {r.get('eta_s', 0):>4.1f}s | —          |")
    for path, r in errors:
        lines.append(f"| ERROR   | {path[:40]:<40} | —       | —      | {str(r.get('error', '?'))[:30]:<30} |")

    lines.append("")
    lines.append("READY = retrieve now. LOADING = ETA is estimated, poll again.")
    return "\n".join(lines)


PREFETCH_STATUS_SCHEMA = {
    "name": "aphrodite_prefetch_status",
    "description": "Live prefetch schedule — what's loading, what's ready, ETAs per file. Use to plan retrievals without polling blindly.",
    "parameters": {"type": "object", "properties": {}},
}




