"""aphrodite — session management: instruction injection, pre-LLM hook, turn storage."""

import json
import logging
import os
import time
import urllib.request

from .._core import (
    _CCR_RE,
    _DEV,
    AUTO_EXPAND_LIMIT,
    CATALOG_MODE,
    CONTEXT_ENGINE,
    DEBUG_LOGGING,
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    INLINE_THRESHOLD,
    PLUGIN_VERSION,
    PORTS,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _conv_index,
    _detect_model_family,
    _fmt_size,
    _increment_turn,
    _inline_store,
    _recent_markers,
    _referenced_files,
    _render_prompt_tmpl,
    _scanned_msg_idx,
    _state,
)
from .._engine import get_engine
from .._inline import _inline_retrieve
from .._marker import _classify_content, _make_ccr_preview, _parse_ccr_markers
from .._proxy import (
    _alive,
    _alive_cache,
    _alive_cached,
    _alive_turn_cache,
    _expand_guidance,
    _headroom_context,
    _query_and_set_headroom_budget,
    _update_headroom_context,
)
from .._resolve import _resolve_one
from .._automation import _auto_build_watch, _auto_commit_reminder
from .git import _git_summary

_log = logging.getLogger("aphrodite.hooks.session")

# Module-level state
_last_user_msg = ""
_catalog_injected_this_turn: bool = False
_session_instruction_injected: bool = False

_READ_KEYWORDS: frozenset = frozenset({
    "read", "show", "view", "get", "cat", "display", "retrieve",
    "fetch", "look", "see", "open", "inspect", "check", "print",
    "dump", "output",
})


def _inject_session_instruction(conversation_history):
    """Inject ephemeral system message with aphrodite version + proxy info (once per session)."""
    token_alive = _alive_cached(PORTS["token"])
    threshold = ENGINE_THRESHOLD_PCT
    thresh_str = "disabled (0)" if threshold == 0 else "always (-1)" if threshold == -1 else f"{threshold}%"
    lines = [f"💋 aphrodite v{PLUGIN_VERSION} active."]
    if token_alive:
        lines.append(
            f"  Token proxy :9798 active | engine threshold={thresh_str} | "
            f"tools auto-expand inline (<{_fmt_size(AUTO_EXPAND_LIMIT)})"
        )
    else:
        lines.append(
            f"  Token proxy :9798 offline | inline fallback active | "
            f"engine threshold={thresh_str}"
        )
    lines.append(_render_prompt_tmpl("session_inject"))
    lines.append("  ─ Layer 2: per-turn catalog injected below each turn ─")
    lines.append("  ─ Layer 3: load aphrodite-tool-guide skill for full tool reference ─")
    conversation_history.append({"role": "system", "content": "\n".join(lines), "ephemeral": True})
    _log.info("injected session instruction v%s", PLUGIN_VERSION)
    global _session_instruction_injected
    _session_instruction_injected = True


def _group_into_turns(conversation_history):
    """Group messages into turns (user → assistant → tools)."""
    turns, current, turn_num = [], None, 0
    for msg in conversation_history:
        role, content = msg.get("role", ""), msg.get("content", "")
        if role == "user":
            if current:
                turns.append(current)
            turn_num += 1
            current = {"id": turn_num, "user": str(content)[:1000]}
        elif role == "assistant" and current:
            current["assistant"] = str(content)[:1000]
        elif role == "tool" and current:
            raw = str(content)[:200] if content else ""
            if raw:
                current.setdefault("tools", []).append(raw)
    if current:
        turns.append(current)
    return turns


def _extract_preview(marker, conversation_history):
    """Extract a short preview for a CCR marker from conversation history (fallback)."""
    h = marker["hash"]
    for msg in conversation_history:
        c = msg.get("content", "")
        if isinstance(c, str) and h in c:
            idx = c.find(h)
            after = c[idx + len(h):].strip()
            if ">>>" in after:
                after = after.split(">>>", 1)[-1].strip()
            return after[:80].strip()
    return ""


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    if not conversation_history or assistant_response is None or _DEV:
        return
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]
    tnum = _increment_turn()
    last_user = _last_user_msg
    summary = f"T{tnum}: {last_user}… → {str(assistant_response)[:200]}"
    if _referenced_files:
        exts = {}
        for path in list(_referenced_files)[-10:]:
            ext = os.path.splitext(path)[1] or "noext"
            exts[ext] = exts.get(ext, 0) + 1
        top_exts = sorted(exts.items(), key=lambda x: x[1], reverse=True)[:3]
        summary += " [" + " ".join(f"{ext}({n})" for ext, n in top_exts) + "]"

    try:
        data = json.dumps({
            "turn": tnum,
            "user": last_user,
            "assistant": str(assistant_response)[:4096],
        }).encode()
        store_headers = {"Content-Type": "application/octet-stream"}
        if _headroom_context:
            store_headers.update(_headroom_context)
        req = urllib.request.Request(f"http://127.0.0.1:{target}/ccr/create", data=data, headers=store_headers)
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())
        if len(_conv_index) >= 100:
            del _conv_index[next(iter(_conv_index))]
        _conv_index[tnum] = (ccr["hash"], summary, len(str(assistant_response)[:4096]))
        _log.debug("conv-cache: stored T%d → %s (%d total)", tnum, ccr["hash"], len(_conv_index))
    except Exception as exc:
        _log.debug("_store_conversation_turn: %s", exc)


def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
    """Before LLM call: build navigable compression catalog."""
    if _DEV or not conversation_history or not isinstance(conversation_history, list):
        return
    quiet_mode = os.environ.get("QUIET", "") == "1"
    if quiet_mode and DEBUG_LOGGING:
        _log.debug("pre_llm_hook: quiet_mode=1, catalog injection suppressed")

    global _scanned_msg_idx, _last_user_msg, _catalog_injected_this_turn
    _alive_cache.clear()
    _alive_turn_cache.clear()
    _alive_turn_cache[PORTS["token"]] = _alive(PORTS["token"])
    _alive_turn_cache[PORTS["cache"]] = _alive(PORTS["cache"])
    _last_user_msg = user_message or ""
    _catalog_injected_this_turn = False

    token_alive = _alive_cached(PORTS["token"])
    cache_alive = _alive_cached(PORTS["cache"])
    proxy_available = token_alive or cache_alive
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None
    ctx_len = len(conversation_history)

    # Headroom feedback + session instruction + header pass-through
    if target and proxy_available:
        _query_and_set_headroom_budget(target)
    if not _session_instruction_injected:
        _inject_session_instruction(conversation_history)
    headroom_hdrs = {}
    headers = kwargs.get("headers")
    if headers:
        _update_headroom_context(dict(headers))
        for k, v in headers.items():
            kl = k.lower()
            if kl.startswith("x-headroom-") and kl != "x-headroom-bypass":
                headroom_hdrs[k] = v

    # Scan for CCR markers (incremental)
    markers = []
    total_bytes = 0
    start_idx = max(0, _scanned_msg_idx)
    for msg in conversation_history[start_idx:]:
        role = msg.get("role", "")
        if role not in ("tool", "system"):
            continue
        content = msg.get("content", "")
        if isinstance(content, str) and "CCR:" in content:
            for m in _parse_ccr_markers(content):
                total_bytes += m["size"]
                markers.append(m)
    _scanned_msg_idx = ctx_len
    seen_hashes = {m["hash"] for m in markers if "hash" in m}
    for old_m in _recent_markers:
        if old_m.get("hash") not in seen_hashes:
            markers.append(old_m)
            seen_hashes.add(old_m["hash"])
    current_hashes = {m["hash"] for m in _recent_markers}
    if seen_hashes != current_hashes:
        for m in markers:
            if m["hash"] not in current_hashes:
                _recent_markers.append(m)

    # Auto-classify entries lacking metadata
    classified_this_turn = 0
    for m in _recent_markers:
        if m.get("meta") and m["meta"] != {}:
            continue
        h = m.get("hash", "")
        if not h:
            continue
        h_bare = h[2:] if h.startswith("i:") else h
        content = _inline_store.get(h_bare)
        if content is not None:
            try:
                m["meta"] = _classify_content(content)
                classified_this_turn += 1
            except Exception:
                if DEBUG_LOGGING:
                    _log.debug("pre_llm_hook: auto-classify failed for %s", h[:12])

    # Auto-expand small tool CCR markers
    _expanded_hashes: set = set()
    if AUTO_EXPAND_LIMIT > 0:
        expanded_count = 0
        for msg in conversation_history:
            role = msg.get("role", "")
            if role not in ("tool", "system"):
                continue
            content = msg.get("content", "")
            if not isinstance(content, str) or "CCR:" not in content:
                continue
            replacements = {}
            for match in _CCR_RE.finditer(content):
                full_marker = match.group(0)
                h = match.group(1)
                inner = full_marker.split("CCR:", 1)[1]
                for suffix in (">>>", "]", "⫸"):
                    if inner.endswith(suffix):
                        inner = inner[:-len(suffix)]
                        break
                parts = inner.split("|")
                if len(parts) < 3 or str(parts[1]) != "aphrodite":
                    continue
                try:
                    marker_size = int(parts[2])
                except ValueError:
                    continue
                if marker_size >= AUTO_EXPAND_LIMIT:
                    continue
                resolved = _resolve_one(h, timeout=2)
                if resolved is not None and len(resolved) < AUTO_EXPAND_LIMIT:
                    replacements[full_marker] = resolved
                    _expanded_hashes.add(h)
            if replacements:
                new_content = content
                for old, new in replacements.items():
                    new_content = new_content.replace(old, new, 1)
                msg["content"] = new_content
                expanded_count += len(replacements)

    if _expanded_hashes:
        markers = [m for m in markers if m["hash"] not in _expanded_hashes]

    if CATALOG_MODE == "tool" and not markers:
        return None

    # Compress old turns to CCR
    compress_hint = ""
    if proxy_available and target and ctx_len > 30:
        turns = _group_into_turns(conversation_history)
        if len(turns) > 6:
            old_turns = [t for t in turns[:-6] if t["id"] not in _conv_index]
            if old_turns:
                try:
                    summaries = [{"turn": t["id"], "user": t.get("user", "")[:1000],
                                  "assistant": t.get("assistant", "(tool calls)")[:1000]} for t in old_turns]
                    packed = json.dumps(summaries)
                    if len(packed) > 500:
                        data = packed.encode()
                        archive_headers = {"Content-Type": "application/octet-stream"}
                        if _headroom_context:
                            archive_headers.update(_headroom_context)
                        req = urllib.request.Request(f"http://127.0.0.1:{target}/ccr/create",
                                                      data=data, headers=archive_headers)
                        with urllib.request.urlopen(req, timeout=3) as r:
                            ccr = json.loads(r.read())
                        kept = len(turns) - len(old_turns)
                        compress_hint = (
                            f"  [TURN ARCHIVE] CCR:{ccr['hash']} | "
                            f"turns T{turns[0]['id']}-T{old_turns[-1]['id']} "
                            f"({len(old_turns)} turns compressed, last {kept} raw)\n"
                        )
                        for t in old_turns:
                            _conv_index[t["id"]] = (ccr["hash"], f"turn {t['id']}", 0)
                except Exception as exc:
                    _log.debug("_pre_llm_hook turn archive: %s", exc)

    # Build catalog
    parts = []
    if markers or _conv_index or compress_hint or len(_referenced_files) > 5 or DEBUG_LOGGING or _expand_guidance:
        parts.append("💋")
        if _expand_guidance:
            parts.append(f"  {_expand_guidance}")

        auto_parts = []
        build_info = _auto_build_watch()
        if build_info:
            auto_parts.append(build_info.replace("  ", ""))
        commit_info = _auto_commit_reminder()
        if commit_info:
            auto_parts.append(commit_info.replace("  ", ""))
        up_ports = [str(port) for name, port in PORTS.items() if _alive_turn_cache.get(port)]
        auto_parts.append(f"proxy: {','.join(up_ports)} up" if up_ports else "proxy: none ⚠")
        if auto_parts:
            parts.append("  [AUTO] " + " | ".join(auto_parts))

        if DEBUG_LOGGING or CATALOG_MODE == "full":
            parts.append(f"  ⚙ v{PLUGIN_VERSION} | engine={'on' if CONTEXT_ENGINE else 'off'} | dev={'on' if _DEV else 'off'}")
            parts.append(f"  ⚙ thresholds: term={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} "
                         f"tool_tok={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE} "
                         f"engine_pct={ENGINE_THRESHOLD_PCT}% prot={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} "
                         f"min={ENGINE_MIN_MSGS}")

        if CATALOG_MODE != "tool":
            git_info = _git_summary()
            if git_info:
                parts.append(f"  git: {git_info}")

        if proxy_available:
            mode = "token" if token_alive else "cache"
            if CATALOG_MODE == "tool":
                parts.append(f"  {len(markers)} items compressed")
            elif CATALOG_MODE == "compact":
                by_type = {}
                for m in markers:
                    by_type.setdefault(m["type"], []).append(m)
                if by_type:
                    type_parts = " ".join(f"{len(items)} [{ctype}]" for ctype, items in sorted(by_type.items()))
                    parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved) - {type_parts}")
                else:
                    parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved)")
            else:
                parts.append(f"  mode={mode} | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
        elif CATALOG_MODE != "tool":
            parts.append(f"  mode=inline | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")

        if markers:
            parts.append("  ⚡ Tool outputs auto-expand before you see them - full content is inline. "
                         "Context/terminal markers require aphrodite_retrieve(hash) to fetch.")

        if markers or len(_expanded_hashes) > 0 or not _session_instruction_injected:
            parts.extend([
                f"  [{len(markers)} markers available | {len(_expanded_hashes)} tool outputs auto-expanded this turn]",
                "  Call aphrodite_catalog to list all entries, aphrodite_retrieve(hash) to fetch.",
                "  For full tool reference, load aphrodite-tool-guide skill (skill_view).",
            ])

        engine = get_engine()
        if engine and engine.compression_count > 0:
            parts.append(f"  engine: {engine.compression_count} compressions | last: "
                         f"{engine.last_compression.get('messages_compressed', '?')} msgs → "
                         f"CCR:{engine.last_compression.get('hash', '?')[:8]}")

        if compress_hint:
            parts.append(compress_hint)

        if CATALOG_MODE == "full" and markers:
            live = [m for m in markers if m["hash"] in _inline_store or _inline_retrieve(m["hash"])]
            if not live and markers:
                live = markers
            preview_cache = {}
            expanded = []
            for m in live:
                h = m.get("hash", "")
                if m.get("size", 0) < 10240 and h in _inline_store:
                    preview_cache[h] = _inline_store[h][:200].replace("\n", " ").strip()
                else:
                    preview_cache[h] = m.get("preview", "")
                expanded.append({**m, "preview": preview_cache[h]})
            live = expanded
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
                    h = str(m.get("hash", "")).strip()
                    if len(h) < 4 or h in ("{}", "?", "None", "null", "undefined"):
                        continue
                    meta = m.get("meta", {}) or {}
                    if meta:
                        kvs = ", ".join(f"{k}={v}" for k, v in sorted(meta.items()))
                        parts.append(f"      {h[:12]} - {m.get('type', '?')} [{kvs}] ({_fmt_size(m['size'])})")
                    else:
                        parts.append(f"      CCR:{h} | {_fmt_size(m['size'])} | {preview_cache.get(m['hash'], '')}")
                if len(items) > visible:
                    parts.append(f"      ... +{len(items) - visible} more")
            parts.append("  ⚡ Markers include structured metadata - use hints to decide retrieval.")

        if CATALOG_MODE == "full" and _conv_index:
            recent = sorted(_conv_index.items(), reverse=True)[:3]
            parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in recent))

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

        if CATALOG_MODE != "tool" and ctx_len > 20:
            if ctx_len > 100:
                parts.append(_render_prompt_tmpl("catalog_context_warn", {"ctx": ctx_len}))
            else:
                parts.append(f"  context={ctx_len} msgs")

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
                parts.append(f"  intent=read | recent CCRs: {hashes}")

    if quiet_mode:
        return None

    if parts:
        if _catalog_injected_this_turn:
            return None
        catalog = "\n".join(parts)
        conversation_history.append({"role": "system", "content": catalog, "ephemeral": True})
        _catalog_injected_this_turn = True
    return None
