"""
headroom-retrieve v0.2.0 — Hermes plugin.  Registers headroom_retrieve with
proxy (/v1/retrieve) + local file-read fallback.  Detects re-compressed
(CCR-marker) responses from the proxy and falls back automatically.
"""
from __future__ import annotations

import json
import os
import re
import urllib.request
import urllib.error

PROXY_URL = "http://127.0.0.1:8788"

# If content matches this, the proxy returned a CCR marker instead of real data
_CCR_LIKE = re.compile(r"<<ccr:|\[.*?items compressed.*?hash=")


def _normalize_hash(raw: str) -> str:
    h = raw.strip("<>").removeprefix("ccr:").removeprefix("hash=")
    return h.split(",")[0].strip()


def _looks_like_ccr(content: str) -> bool:
    """True if the content is just another CCR marker instead of real text."""
    if not content or not content.strip():
        return False
    stripped = content.strip()
    if stripped.startswith("<<ccr:") or stripped.startswith("[") and "compressed" in stripped:
        return True
    return bool(_CCR_LIKE.search(stripped[:200]))


HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind a headroom compression marker. "
        "Markers look like '[N items compressed ... hash=abc123]' or "
        "'<<ccr:abc,base64,4.5KB>>'. Extract just the hash. "
        "ALWAYS include the `path` parameter — the original file path — "
        "so the tool can read the file directly when the proxy cache expires."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "Hash from marker (e.g. 'abc123')"},
            "path": {"type": "string", "description": "Original file path for fallback read (REQUIRED)"},
            "query": {"type": "string", "description": "Optional BM25 search query"},
        },
        "required": ["hash", "path"],
    },
}


def _call_proxy(payload: dict) -> dict | None:
    """Try the proxy; return parsed result or None on any failure."""
    try:
        import httpx
        resp = httpx.post(f"{PROXY_URL}/v1/retrieve", json=payload, timeout=5)
        if resp.status_code == 404:
            return None
        if resp.status_code == 200:
            data = resp.json()
            content = data.get("original_content", "")
            if _looks_like_ccr(content):
                return None  # proxy returned a CCR marker, not real content
            return {
                "original_content": content,
                "original_tokens": data.get("original_tokens"),
                "source": "proxy",
            }
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
        if _looks_like_ccr(content):
            return None
        return {
            "original_content": content,
            "original_tokens": data.get("original_tokens"),
            "source": "proxy",
        }
    except Exception:
        return None


def _read_file_fallback(path: str) -> str | None:
    """Read a file directly."""
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
        lines = content.split("\n")
        return "\n".join(f"{i+1}|{line}" for i, line in enumerate(lines))
    except Exception:
        return None


def _handle_headroom_retrieve(args: dict, **kwargs) -> str:
    hash_key = _normalize_hash(str(args.get("hash") or "").strip())
    file_path = str(args.get("path") or "").strip()

    if not hash_key:
        return json.dumps({"error": "hash required"})
    if not file_path:
        return json.dumps({"error": "path required — always include the original file path"})

    payload: dict = {"hash": hash_key}
    query = str(args.get("query") or "").strip()
    if query:
        payload["query"] = query

    # 1. Try proxy
    result = _call_proxy(payload)
    if result and result.get("original_content"):
        return json.dumps(result)

    # 2. Fallback: read file directly
    content = _read_file_fallback(file_path)
    if content:
        return json.dumps({
            "original_content": content,
            "source": "fallback (local read)",
        })

    return json.dumps({
        "error": f"CCR expired and file not found at '{file_path}'. Re-run read_file.",
    })


def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
