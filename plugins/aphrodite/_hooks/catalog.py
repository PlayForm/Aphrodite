"""aphrodite — compression catalog handler and table-of-contents builder."""

import json
import logging
import os

from .._automation import _auto_build_watch, _auto_commit_reminder
from .._core import (
    _DEV,
    CATALOG_MODE,
    CONTEXT_ENGINE,
    DEBUG_LOGGING,
    ENGINE_MIN_MSGS,
    ENGINE_PROTECT_FIRST,
    ENGINE_PROTECT_LAST,
    ENGINE_THRESHOLD_PCT,
    PLUGIN_VERSION,
    PORTS,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    _conv_index,
    _fmt_size,
    _inline_store,
    _recent_markers,
    _referenced_files,
    _render_prompt_tmpl,
)
from .._engine import get_engine
from .._inline import _inline_retrieve
from .._proxy import _alive_turn_cache, _expand_guidance
from .git import _git_summary
from .session_helpers import _READ_KEYWORDS

_log = logging.getLogger("aphrodite.hooks.catalog")


def _fmt_catalog(data: dict) -> str:
    items = data.get("items", [])
    total_saved = data.get("total_saved", 0)
    conv_turns = data.get("conv_turns", 0)
    ref_files = data.get("referenced_files", 0)
    saved_str = f"{total_saved / 1024:.1f}KB" if total_saved >= 1024 else f"{total_saved}B"
    lines = [f"Catalog: {len(items)} items {saved_str} saved {conv_turns} turns {ref_files} files"]
    if items:
        by_type = data.get("by_type", {})
        if by_type:
            lines.append(" ".join(f"{t}({v['count']})" for t, v in sorted(by_type.items())))
        lines.extend(["", "| Hash | Type | Size | Preview |", "|------|------|------|---------|"])
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


def _catalog_handler(args=None, **kwargs):
    args = args if isinstance(args, dict) else {}
    if args.get("mode") == "toc":
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
        "items": items, "conv_turns": len(_conv_index), "referenced_files": len(_referenced_files),
    }
    return json.dumps(result, indent=2)


CATALOG_SCHEMA = {
    "name": "aphrodite_catalog",
    "description": "Return full compression catalog with hashes, sizes, types, previews. "
    "Mode 'toc' for compact table-of-contents with Retrieve? recommendations. "
    "Use toc BEFORE retrieving to avoid wasted round-trips.",
    "parameters": {"type": "object", "properties": {
        "mode": {"type": "string", "description": "Optional: 'toc' for compact table-of-contents, default full catalog"},
    }},
}


def _build_toc() -> str:
    markers = list(_recent_markers)
    if not markers:
        return "Catalog: 0 items"
    lines = [
        f"Catalog: {len(markers)} items, {sum(m['size'] for m in markers)}B saved", "",
        "| Hash    | Type           | Size  | Preview                          | Retrieve? |",
        "|---------|----------------|-------|----------------------------------|-----------|",
    ]
    for m in reversed(markers[-20:]):
        h = m["hash"][:12]
        t = m["type"][:14]
        s = _fmt_size(m["size"])
        p = (m.get("preview", "") or "")[:45].replace("|", "/")
        retrieve = "YES"
        if t in ("build_output", "build_error") and "0e" in p.lower() and "0w" in p.lower() or t == "terminal" and "exit=0" in p or t in ("grep", "search_files", "search_results") and ("0 matches" in p or "0m" in p) or t not in ("build_output", "build_error", "terminal") and "0E 0W" in p:
            retrieve = "NO"
        lines.append(f"| {h:<7} | {t:<14} | {s:>5} | {p:<45} | {retrieve:<9} |")
    lines.extend(["", "Retrieve? = NO means the preview is sufficient — skip retrieval."])
    return "\n".join(lines)


def _build_catalog_parts(markers, total_bytes, expanded_hashes, compress_hint,
                         proxy_available, token_alive, cache_alive, target, ctx_len,
                         quiet_mode, user_message):
    parts = []
    if not (markers or _conv_index or compress_hint or len(_referenced_files) > 5 or DEBUG_LOGGING or _expand_guidance):
        return parts if parts else None
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
    up_ports = [str(port) for _, port in PORTS.items() if _alive_turn_cache.get(port)]
    auto_parts.append(f"proxy: {','.join(up_ports)} up" if up_ports else "proxy: none ⚠")
    if auto_parts:
        parts.append("  [AUTO] " + " | ".join(auto_parts))
    if DEBUG_LOGGING or CATALOG_MODE == "full":
        parts.append(f"  ⚙ v{PLUGIN_VERSION} | engine={'on' if CONTEXT_ENGINE else 'off'} | dev={'on' if _DEV else 'off'}")
        parts.append(f"  ⚙ thresholds: term={TERMINAL_THRESHOLD} inline=-- "
                     f"tool_tok={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE} "
                     f"engine_pct={ENGINE_THRESHOLD_PCT}% prot={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min={ENGINE_MIN_MSGS}")
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
                tp = " ".join(f"{len(items)} [{ctype}]" for ctype, items in sorted(by_type.items()))
                parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved) - {tp}")
            else:
                parts.append(f"  {len(markers)} items ({_fmt_size(total_bytes)} saved)")
        else:
            parts.append(f"  mode={mode} | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
    elif CATALOG_MODE != "tool":
        parts.append(f"  mode=inline | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
    if markers:
        parts.append("  ⚡ Tool outputs auto-expand before you see them - full content is inline. "
                     "Context/terminal markers require aphrodite_retrieve(hash) to fetch.")
    if markers or len(expanded_hashes) > 0:
        parts.extend([
            f"  [{len(markers)} markers available | {len(expanded_hashes)} tool outputs auto-expanded this turn]",
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
        seen, deduped = set(), []
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
            for m in items[:visible]:
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
        words = set(user_message.lower().split()) if user_message else set()
        if words & _READ_KEYWORDS and markers:
            recent_markers = markers[-3:]
            hashes = " ".join(m['hash'][:12] for m in recent_markers)
            parts.append(f"  intent=read | recent CCRs: {hashes}")
    return parts if parts else None
