"""
headroom-tools — full MCP suite for Hermes: compress, retrieve, stats.
Uses the token-mode proxy (:8788) for compress/retrieve operations.
"""
import json, os, urllib.request

PROXY = "http://127.0.0.1:8788"

# ═══════════════════════════════════════════════
# headroom_compress — on-demand compression
# ═══════════════════════════════════════════════

COMPRESS_SCHEMA = {
    "name": "headroom_compress",
    "description": (
        "Compress content on demand. Shrinks large text (files, JSON, logs, search results) "
        "before reasoning over it. The original is stored and can be retrieved later via "
        "headroom_retrieve. Returns the compressed text and a hash key for retrieval."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Text to compress"},
        },
        "required": ["content"],
    },
}

def _handle_compress(args, **kw):
    content = str(args.get("content", ""))
    if not content:
        return json.dumps({"error": "content required"})

    # Use proxy's /v1/compress endpoint
    try:
        req = urllib.request.Request(
            f"{PROXY}/v1/compress",
            data=json.dumps({"messages": [{"role": "user", "content": content}]}).encode(),
            headers={"Content-Type": "application/json"},
        )
        data = json.loads(urllib.request.urlopen(req, timeout=10).read())
        return json.dumps({
            "compressed": data.get("compressed_messages", [{}])[0].get("content", content),
            "hash": data.get("hash", ""),
            "tokens_before": data.get("tokens_before"),
            "tokens_after": data.get("tokens_after"),
            "savings_percent": data.get("savings_percent"),
        })
    except Exception as e:
        return json.dumps({"error": f"compress failed: {e}. Use headroom_retrieve later if needed."})


# ═══════════════════════════════════════════════
# headroom_retrieve — with local + proxy + BM25
# ═══════════════════════════════════════════════

RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": (
        "Retrieve original content behind compression markers like "
        "'<<ccr:abc,string,5KB>>' or '[N items compressed...]'. "
        "Include `path` for instant local-file read (fastest). "
        "Include `query` for BM25 search within the retrieved content. "
        "Without path: tries proxy cache."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker or raw hash"},
            "path": {"type": "string", "description": "File path for local disk read"},
            "query": {"type": "string", "description": "BM25 search query within retrieved content"},
        },
        "required": ["hash"],
    },
}

def _hash(raw):
    for s in ("<<ccr:", "hash="):
        if s in raw:
            return raw.split(s, 1)[1].split(",")[0].rstrip(">")
    return raw.strip("<>").split(",")[0]

def _file(path):
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        return "".join(f"{i+1}|{l}" for i, l in enumerate(lines))
    except Exception:
        return None

def _proxy_retrieve(h, q=""):
    try:
        payload = {"hash": h}
        if q:
            payload["query"] = q
        req = urllib.request.Request(f"{PROXY}/v1/retrieve",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"})
        c = json.loads(urllib.request.urlopen(req, timeout=5).read()).get("original_content", "")
        if c and not c.lstrip()[:6] == "<<ccr:":
            return c
    except Exception:
        pass
    return None

def _handle_retrieve(args, **kw):
    h = _hash(str(args.get("hash", "")))
    if not h:
        return json.dumps({"error": "no hash"})
    p = str(args.get("path", "")).strip()
    q = str(args.get("query", "")).strip()

    # Local file read first
    if p:
        c = _file(p)
        if c:
            return json.dumps({"original_content": c, "source": "local"})

    # Proxy with optional BM25 query
    c = _proxy_retrieve(h, q)
    if c:
        return json.dumps({"original_content": c, "source": "proxy" + (" (BM25)" if q else "")})

    return json.dumps({"error": "expired — re-run command" if p else "expired"})


# ═══════════════════════════════════════════════
# headroom_stats — session compression statistics
# ═══════════════════════════════════════════════

STATS_SCHEMA = {
    "name": "headroom_stats",
    "description": (
        "Get compression statistics: tokens saved, compression ratio, "
        "proxy status, and recent compression events."
    ),
    "parameters": {
        "type": "object",
        "properties": {},
    },
}

def _handle_stats(args, **kw):
    try:
        req = urllib.request.Request(f"{PROXY}/stats")
        data = json.loads(urllib.request.urlopen(req, timeout=5).read())
        s = data.get("summary", {})
        return json.dumps({
            "mode": s.get("mode"),
            "requests": s.get("api_requests"),
            "compressed": s.get("compression", {}).get("requests_compressed", 0),
            "avg_compression_pct": s.get("compression", {}).get("avg_compression_pct", 0),
            "tokens_removed": s.get("compression", {}).get("total_tokens_removed", 0),
            "prefix_frozen": s.get("uncompressed_requests", {}).get("prefix_frozen", 0),
            "retrievals": s.get("mcp", {}).get("retrievals", 0),
        })
    except Exception as e:
        return json.dumps({"error": f"stats unavailable: {e}"})


# ═══════════════════════════════════════════════
# Native CCR resolution middleware (HERMES_HEADROOM_NATIVE=1)
# Resolves CCR markers in messages BEFORE the proxy sees them.
# ═══════════════════════════════════════════════

import re as _re

_CCR_PATTERN = _re.compile(r"<<ccr:([a-f0-9]{6,64})")
_NATIVE = os.environ.get("HERMES_HEADROOM_NATIVE", "") == "1"

def _resolve_ccr_in_messages(messages):
    """Scan messages for CCR markers, resolve them in-place."""
    if not messages or not isinstance(messages, list):
        return
    for i, msg in enumerate(messages):
        content = msg.get("content", "")
        if not isinstance(content, str) or not content.strip():
            continue
        # Find all CCR hashes in the content
        hashes = _CCR_PATTERN.findall(content)
        if not hashes:
            # Also check [N items compressed...] format
            if "items compressed" in content[:500]:
                # Try to extract hash from [N items compressed ... hash=abc123]
                import re as _re2
                m = _re2.search(r'hash[=:\s]*([a-f0-9]{6,64})', content[:500], _re2.I)
                if m:
                    hashes = [m.group(1)]
        if not hashes:
            continue
        # Resolve each hash — try local file path from context, then proxy
        resolved = None
        for h in hashes:
            # Try proxy first (fastest for non-file content)
            c = _proxy_retrieve(h)
            if c:
                resolved = c
                break
        if resolved:
            messages[i] = {**msg, "content": resolved}
    return messages


def _on_llm_request(**kwargs):
    """Middleware: resolve CCR markers before the API call goes through proxy."""
    if not _NATIVE:
        return None
    request = dict(kwargs.get("request", {}))
    msgs = request.get("messages")
    if msgs:
        _resolve_ccr_in_messages(msgs)
        request["messages"] = msgs
    return {"request": request, "source": "headroom-native"}


# ═══════════════════════════════════════════════
# Register all
# ═══════════════════════════════════════════════

def register(ctx):
    ctx.register_tool(name="headroom_compress", toolset="headroom",
                      schema=COMPRESS_SCHEMA, handler=_handle_compress, emoji="🗜️")
    ctx.register_tool(name="headroom_retrieve", toolset="headroom",
                      schema=RETRIEVE_SCHEMA, handler=_handle_retrieve, emoji="🗜️")
    ctx.register_tool(name="headroom_stats",   toolset="headroom",
                      schema=STATS_SCHEMA,   handler=_handle_stats,   emoji="📊")
    if _NATIVE:
        ctx.register_middleware("llm_request", _on_llm_request)
