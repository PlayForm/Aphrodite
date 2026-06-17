"""aphrodite — session management: instruction injection, pre-LLM hook, turn storage."""

import json
import logging
import os
import urllib.request

from .._core import (
    _CCR_RE,
    _DEV,
    AUTO_EXPAND_LIMIT,
    CATALOG_MODE,
    PLUGIN_VERSION,
    PORTS,
    _conv_index,
    _fmt_size,
    _increment_turn,
    _inline_store,
    _recent_markers,
    _referenced_files,
    _render_prompt_tmpl,
    _scanned_msg_idx,
)
from .._marker import _classify_content, _parse_ccr_markers
from .._proxy import (
    _alive,
    _alive_cache,
    _alive_cached,
    _alive_turn_cache,
    _headroom_context,
    _query_and_set_headroom_budget,
    _update_headroom_context,
)
from .._resolve import _resolve_one
from .catalog import _build_catalog_parts
from .session_helpers import _group_into_turns

_log = logging.getLogger("aphrodite.hooks.session")
_last_user_msg = ""
_catalog_injected_this_turn: bool = False
_session_instruction_injected: bool = False


def _inject_session_instruction(conversation_history):
    token_alive = _alive_cached(PORTS["token"])
    lines = [f"💋 aphrodite v{PLUGIN_VERSION} active."]
    if token_alive:
        lines.append(f"  Token proxy :9798 active | tools auto-expand inline (<{_fmt_size(AUTO_EXPAND_LIMIT)})")
    else:
        lines.append("  Token proxy :9798 offline | inline fallback active")
    lines.append(_render_prompt_tmpl("session_inject"))
    lines.append("  ─ Layer 2: per-turn catalog injected below each turn ─")
    lines.append("  ─ Layer 3: load aphrodite-tool-guide skill for full tool reference ─")
    conversation_history.append({"role": "system", "content": "\n".join(lines), "ephemeral": True})
    _log.info("injected session instruction v%s", PLUGIN_VERSION)
    global _session_instruction_injected
    _session_instruction_injected = True


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
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
        top = sorted(exts.items(), key=lambda x: x[1], reverse=True)[:3]
        summary += " [" + " ".join(f"{e}({n})" for e, n in top) + "]"
    try:
        data = json.dumps({"turn": tnum, "user": last_user, "assistant": str(assistant_response)[:4096]}).encode()
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
    if _DEV or not conversation_history or not isinstance(conversation_history, list):
        return
    quiet_mode = os.environ.get("QUIET", "") == "1"
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
    if target and proxy_available:
        _query_and_set_headroom_budget(target)
    if not _session_instruction_injected:
        _inject_session_instruction(conversation_history)
    headers = kwargs.get("headers")
    if headers:
        _update_headroom_context(dict(headers))

    markers, total_bytes = [], 0
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

    for m in _recent_markers:
        if m.get("meta") and m["meta"] != {}:
            continue
        h = m.get("hash", "")
        if not h:
            continue
        h_bare = h[2:] if h.startswith("i:") else h
        content = _inline_store.get(h_bare)
        if content is not None:
            import contextlib
            with contextlib.suppress(Exception):
                m["meta"] = _classify_content(content)

    _expanded_hashes: set = set()
    if AUTO_EXPAND_LIMIT > 0:
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
    if _expanded_hashes:
        markers = [m for m in markers if m["hash"] not in _expanded_hashes]
    if CATALOG_MODE == "tool" and not markers:
        return None

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
                        req = urllib.request.Request(f"http://127.0.0.1:{target}/ccr/create", data=data, headers=archive_headers)
                        with urllib.request.urlopen(req, timeout=3) as r:
                            ccr = json.loads(r.read())
                        kept = len(turns) - len(old_turns)
                        compress_hint = (f"  [TURN ARCHIVE] CCR:{ccr['hash']} | "
                                         f"turns T{turns[0]['id']}-T{old_turns[-1]['id']} "
                                         f"({len(old_turns)} turns compressed, last {kept} raw)\n")
                        for t in old_turns:
                            _conv_index[t["id"]] = (ccr["hash"], f"turn {t['id']}", 0)
                except Exception as exc:
                    _log.debug("pre_llm_hook turn archive: %s", exc)

    user_msg = user_message or ""
    if not user_msg and isinstance(conversation_history, list):
        for msg in reversed(conversation_history):
            if msg.get("role") == "user":
                user_msg = str(msg.get("content", ""))[:200].lower()
                break
    parts = _build_catalog_parts(
        markers, total_bytes, _expanded_hashes, compress_hint,
        proxy_available, token_alive, cache_alive, target, ctx_len, quiet_mode, user_msg,
    )
    if quiet_mode:
        return None
    if parts:
        if _catalog_injected_this_turn:
            return None
        conversation_history.append({"role": "system", "content": "\n".join(parts), "ephemeral": True})
        _catalog_injected_this_turn = True
    return None
