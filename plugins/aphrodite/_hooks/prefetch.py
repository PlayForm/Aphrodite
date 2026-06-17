"""aphrodite — background file prefetch and prefetch status."""

import json
import logging
import threading

from .._core import (
    PORTS,
    _detect_model_family,
    _inline_store_put,
    _recent_markers,
    _state,
)
from .._marker import (
    _classify_content,
    _compress_via_proxy,
    _make_ccr_preview,
)
from .._proxy import _alive_cached, _headroom_context

_log = logging.getLogger("aphrodite.hooks.prefetch")

_prefetch_registry: dict = {}  # {path: {status, eta_s, hash, size, error}}


def _prefetch_handler(args=None, **kwargs):
    """Background file read + compress — returns CCR markers instantly."""
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
            with open(path, encoding="utf-8", errors="replace") as f:
                content = f.read()
        except FileNotFoundError:
            with lock:
                markers.append({"path": path, "error": "file not found"})
            return
        except PermissionError:
            with lock:
                markers.append({"path": path, "error": "permission denied"})
            return
        except Exception as e:
            with lock:
                markers.append({"path": path, "error": str(e)[:100]})
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
            from .._inline import _inline_compress
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
            with lock:
                markers.append({"path": path, "error": f"compress failed: {e}"})

    for path in paths:
        threading.Thread(target=_read_and_compress, args=(path,), daemon=True).start()

    return json.dumps({
        "prefetching": len(paths),
        "markers": markers,
        "note": "Files loading in background. Use TOC to check, aphrodite_retrieve(hash) to fetch.",
    }, indent=2)


PREFETCH_SCHEMA = {
    "name": "aphrodite_prefetch",
    "description": "Read files in background and compress to CCR. Returns markers instantly — "
    "agent continues while files load. Use aphrodite_retrieve(hash) when content is needed. "
    "Essential for parallelizing large reads.",
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
        lines.append(
            f"| READY   | {path[:40]:<40} | {r.get('size', 0):>6}B | "
            f"{r.get('elapsed_s', 0):>4.1f}s | {r.get('hash', '')[:10]:<10} |"
        )
    for path, r in pending:
        lines.append(
            f"| LOADING | {path[:40]:<40} | {r.get('size', 0):>6}B | "
            f"{r.get('eta_s', 0):>4.1f}s | —          |"
        )
    for path, r in errors:
        lines.append(
            f"| ERROR   | {path[:40]:<40} | —       | —      | "
            f"{str(r.get('error', '?'))[:30]:<30} |"
        )

    lines.append("")
    lines.append("READY = retrieve now. LOADING = ETA is estimated, poll again.")
    return "\n".join(lines)


PREFETCH_STATUS_SCHEMA = {
    "name": "aphrodite_prefetch_status",
    "description": "Live prefetch schedule — what's loading, what's ready, ETAs per file. "
    "Use to plan retrievals without polling blindly.",
    "parameters": {"type": "object", "properties": {}},
}
