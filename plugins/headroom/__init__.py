"""
headroom v1.0.1 - Hermes plugin: MCP tools + transparent CCR middleware.

Tools:
  headroom_compress     - compress content on demand via proxy /v1/compress
  headroom_retrieve     - resolve CCR markers (local file + proxy + BM25)
  headroom_stats        - proxy compression statistics
  headroom_proxy_start  - start headroom proxy
  headroom_proxy_stop   - stop headroom proxy

Middleware (HERMES_HEADROOM_NATIVE=1):
  llm_request hook scans API messages for <<ccr:HASH>> markers,
  resolves ALL of them via proxy /v1/retrieve, and returns a new
  message list.  Proxy-aware: skips inline compression when routing
  through a local headroom proxy; applies inline compression on
  direct API using the hermes_compress.Compress engine.
"""
import json, os, re, urllib.request

PROXY = "http://127.0.0.1:8788"
_CCR = re.compile(r"<<ccr:([a-f0-9]{1,64})[^>]*>>")
_HASH = re.compile(r'hash[=:\s]*([a-f0-9]{1,64})', re.I)
_NATIVE = os.environ.get("HERMES_HEADROOM_NATIVE", "1") != "0"
_MODEL = os.environ.get("HERMES_MODEL", "deepseek-v4-pro")

# ═══════════════════════════════════════
# Utilities
# ═══════════════════════════════════════

def _extract_hash(raw):
    for s in ("<<ccr:", "hash="):
        if s in raw: return raw.split(s, 1)[1].split(",")[0].rstrip(">]")
    return raw.strip("<>[]").split(",")[0]

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

def _is_proxy(base_url):
    u = str(base_url)
    return any(h in u for h in ("127.0.0.1", "localhost", "0.0.0.0"))

# ═══════════════════════════════════════
# headroom_compress
# ═══════════════════════════════════════

COMPRESS_SCHEMA = {
    "name": "headroom_compress",
    "description": "Compress text on demand via headroom proxy. Returns compressed version + hash.",
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
    "description": "Resolve CCR markers. Include `path` for local file read. Include `query` for BM25 search.",
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
# headroom_proxy_start / stop
# ═══════════════════════════════════════

PROXY_START_SCHEMA = {
    "name": "headroom_proxy_start",
    "description": "Start the headroom proxy server. Returns port and mode.",
    "parameters": {
        "type": "object",
        "properties": {
            "port": {"type": "integer", "description": "Port (default: 8787)"},
            "mode": {"type": "string", "description": "cache or token (default: cache)"},
        },
    },
}

def _handle_proxy_start(args, **kw):
    port = int(args.get("port", 8787))
    mode = str(args.get("mode", "cache"))
    try:
        from hermes_compress._compress import Proxy
        p = Proxy(port=port, mode=mode)
        p.start()
        return json.dumps({"status": "started", "port": port, "mode": mode, "base_url": p.base_url})
    except Exception as e:
        return json.dumps({"error": f"proxy start failed: {e}"})

PROXY_STOP_SCHEMA = {
    "name": "headroom_proxy_stop",
    "description": "Stop the headroom proxy server.",
    "parameters": {
        "type": "object",
        "properties": {
            "port": {"type": "integer", "description": "Port to stop (default: 8787)"},
        },
    },
}

def _handle_proxy_stop(args, **kw):
    port = int(args.get("port", 8787))
    try:
        from hermes_compress._compress import Proxy
        Proxy(port=port).stop()
        return json.dumps({"status": "stopped", "port": port})
    except Exception as e:
        return json.dumps({"error": f"proxy stop failed: {e}"})

# ═══════════════════════════════════════
# Native middleware (HERMES_HEADROOM_NATIVE=1)
# ═══════════════════════════════════════

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
            _COMPRESS_ENGINE = Compress(model=_MODEL, option=opt)
        except Exception: _COMPRESS_ENGINE = False
    if not _COMPRESS_ENGINE or _COMPRESS_ENGINE is False: return messages
    try: return _COMPRESS_ENGINE.compress(messages).messages
    except Exception: return messages

# Cache of resolved hashes to avoid redundant proxy calls
_RESOLVED: dict[str, str] = {}

def _resolve_one(hash_key):
    if hash_key in _RESOLVED:
        return _RESOLVED[hash_key]
    data = _proxy_post("/v1/retrieve", {"hash": hash_key})
    if data:
        c = data.get("original_content", "")
        if c and not c.lstrip()[:6] == "<<ccr:":
            _RESOLVED[hash_key] = c
            return c
    return None

def _resolve_ccr_in_messages(messages):
    """Return a NEW list with CCR markers resolved in all messages.
    Resolves ALL markers in each message (not just the first).
    Does not mutate the input list.
    """
    result = []
    resolved = 0
    for msg in messages:
        content = str(msg.get("content", ""))
        if not content.strip():
            result.append(msg)
            continue

        # Find all hashes in this message
        hashes = _CCR.findall(content)
        if not hashes and "items compressed" in content[:500]:
            m = _HASH.search(content[:500])
            if m: hashes = [m.group(1)]

        if not hashes:
            result.append(msg)
            continue

        # Format 1: <<ccr:HASH,...>>  — resolve all via re.sub
        def _replacer(m):
            nonlocal resolved
            h = m.group(1)
            c = _resolve_one(h)
            if c:
                resolved += 1
                return c
            return m.group(0)

        new_content = _CCR.sub(_replacer, content)

        # Format 2: [N items compressed ... hash=HASH] — replace whole content
        _hash_m = _HASH.search(new_content[:500])
        if _hash_m and "items compressed" in new_content[:500]:
            c = _resolve_one(_hash_m.group(1))
            if c:
                resolved += 1
                new_content = c

        result.append({**msg, "content": new_content})

    return result, resolved

def _on_llm_request(**kwargs):
    if not _NATIVE: return None
    request = dict(kwargs.get("request", {}))
    msgs = request.get("messages")
    if not msgs: return None

    using_proxy = _is_proxy(kwargs.get("base_url", ""))

    # Resolve CCR markers (returns new list, no mutation)
    msgs, _ = _resolve_ccr_in_messages(msgs)

    # If direct API, compress inline
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
    ctx.register_tool(name="headroom_proxy_start", toolset="headroom",
                      schema=PROXY_START_SCHEMA, handler=_handle_proxy_start, emoji="▶️")
    ctx.register_tool(name="headroom_proxy_stop", toolset="headroom",
                      schema=PROXY_STOP_SCHEMA, handler=_handle_proxy_stop, emoji="⏹️")
    if _NATIVE:
        ctx.register_middleware("llm_request", _on_llm_request)
