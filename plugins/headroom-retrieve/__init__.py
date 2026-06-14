"""
headroom-retrieve v0.2.1 — Hermes plugin.
Proxy (/v1/retrieve) + local file-read.  Local read is ALWAYS preferred
when a path is provided — the proxy is used only as fallback.
"""
from __future__ import annotations

import json
import os
import urllib.request
import urllib.error

PROXY_URL = "http://127.0.0.1:8788"


def _normalize_hash(raw: str) -> str:
    h = raw.strip("<>").removeprefix("ccr:").removeprefix("hash=")
    return h.split(",")[0].strip()


HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind a headroom compression marker. "
        "Markers look like '[N items compressed ... hash=abc123]' or "
        "'<<ccr:abc,base64,4.5KB>>'. Extract just the hash. "
        "ALWAYS include the `path` parameter — the tool reads the file "
        "directly from disk, bypassing any compression layer entirely."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "Hash from marker (e.g. 'abc123')"},
            "path": {"type": "string", "description": "Original file path — tool reads directly from disk"},
            "query": {"type": "string", "description": "Optional BM25 search query"},
        },
        "required": ["hash", "path"],
    },
}


def _read_file_direct(path: str) -> str | None:
    """Read a file directly from disk — the reliable path."""
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
        lines = content.split("\n")
        return "\n".join(f"{i+1}|{line}" for i, line in enumerate(lines))
    except Exception:
        return None


def _call_proxy(payload: dict) -> str | None:
    """Try proxy retrieve.  Returns content string or None."""
    try:
        import httpx
        resp = httpx.post(f"{PROXY_URL}/v1/retrieve", json=payload, timeout=5)
        if resp.status_code == 200:
            data = resp.json()
            content = data.get("original_content", "")
            # Only accept content that doesn't look like a CCR marker
            stripped = content.strip() if content else ""
            if stripped and not stripped.startswith("<<ccr:") and not (
                stripped.startswith("[") and "compressed" in stripped[:200]
            ):
                return content
        return None
    except Exception:
        pass

    try:
        req = urllib.request.Request(
            f"{PROXY_URL}/v1/retrieve",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        data = json.loads(urllib.request.urlopen(req, timeout=5).read())
        content = data.get("original_content", "")
        stripped = content.strip() if content else ""
        if stripped and not stripped.startswith("<<ccr:") and not (
            stripped.startswith("[") and "compressed" in stripped[:200]
        ):
            return content
        return None
    except Exception:
        return None


def _handle_headroom_retrieve(args: dict, **kwargs) -> str:
    hash_key = _normalize_hash(str(args.get("hash") or "").strip())
    file_path = str(args.get("path") or "").strip()

    if not hash_key:
        return json.dumps({"error": "hash required"})
    if not file_path:
        return json.dumps({"error": "path required — always include the original file path"})

    # 1. LOCAL READ FIRST — always reliable
    content = _read_file_direct(file_path)
    if content:
        return json.dumps({
            "original_content": content,
            "source": "local (direct disk read)",
        })

    # 2. PROXY FALLBACK — only if local read failed
    payload: dict = {"hash": hash_key}
    query = str(args.get("query") or "").strip()
    if query:
        payload["query"] = query
    proxy_content = _call_proxy(payload)
    if proxy_content:
        return json.dumps({
            "original_content": proxy_content,
            "source": "proxy",
        })

    return json.dumps({
        "error": f"File not found at '{file_path}' and CCR cache expired. Re-run read_file.",
    })


def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
