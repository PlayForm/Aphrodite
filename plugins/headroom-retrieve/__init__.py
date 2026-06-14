"""
headroom-retrieve v0.3.0 — Hermes plugin.
- Multi-hash batch retrieval (pass an array of hashes)
- Auto-extract hash from raw CCR strings like '<<ccr:abc,string,5KB>>'
- Local file read first when path provided
- Self-caching (hash→content remembered within session)
- Proxy fallback for terminal/execute_code output
"""
from __future__ import annotations

import json
import os
import re
import urllib.request
import urllib.error

PROXY_URL = "http://127.0.0.1:8788"

# Session-local cache — survives across calls within same Hermes session
_CACHE: dict[str, str] = {}

_CCR_EXTRACT = re.compile(r"(?:<<ccr:|hash[=:\s]*)([a-f0-9]{6,64})", re.I)


def _extract_hash(raw: str) -> str:
    """Extract hash from a raw CCR marker string like '<<ccr:abc123,string,5KB>>'."""
    m = _CCR_EXTRACT.search(raw)
    return m.group(1) if m else raw.strip("<>").split(",")[0].strip()


HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind headroom compression markers. "
        "Pass markers directly as `hash` — the tool extracts the hash automatically. "
        "For files: include `path` for instant disk read. "
        "For terminal/execute_code output: pass the hash and the tool "
        "retrieves from the proxy cache. "
        "Can batch multiple markers: pass comma-separated hashes."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": (
                    "One or more CCR markers or raw hashes. Can be a single marker "
                    "like '<<ccr:abc,string,5KB>>' or comma-separated like "
                    "'hash1,hash2,hash3'. The tool auto-extracts hashes."
                ),
            },
            "path": {
                "type": "string",
                "description": "File path for local disk read (fastest). Can be comma-separated for batches.",
            },
            "query": {"type": "string", "description": "Optional BM25 search query"},
        },
        "required": ["hash"],
    },
}


def _read_file_direct(path: str) -> str | None:
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
        lines = content.split("\n")
        return "\n".join(f"{i+1}|{line}" for i, line in enumerate(lines))
    except Exception:
        return None


def _call_proxy(hash_key: str, query: str = "") -> str | None:
    """Try proxy retrieve for a single hash."""
    if hash_key in _CACHE:
        return _CACHE[hash_key]

    payload: dict = {"hash": hash_key}
    if query:
        payload["query"] = query

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

    # Reject content that is itself a CCR marker
    if content:
        stripped = content.strip()
        if stripped.startswith("<<ccr:") or (
            stripped.startswith("[") and "compressed" in stripped[:200]
        ):
            content = None

    if content:
        _CACHE[hash_key] = content
    return content


def _handle_headroom_retrieve(args: dict, **kwargs) -> str:
    raw = str(args.get("hash") or "").strip()
    if not raw:
        return json.dumps({"error": "hash required"})

    # Parse multiple hashes (comma-separated or raw CCR markers)
    raw_hashes: list[str] = []
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        h = _extract_hash(part)
        if h:
            raw_hashes.append(h)

    if not raw_hashes:
        return json.dumps({"error": "no valid hash found in input"})

    # Parse paths (comma-separated)
    path_str = str(args.get("path") or "").strip()
    paths: list[str] = [p.strip() for p in path_str.split(",") if p.strip()] if path_str else []

    query = str(args.get("query") or "").strip()

    results: list[dict] = []

    for i, h in enumerate(raw_hashes):
        entry: dict = {"hash": h}

        # 1. Cache hit?
        if h in _CACHE:
            entry["original_content"] = _CACHE[h]
            entry["source"] = "cache"
            results.append(entry)
            continue

        # 2. Local file read if path available
        file_path = paths[i] if i < len(paths) else None
        if file_path:
            content = _read_file_direct(file_path)
            if content:
                _CACHE[h] = content
                entry["original_content"] = content
                entry["source"] = "local"
                results.append(entry)
                continue

        # 3. Proxy
        content = _call_proxy(h, query)
        if content:
            entry["original_content"] = content
            entry["source"] = "proxy"
            results.append(entry)
            continue

        # 4. Failed
        hint = f" (path={file_path})" if file_path else ""
        entry["error"] = f"CCR expired{hint}. Re-run original command."
        results.append(entry)

    # Single hash → return flat (backward compat)
    if len(results) == 1:
        r = results[0]
        if "error" in r:
            return json.dumps(r)
        return json.dumps({
            "original_content": r["original_content"],
            "source": r.get("source", "unknown"),
        })

    # Multi-hash → return array
    return json.dumps({"results": results})


def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
