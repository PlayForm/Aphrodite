"""aphrodite — hook handlers for Hermes tool/terminal/LLM calls."""

import hashlib
import json
import logging
import os
import subprocess
import time
import urllib.request

from ._binary import _ensure_binary
from ._core import (
    _CCR_RE,
    _DEV,
    _FILE_TOOLS,
    BINARY,
    CATALOG_MODE,
    DEBUG_LOGGING,
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    INLINE_THRESHOLD,
    PLUGIN_VERSION,
    PORTS,
    RECURSIVE_DEPTH,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _conv_index,
    _fmt_size,
    _git_cache,
    _inline_store,
    _recent_markers,
    _referenced_files,
    _turn_counter,
)
from ._engine import AphroditeContextEngine, get_engine
from ._inline import _inline_compress, _inline_retrieve
from ._marker import _ccr_marker, _compress_via_proxy
from ._proxy import _alive, on_start
from ._tools import COMPRESS_SCHEMA, RETRIEVE_SCHEMA, _compress_handler, _retrieve_handler

# These are defined within this module (extracted from original monolithic file)
# _track_file_refs, _fmt_size, _extract_preview, _group_into_turns are defined below
# _referenced_files, _recent_markers, _conv_index, _FILE_TOOLS, _git_cache are module-level

_log = logging.getLogger("aphrodite")

# ── Hooks ─────────────────────────────────────────────────────


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

    if _DEV:
        return result  # dev mode: passthrough
    # Track file references for aphrodite_files tool
    _track_file_refs(tool_name, args)
    token_alive = _alive(9798)
    cache_alive = _alive(9797)
    proxy_available = token_alive or cache_alive

    # Essential tools: never compress - agent needs immediate access to skills, memory, session history
    _ESSENTIAL_TOOLS = {
        "skill_view",
        "skills_list",
        "skill_manage",
        "memory",
        "session_search",
        "read_file",
        "read_terminal",
    }
    skip = (
        _ESSENTIAL_TOOLS | {"aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"}
        if token_alive
        else _ESSENTIAL_TOOLS
        | {
            "execute_code",
            "patch",
            "write_file",
            "search_files",
            "todo",
            "aphrodite_retrieve",
            "aphrodite_compress",
            "aphrodite_stats",
        }
    )
    if tool_name in skip:
        if DEBUG_LOGGING:
            _log.debug(
                "transform_tool_result: SKIP %s %.1fms (in skip list)", tool_name[:40], (time.time() - _t0) * 1000
            )
        return result

    threshold = TOOL_THRESHOLD_TOKEN if token_alive else TOOL_THRESHOLD_CACHE if cache_alive else INLINE_THRESHOLD
    result_len = len(result)
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

    preview = result[:120].replace("\\n", " ").strip()

    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(result, target)
        if ccr:
            h, sz = ccr
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
            _recent_markers.append({"hash": h, "type": "tool", "size": result_len, "preview": preview})
            if len(_recent_markers) > 200:
                _recent_markers.pop(0)
            _inline_store[h] = result  # mirror for aphrodite_search
            return _ccr_marker(h, "tool", result_len, label, preview)
        elif DEBUG_LOGGING:
            _log.debug("transform_tool_result: PROXY FAIL %s - proxy returned no hash", tool_name[:40])

    # Fallback: inline compression (works without proxy)
    if result_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(result)
            if DEBUG_LOGGING:
                _log.debug(
                    "transform_tool_result: INLINE %s hash=%s size=%s %.1fms",
                    tool_name[:40],
                    h,
                    result_len,
                    (time.time() - _t0) * 1000,
                )
            _recent_markers.append({"hash": h, "type": "tool", "size": result_len, "preview": preview})
            if len(_recent_markers) > 200:
                _recent_markers.pop(0)
            return _ccr_marker(h, "tool", result_len, "inline", preview)
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
    """Rebuild aphrodite crate and copy binary to ~/.hermes/aphrodite/."""
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
    if os.path.exists(src):
        import shutil

        shutil.copy2(src, BINARY)
        os.chmod(BINARY, 0o755)
        return f'{{"ok": true, "size": {os.path.getsize(BINARY)}, "path": "{BINARY}"}}'
    return '{"error": "binary not found after build"}'


REBUILD_SCHEMA = {
    "name": "aphrodite_rebuild",
    "description": "Rebuild aphrodite crate from source and install binary. Use after code changes.",
    "parameters": {"type": "object", "properties": {}},
}


# ── Conversation Memory via CCR ─────────────────────────────────────


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    global _turn_counter
    if not conversation_history or assistant_response is None:
        return

    if _DEV:
        return
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]
    _turn_counter += 1
    tnum = _turn_counter

    # Capture the last user message from conversation history
    last_user = ""
    for msg in reversed(conversation_history):
        if msg.get("role") == "user":
            last_user = msg.get("content", "")[:200]
            break

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
        data = json.dumps(
            {
                "content": json.dumps(
                    {
                        "turn": tnum,
                        "user": last_user,
                        "assistant": str(assistant_response)[:5000],
                    }
                )
            }
        ).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create", data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())

        _conv_index[tnum] = (ccr["hash"], summary, len(str(assistant_response)))
        if len(_conv_index) > 100:
            oldest = min(_conv_index.keys())
            del _conv_index[oldest]

        _log.debug("conv-cache: stored T%d → %s (%d total)", tnum, ccr["hash"], len(_conv_index))
    except Exception:
        pass


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|mode>>> markers from text. Returns list of dicts."""
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
                pass
    # Filter out entries with missing/empty hashes
    # Filter: real CCR hashes are hex (0-9,a-f), ≥8 chars. Placeholders like abc123 filtered.
    return [
        m
        for m in markers
        if m["hash"] and len(m["hash"]) >= 8 and all(c in "0123456789abcdef" for c in m["hash"].lower())
    ]


def _git_summary():
    """Get cached git diff --stat summary. Returns string or None."""
    now = time.time()
    if _git_cache.get("ts", 0) > now - 30:
        return _git_cache.get("summary")
    try:
        import subprocess

        r = subprocess.run(["git", "diff", "--stat"], capture_output=True, text=True, timeout=3)
        if r.returncode == 0 and r.stdout.strip():
            summary = r.stdout.strip().split("\n")[-1] if r.stdout.strip() else None
            _git_cache["ts"] = now
            _git_cache["summary"] = summary
            return summary
    except Exception:
        pass
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

    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None
    ctx_len = len(conversation_history)

    # ── 1. Scan for CCR markers (injected by transform hooks) ──
    markers = []
    total_bytes = 0
    for msg in conversation_history:
        content = msg.get("content", "")
        if isinstance(content, str):
            for m in _parse_ccr_markers(content):
                total_bytes += m["size"]
                markers.append(m)
    global _recent_markers
    _recent_markers.clear()
    _recent_markers.extend(markers)  # cache for aphrodite_search
    if DEBUG_LOGGING and markers:
        _log.debug(
            "pre_llm_hook: scanned %d CCR markers across %d msgs, %s total compressed",
            len(markers),
            ctx_len,
            _fmt_size(total_bytes),
        )

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
                                "user": t.get("user", "")[:300],
                                "assistant": t.get("assistant", "(tool calls)")[:300],
                            }
                        )
                    packed = json.dumps(summaries)
                    if len(packed) > 500:
                        data = json.dumps({"content": packed}).encode()
                        req = urllib.request.Request(
                            f"http://127.0.0.1:{target}/ccr/create",
                            data=data,
                            headers={"Content-Type": "application/json"},
                        )
                        with urllib.request.urlopen(req, timeout=3) as r:
                            ccr = json.loads(r.read())
                        kept = len(turns) - len(old_turns)
                        compress_hint = (
                            f"  [TURN ARCHIVE] CCR:{ccr['hash']} | "
                            f"turns T{turns[0]['id']}-T{old_turns[-1]['id']} "
                            f"({len(old_turns)} turns compressed, last {kept} raw)\n"
                            f"  retrieve: aphrodite_retrieve({ccr['hash']})"
                        )
                except Exception:
                    pass

    # ── 3. Build the catalog (mode-aware) ─────────────────────
    parts = []
    if markers or _conv_index or compress_hint or len(_referenced_files) > 5 or DEBUG_LOGGING:
        parts.append("[APHRODITE]")

        # Debug banner (only in DEBUG mode or full catalog)
        if DEBUG_LOGGING or CATALOG_MODE == "full":
            parts.append(
                f"  ⚙ v{PLUGIN_VERSION} | mode={'proxy+hooks' if not os.environ.get('APHRODITE_CONTEXT_ENGINE') else 'proxy+hooks+engine'} | engine={'enabled' if os.environ.get('APHRODITE_CONTEXT_ENGINE') == '1' else 'off'} | dev={'on' if _DEV else 'off'}"
            )
            parts.append(
                f"  ⚙ thresholds: term={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_tok={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE} engine_pct={ENGINE_THRESHOLD_PCT}% prot={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min={ENGINE_MIN_MSGS}"
            )

        # Git diff summary
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

            # Auto-expand: resolve small cached items inline - LLM never sees aphrodite_retrieve
            expanded = []
            for m in live:
                if m["size"] < 10240 and m["hash"] in _inline_store:
                    content = _inline_store[m["hash"]]
                    m["preview"] = content[:200].replace("\n", " ").strip()
                    m["auto_expanded"] = True
                expanded.append(m)
            live = expanded

            # Deduplicate by hash — keep first occurrence only
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
                    preview = _extract_preview(m, conversation_history)
                    h = str(m.get("hash", "")).strip()
                    if len(h) < 4 or h in ("{}", "?", "None", "null", "undefined"):
                        continue
                    parts.append(f"      CCR:{h} | {_fmt_size(m['size'])} | {preview}")
                if len(items) > visible:
                    parts.append(f"      ... +{len(items) - visible} more (use aphrodite_retrieve)")

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
                parts.append(f"  ⚠ context={ctx_len} msgs - prefer aphrodite_retrieve over scanning")
            else:
                parts.append(f"  context={ctx_len} msgs")

        # Read-intent detection (skip in tool mode)
        if CATALOG_MODE != "tool":
            READ_KEYWORDS = {
                "read",
                "show",
                "view",
                "get",
                "cat",
                "display",
                "retrieve",
                "fetch",
                "look",
                "see",
                "open",
                "inspect",
                "check",
                "print",
                "dump",
                "output",
            }
            last_user = user_message or ""
            if isinstance(conversation_history, list):
                for msg in reversed(conversation_history):
                    if msg.get("role") == "user":
                        last_user = str(msg.get("content", ""))[:200].lower()
                        break
            words = set(last_user.lower().split())
            has_read_intent = bool(words & READ_KEYWORDS)
            if has_read_intent and markers:
                recent_markers = markers[-3:]
                parts.append(
                    "  intent=read | recent CCRs available: "
                    + " ".join(f"aphrodite_retrieve({m['hash']})" for m in recent_markers)
                )

    if parts:
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
        return catalog


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
            # Tool results accumulate under the current turn
            pass
    if current:
        turns.append(current)
    return turns


def _extract_preview(marker, conversation_history):
    """Extract a short preview for a CCR marker from conversation history."""
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
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    out_len = len(output)
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
    if is_build and output.count("\n") > 20:
        lines = output.splitlines()
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
            ccr = _compress_via_proxy(output, target)
            if ccr:
                h, _ = ccr
                if DEBUG_LOGGING:
                    _log.debug("terminal_hook: BUILD-CCR %s:%s", "token" if token_alive else "cache", h)
                return f"<<<CCR:{h}|build|{len(output)}>>> {summary}…(use aphrodite_retrieve)"
        # Inline fallback
        h, _ = _inline_compress(output)
        return f"<<<CCR:{h}|build|{len(output)}|inline>>> {summary}…(use aphrodite_retrieve)"

    preview = output[:200].replace("\n", " ").strip()

    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(output, target)
        if ccr:
            h, _ = ccr
            if DEBUG_LOGGING:
                ratio = out_len / max(len(h), 1)
                _log.debug(
                    "terminal_hook: CCR %s:%s size=%s ratio=%.1fx",
                    "token" if token_alive else "cache",
                    h,
                    out_len,
                    ratio,
                )
            return f"<<<CCR:{h}|terminal|{out_len}>>> {preview}…(use aphrodite_retrieve)"
        elif DEBUG_LOGGING:
            _log.debug("terminal_hook: PROXY FAIL - returned no hash (cmd: %s)", command[:60])

    # Fallback: inline compression
    if out_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(output)
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE hash=%s size=%s", h, out_len)
            return f"<<<CCR:{h}|terminal|{out_len}|inline>>> {preview}…(use aphrodite_retrieve)"
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
            "total_bytes": sum(len(v) for v in _inline_store.values()),
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
    """Track file paths referenced by tool calls."""
    if tool_name not in _FILE_TOOLS:
        return
    args = args if isinstance(args, dict) else {}
    path = args.get("path", args.get("file", ""))
    if path and isinstance(path, str) and len(path) < 500:
        _referenced_files[path] = tool_name
        if len(_referenced_files) > 200:
            oldest = next(iter(_referenced_files))
            del _referenced_files[oldest]


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
    """Return full compression catalog: all items with hashes, sizes, types, previews.
    Use when catalog mode is 'tool' and you need detailed CCR information."""
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


CATALOG_SCHEMA = {
    "name": "aphrodite_catalog",
    "description": "Return full compression catalog - all CCR items with hashes, sizes, types, and previews. Use when you need detailed information about what has been compressed.",
    "parameters": {"type": "object", "properties": {}},
}


def _search_handler(args=None, **kwargs):
    """Search across compressed items by type or content pattern."""
    args = args if isinstance(args, dict) else {}
    query = args.get("query", "").lower()
    ccr_type = args.get("type", "")

    results = []
    # Search conversation turn index
    for tnum, (h, summary, size) in sorted(_conv_index.items(), reverse=True):
        if query and query not in summary.lower():
            continue
        results.append({"source": "turn", "turn": tnum, "hash": h, "summary": summary, "size": size})

    # Search inline store
    for h, content in _inline_store.items():
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
            "hint": "Use aphrodite_retrieve(hash) on any result hash to get full content.",
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
            "def foo"
            in _retrieve_handler(args={"hash": hashlib.sha256(b"def foo():\n    return 42\n").hexdigest()[:16]})
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
        results_path = os.path.join(os.path.dirname(__file__), ".test-results.json")
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


def register(ctx):
    # Install binary on registration
    _ensure_binary()
    ctx.register_hook("on_session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
    ctx.register_hook("post_llm_call", _store_conversation_turn)
    ctx.register_hook("transform_tool_result", _transform_tool_result)
    ctx.register_tool(
        name="aphrodite_rebuild",
        schema=REBUILD_SCHEMA,
        handler=_rebuild_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_compress",
        schema=COMPRESS_SCHEMA,
        handler=_compress_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_retrieve",
        schema=RETRIEVE_SCHEMA,
        handler=_retrieve_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_stats",
        schema=STATS_SCHEMA,
        handler=_stats_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_files",
        schema=FILES_SCHEMA,
        handler=_files_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_diff",
        schema=DIFF_SCHEMA,
        handler=_diff_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_search",
        schema=SEARCH_SCHEMA,
        handler=_search_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_test",
        schema=TEST_SCHEMA,
        handler=_test_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_catalog",
        schema=CATALOG_SCHEMA,
        handler=_catalog_handler,
        toolset="aphrodite",
    )
    # Only register context engine when explicitly configured
    engine_configured = os.environ.get("APHRODITE_CONTEXT_ENGINE", "") == "1"
    if engine_configured:
        try:
            ctx.register_context_engine(AphroditeContextEngine())
            _log.info("aphrodite context engine registered")
        except Exception as e:
            _log.debug("context engine registration skipped: %s", e)
    else:
        _log.info("context engine not registered - set APHRODITE_CONTEXT_ENGINE=1 to enable")
    _log.info("aphrodite v%s registered - %d tools + hooks", PLUGIN_VERSION, 9)

    # ── Debug banner: print configuration on startup ──────────
    if DEBUG_LOGGING:
        lines = [
            "=" * 60,
            f"APHRODITE v{PLUGIN_VERSION} - DEBUG MODE",
            f"  Mode: {'proxy+hooks' if not engine_configured else 'proxy+hooks+engine'} | Engine: {'enabled' if engine_configured else 'disabled'} | Dev: {'on' if _DEV else 'off'}",
            f"  Thresholds: terminal={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_token={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE}",
            f"  Engine: threshold={ENGINE_THRESHOLD_PCT}% protect={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min_msgs={ENGINE_MIN_MSGS}",
            f"  CCR: regex={_CCR_RE.pattern} depth={RECURSIVE_DEPTH}",
            "  Tools: retrieve, compress, stats, rebuild, files, diff, search, test, catalog",
            f"  Catalog mode: {CATALOG_MODE} (APHRODITE_CATALOG=full|compact|tool)",
            "  Proxies: cache=:9797 token=:9798 | waiting for session_start...",
            "=" * 60,
        ]
        for line in lines:
            print(line)
            _log.info(line)
