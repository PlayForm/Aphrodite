"""
headroom-retrieve v0.3.1 — Hermes plugin.
- Single or batch: pass one hash or comma-separated — DeepSeek chooses
- Local disk read first when path provided (instant, bypasses all compression)
- Proxy fallback for terminal/execute_code output
- Self-caching per session
- Rich metadata in responses (path, size, line count, source)
"""
from __future__ import annotations

import json
import os
import re
import urllib.request
import urllib.error

PROXY_URL = "http://127.0.0.1:8788"
_CACHE: dict[str, str] = {}
_CCR_EXTRACT = re.compile(r"(?:<<ccr:|hash[=:\s]*)([a-f0-9]{6,64})", re.I)


def _extract_hash(raw: str) -> str:
    m = _CCR_EXTRACT.search(raw)
    return m.group(1) if m else raw.strip("<>").split(",")[0].strip()


HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind headroom compression markers. "
        "Pass markers directly — the tool auto-extracts hashes. "
        "For FILES: include `path` for instant local disk read (fastest). "
        "For TERMINAL/CODE output: omit `path` and the tool tries the proxy cache. "
        "One hash or many (comma-separated) — your choice."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": "CCR marker(s) or hash(es). Single or comma-separated.",
            },
            "path": {
                "type": "string",
                "description": "File path(s) for local read. Comma-separated for batch.",
            },
        },
        "required": ["hash"],
    },
}


def _read_file_direct(path: str, max_lines: int = 0) -> dict | None:
    """Read file from disk. Returns {content, lines, size, path} or None."""
    if not path or not os.path.isfile(path):
        return None
    try:
        size = os.path.getsize(path)
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
        all_lines = content.split("\n")
        total_lines = len(all_lines)
        if max_lines and total_lines > max_lines:
            shown = all_lines[:max_lines]
            content = "\n".join(f"{i+1}|{line}" for i, line in enumerate(shown))
            content += f"\n... [{total_lines - max_lines} more lines, {size} bytes total]"
        else:
            content = "\n".join(f"{i+1}|{line}" for i, line in enumerate(all_lines))
        return {"content": content, "lines": total_lines, "size": size, "path": path}
    except Exception:
        return None


def _call_proxy(hash_key: str) -> str | None:
    """Try proxy retrieve. Returns content string or None."""
    if hash_key in _CACHE:
        return _CACHE[hash_key]

    payload = {"hash": hash_key}
    content = None

    try:
        import httpx
        resp = httpx.post(f"{PROXY_URL}/v1/retrieve", json=payload, timeout=5)
        if resp.status_code == 200:
            data = resp.json()
            content = data.get("original_content", "")
    except Exception:
        pass

    if content is None:
        try:
            req = urllib.request.Request(
                f"{PROXY_URL}/v1/retrieve",
                data=json.dumps(payload).encode(),
                headers={"Content-Type": "application/json"},
            )
            data = json.loads(urllib.request.urlopen(req, timeout=5).read())
            content = data.get("original_content", "")
        except Exception:
            pass

    if content:
        stripped = content.strip()
        if stripped.startswith("<<ccr:") or (
            stripped.startswith("[") and "compressed" in stripped[:200]
        ):
            return None
        _CACHE[hash_key] = content

    return content


def _handle_headroom_retrieve(args: dict, **kwargs) -> str:
    raw = str(args.get("hash") or "").strip()
    if not raw:
        return json.dumps({"error": "hash required"})

    raw_hashes = [h for part in raw.split(",") if (h := _extract_hash(part.strip()))]
    if not raw_hashes:
        return json.dumps({"error": "no valid hash found"})

    path_str = str(args.get("path") or "").strip()
    paths = [p.strip() for p in path_str.split(",") if p.strip()] if path_str else []

    results = []
    for i, h in enumerate(raw_hashes):
        file_path = paths[i] if i < len(paths) else None

        # 1. Cache hit
        if h in _CACHE:
            results.append({"hash": h, "content": _CACHE[h], "source": "cache"})
            continue

        # 2. Local file read (fastest, most reliable)
        if file_path:
            info = _read_file_direct(file_path)
            if info:
                _CACHE[h] = info["content"]
                results.append({
                    "hash": h,
                    "content": info["content"],
                    "source": "local",
                    "path": info["path"],
                    "lines": info["lines"],
                    "bytes": info["size"],
                })
                continue

        # 3. Proxy
        content = _call_proxy(h)
        if content:
            results.append({"hash": h, "content": content, "source": "proxy"})
            continue

        # 4. Failed
        hint = f' (path={file_path})' if file_path else ''
        msg = (
            f"Content not cached{hint}. "
            + ("File not found on disk — check path." if file_path else
               "Re-run the original terminal/execute_code command to get fresh output.")
        )
        results.append({"hash": h, "error": msg})

    # Single result → return flat
    if len(results) == 1:
        r = results[0]
        return json.dumps(r)

    # Batch → return array with summary
    ok = sum(1 for r in results if "content" in r)
    fail = len(results) - ok
    return json.dumps({
        "summary": f"{ok} retrieved, {fail} failed",
        "results": results,
    })


def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
