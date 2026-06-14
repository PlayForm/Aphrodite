"""
headroom-retrieve v0.4.0 — Hermes plugin.
Local disk read when path given; proxy fallback otherwise.
"""
from __future__ import annotations

import json, os, urllib.request

PROXY = "http://127.0.0.1:8788"


def _hash(raw: str) -> str:
    for sep in ("<<ccr:", "hash=", "hash:"):
        if sep in raw:
            return raw.split(sep, 1)[1].split(",")[0].split(">")[0].strip()
    return raw.strip("<>").split(",")[0].strip()


HEADROOM_RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind compression markers like "
        "'<<ccr:abc,string,5KB>>' or '[N items compressed...]'. "
        "Include `path` for instant local-file read. "
        "Without path: tries proxy cache (may be expired)."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker or raw hash"},
            "path": {"type": "string", "description": "File path for local disk read"},
        },
        "required": ["hash"],
    },
}


def _read_file(path: str, limit: int = 0) -> str | None:
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        total = len(lines)
        if limit and total > limit:
            lines = lines[:limit]
        out = "".join(f"{i+1}|{line}" for i, line in enumerate(lines))
        if limit and total > limit:
            out += f"\n... [{total - limit} more lines]"
        return out
    except Exception:
        return None


def _proxy(hash_key: str) -> str | None:
    try:
        req = urllib.request.Request(
            f"{PROXY}/v1/retrieve",
            data=json.dumps({"hash": hash_key}).encode(),
            headers={"Content-Type": "application/json"},
        )
        data = json.loads(urllib.request.urlopen(req, timeout=5).read())
        content = data.get("original_content", "")
        if not content:
            return None
        # Reject CCR re-compression
        head = content.strip()[:200]
        if head.startswith("<<ccr:") or ("compressed" in head and head.startswith("[")):
            return None
        return content
    except Exception:
        return None


def _handle_headroom_retrieve(args: dict, **kwargs) -> str:
    h = _hash(str(args.get("hash", "")))
    if not h:
        return json.dumps({"error": "no hash found"})

    path = str(args.get("path", "")).strip()

    # Path → local read (always wins)
    if path:
        content = _read_file(path)
        if content:
            return json.dumps({"content": content, "source": "local"})
        content = _proxy(h)
        if content:
            return json.dumps({"content": content, "source": "proxy"})
        return json.dumps({"error": f"file not found: {path}"})

    # No path → proxy only
    content = _proxy(h)
    if content:
        return json.dumps({"content": content, "source": "proxy"})
    return json.dumps({"error": "Content expired. Re-run original command."})


def register(ctx) -> None:
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="headroom",
        schema=HEADROOM_RETRIEVE_SCHEMA,
        handler=_handle_headroom_retrieve,
        emoji="🗜️",
    )
