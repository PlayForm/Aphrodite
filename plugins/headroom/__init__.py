"""
headroom — Hermes plugin: 3 MCP tools + transparent CCR resolution.

Tools:
  headroom_compress  — compress content on demand via proxy /v1/compress
  headroom_retrieve  — resolve CCR markers (local file + proxy + BM25)
  headroom_stats     — proxy compression statistics

Middleware (HERMES_HEADROOM_NATIVE=1):
  Transparent CCR resolution via llm_request middleware.
  Scans API messages for CCR markers and resolves them BEFORE the proxy
  sees them — breaking the token-proxy re-compression loop.
  Proxy-aware: skips inline compression when routing through headroom proxy.
  Applies inline compression when on direct API.

See patches/hermes-headroom-native.patch for the standalone Hermes agent module.
"""
import json, os, re, urllib.request

PROXY = "http://127.0.0.1:8788"
_CCR = re.compile(r"<<ccr:([a-f0-9]{6,64})")
_HASH = re.compile(r'hash[=:\s]*([a-f0-9]{6,64})', re.I)
_NATIVE = os.environ.get("HERMES_HEADROOM_NATIVE") == "1"

# ═══════════════════════════════════════
# Utilities
# ═══════════════════════════════════════

def _extract_hash(raw):
    for s in ("<<ccr:", "hash="):
        if s in raw: return raw.split(s, 1)[1].split(",")[0].rstrip(">")
    return raw.strip("<>").split(",")[0]

def _read_file(path):
    if not path or not os.path.isfile(path): return None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        return "".join(f"{i+1}|{l}" for i, l in enumerate(lines))
    except Exception: return None

def _proxy_post(endpoint, payload, timeout=5):
    try:
        req = urllib.request.Request(f"{PROXY}{endpoint}",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"})
        return json.loads(urllib.request.urlopen(req, timeout=timeout).read())
    except Exception: return None

# ═══════════════════════════════════════
# headroom_compress
# ═══════════════════════════════════════

COMPRESS_SCHEMA = {
    "name": "headroom_compress",
    "description": "Compress text on demand via headroom proxy. Returns compressed version + hash for later retrieval.",
    "parameters": {
        "type": "object",
        "properties": {"content": {"type": "string", "description": "Text to compress"}},
        "required": ["content"],
    },
}

def _handle_compress(args, **kw):
    content = str(args.get("content", ""))
    if not content: return json.dumps({"error": "content required"})
    data = _proxy_post("/v1/compress", {"messages": [{"role": "user", "content": content}]}, timeout=10)
    if not data: return json.dumps({"error": "compress unavailable"})
    return json.dumps({
        "compressed": data.get("compressed_messages", [{}])[0].get("content", content),
        "hash": data.get("hash", ""),
        "tokens_before": data.get("tokens_before"),
        "tokens_after": data.get("tokens_after"),
        "savings_percent": data.get("savings_percent"),
    })

# ═══════════════════════════════════════
# headroom_retrieve
# ═══════════════════════════════════════

RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": "Resolve CCR markers. Include `path` for local file read (fastest). Include `query` for BM25 search.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker or raw hash"},
            "path": {"type": "string", "description": "File path for local disk read"},
            "query": {"type": "string", "description": "BM25 search within retrieved content"},
        },
        "required": ["hash"],
    },
}

def _handle_retrieve(args, **kw):
    h = _extract_hash(str(args.get("hash", "")))
    if not h: return json.dumps({"error": "no hash"})
    p = str(args.get("path", "")).strip()
    q = str(args.get("query", "")).strip()

    if p:
        c = _read_file(p)
        if c: return json.dumps({"original_content": c, "source": "local"})

    data = _proxy_post("/v1/retrieve", {"hash": h, "query": q} if q else {"hash": h})
    if data:
        c = data.get("original_content", "")
        if c and not c.lstrip()[:6] == "<<ccr:":
            return json.dumps({"original_content": c, "source": "proxy" + (" (BM25)" if q else "")})

    return json.dumps({"error": "expired"})

# ═══════════════════════════════════════
# headroom_stats
# ═══════════════════════════════════════

STATS_SCHEMA = {
    "name": "headroom_stats",
    "description": "Proxy compression statistics: requests, compression rate, tokens saved.",
    "parameters": {"type": "object", "properties": {}},
}

def _handle_stats(args, **kw):
    try:
        req = urllib.request.Request(f"{PROXY}/stats")
        s = json.loads(urllib.request.urlopen(req, timeout=5).read()).get("summary", {})
        return json.dumps({
            "mode": s.get("mode"), "requests": s.get("api_requests"),
            "compressed": s.get("compression", {}).get("requests_compressed", 0),
            "avg_compression_pct": s.get("compression", {}).get("avg_compression_pct", 0),
            "tokens_removed": s.get("compression", {}).get("total_tokens_removed", 0),
            "prefix_frozen": s.get("uncompressed_requests", {}).get("prefix_frozen", 0),
        })
    except Exception as e:
        return json.dumps({"error": f"stats unavailable: {e}"})

# ═══════════════════════════════════════
# Native middleware (HERMES_HEADROOM_NATIVE=1)
# ═══════════════════════════════════════

def _is_proxy(base_url):
    return "127.0.0.1" in str(base_url) or "localhost" in str(base_url)

_COMPRESS_ENGINE = None

def _compress_inline(messages):
    global _COMPRESS_ENGINE
    if _COMPRESS_ENGINE is None:
        try:
            from hermes_compress import Compress, CompressOption
            opt = CompressOption()
            opt.Enabled = True; opt.Mode = "inline"; opt.ProtectRecent = 1
            opt.MinTokensToCompress = 100; opt.TargetRatio = None
            opt.PrecompressTools = True; opt.AggressiveKompress = True
            opt.DeduplicateResults = True
            _COMPRESS_ENGINE = Compress(model="deepseek-v4-pro", option=opt)
        except Exception: _COMPRESS_ENGINE = False
    if not _COMPRESS_ENGINE or _COMPRESS_ENGINE is False: return messages
    try: return _COMPRESS_ENGINE.compress(messages).messages
    except Exception: return messages

def _resolve_ccr_in_messages(messages):
    resolved = 0
    for i, msg in enumerate(messages):
        content = str(msg.get("content", ""))
        if not content.strip(): continue
        hashes = _CCR.findall(content)
        if not hashes and "items compressed" in content[:500]:
            m = _HASH.search(content[:500])
            if m: hashes = [m.group(1)]
        if not hashes: continue
        for h in hashes:
            data = _proxy_post("/v1/retrieve", {"hash": h})
            if data:
                c = data.get("original_content", "")
                if c and not c.lstrip()[:6] == "<<ccr:":
                    messages[i] = {**msg, "content": c}
                    resolved += 1
                    break
    return resolved

def _on_llm_request(**kwargs):
    if not _NATIVE: return None
    request = dict(kwargs.get("request", {}))
    msgs = request.get("messages")
    if not msgs: return None

    using_proxy = _is_proxy(kwargs.get("base_url", ""))

    # Always resolve CCR markers (prevents re-compression loop)
    _resolve_ccr_in_messages(msgs)

    # If direct API, compress inline for extra savings
    if not using_proxy and len(msgs) > 2:
        msgs = _compress_inline(msgs)

    request["messages"] = msgs
    return {"request": request, "source": "headroom-native"}

# ═══════════════════════════════════════
# Register
# ═══════════════════════════════════════

def register(ctx):
    ctx.register_tool(name="headroom_compress", toolset="headroom",
                      schema=COMPRESS_SCHEMA, handler=_handle_compress, emoji="🗜️")
    ctx.register_tool(name="headroom_retrieve", toolset="headroom",
                      schema=RETRIEVE_SCHEMA, handler=_handle_retrieve, emoji="🗜️")
    ctx.register_tool(name="headroom_stats", toolset="headroom",
                      schema=STATS_SCHEMA, handler=_handle_stats, emoji="📊")
    if _NATIVE:
        ctx.register_middleware("llm_request", _on_llm_request)
