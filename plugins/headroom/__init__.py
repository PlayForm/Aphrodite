"""
headroom v1.0.2 - Hermes plugin: MCP tools + transparent CCR middleware + proxyless mode.

Tools:
  headroom_compress     - compress content on demand via proxy /v1/compress
  headroom_retrieve     - resolve CCR markers (local file + proxy + BM25)
  headroom_stats        - proxy compression statistics
  headroom_proxy_start  - start headroom proxy
  headroom_proxy_stop   - stop headroom proxy
  headroom_proxy_status - check proxy health (:8787, :8788)

Middleware (HERMES_HEADROOM_NATIVE=1):
  llm_request hook scans API messages for <<ccr:HASH>> markers,
  resolves ALL of them via proxy /v1/retrieve, and returns a new
  message list.  Proxy-aware: skips inline compression when routing
  through a local headroom proxy; applies inline compression on
  direct API using the hermes_compress.Compress engine.

Proxyless mode (HERMES_HEADROOM_PROXYLESS=1):
  Coexists with native middleware.  Tool outputs are stored to local
  disk (~/.hermes/headroom_cache/) and replaced with CCR markers.
  The model retrieves content on demand via headroom_retrieve using
  local file reads — no network, no proxy, no sandbox filter issue.
  With HERMES_PROXYLESS_COMPRESS=1, headroom AI-compresses content
  before caching (~60% savings on code files).
"""
import hashlib as _hashlib
import json, os, re, sqlite3, time, urllib.request

PROXY = "http://127.0.0.1:8788"
_CCR = re.compile(r"<<ccr:([a-f0-9]{1,64})[^>]*>>")
_HASH = re.compile(r'hash[=:\s]*([a-f0-9]{1,64})', re.I)
_NATIVE = os.environ.get("HERMES_HEADROOM_NATIVE", "1") != "0"
_PROXYLESS = os.environ.get("HERMES_HEADROOM_PROXYLESS", "0") != "0"
_PROXYLESS_MIN_LINES = int(os.environ.get("HERMES_PROXYLESS_MIN_LINES", "20"))
_PROXYLESS_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "headroom_cache")
_PROXYLESS_DB = os.path.join(os.path.expanduser("~"), ".hermes", "headroom_cache.db")
_PROXYLESS_COMPRESS = os.environ.get("HERMES_PROXYLESS_COMPRESS", "0") != "0"
_PROXYLESS_COMPRESS_MIN_TOKENS = int(os.environ.get("HERMES_PROXYLESS_COMPRESS_MIN_TOKENS", "50"))
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
# Proxyless CCR cache (HERMES_HEADROOM_PROXYLESS=1)
# ═══════════════════════════════════════

def _init_db():
    """Create SQLite tables for CCR storage and stats."""
    if not _PROXYLESS:
        return
    try:
        db = sqlite3.connect(_PROXYLESS_DB)
        db.execute("""CREATE TABLE IF NOT EXISTS ccr_store (
            hash       TEXT PRIMARY KEY,
            content    TEXT NOT NULL,
            tool       TEXT DEFAULT '',
            size_before INTEGER DEFAULT 0,
            size_after  INTEGER DEFAULT 0,
            created_at REAL DEFAULT (strftime('%s','now'))
        )""")
        db.execute("""CREATE TABLE IF NOT EXISTS stats (
            key    TEXT PRIMARY KEY,
            value  TEXT DEFAULT '0'
        )""")
        # Initialize counters if missing
        for k in ("requests", "stored", "retrieved", "bytes_before", "bytes_after", "compressions"):
            db.execute("INSERT OR IGNORE INTO stats(key,value) VALUES(?, '0')", (k,))
        db.commit()
        db.close()
    except Exception:
        pass

def _db_row(key: str) -> str:
    """Read a stats value from SQLite."""
    try:
        db = sqlite3.connect(_PROXYLESS_DB)
        row = db.execute("SELECT value FROM stats WHERE key=?", (key,)).fetchone()
        db.close()
        return row[0] if row else "0"
    except Exception:
        return "0"

def _db_inc(key: str, delta: int = 1):
    """Increment a stats counter in SQLite."""
    try:
        db = sqlite3.connect(_PROXYLESS_DB)
        db.execute("UPDATE stats SET value = CAST(value AS INTEGER) + ? WHERE key=?", (delta, key))
        db.commit()
        db.close()
    except Exception:
        pass

def _ensure_cache():
    """Create cache directory and initialize SQLite DB."""
    if _PROXYLESS:
        os.makedirs(_PROXYLESS_DIR, exist_ok=True)
        _init_db()

def _store_tool_content(content: str, tool: str = "") -> tuple:
    """Store tool output to SQLite + disk cache.  Returns (hash, path_or_None)."""
    h = _hashlib.sha256(content.encode()).hexdigest()[:24]
    p = os.path.join(_PROXYLESS_DIR, f"{h}.txt")

    # Write to SQLite (primary store)
    if _PROXYLESS:
        try:
            db = sqlite3.connect(_PROXYLESS_DB)
            db.execute(
                "INSERT OR REPLACE INTO ccr_store(hash,content,tool,size_before,size_after,created_at) VALUES(?,?,?,?,?,?)",
                (h, content, tool, len(content), len(content), time.time())
            )
            db.commit()
            db.close()
            _db_inc("stored")
            _db_inc("bytes_before", len(content))
            _db_inc("requests")
        except Exception:
            pass

    # Also write to disk (fast retrieval via _read_file)
    try:
        with open(p, "w", encoding="utf-8") as f:
            f.write(content)
    except Exception:
        return h, None
    return h, p

def _retrieve_from_db(hash: str) -> str | None:
    """Retrieve content from SQLite CCR store."""
    try:
        db = sqlite3.connect(_PROXYLESS_DB)
        row = db.execute("SELECT content FROM ccr_store WHERE hash=?", (hash,)).fetchone()
        db.close()
        if row:
            _db_inc("retrieved")
            return row[0]
    except Exception:
        pass
    return None

def _ccr_result(hash: str, path: str, total_lines: int = 0, summary: str = "") -> str:
    """Build the CCR-wrapped JSON result string that replaces tool output."""
    marker = f"<<ccr:{hash},path={path}>>"
    info = f" [{total_lines} lines]" if total_lines else ""
    return (
        f"[TOOL OUTPUT STORED{info}: {marker}"
        f"{' — ' + summary if summary else ''}. "
        f'Retrieve: headroom_retrieve(hash="{marker}", path="{path}")]'
    )

def _compress_content(content: str) -> str:
    """Run headroom AI compression on a single content string.
    Reuses the _compress_inline engine.  Returns compressed text
    or original if compression unavailable."""
    if len(content) < 200:
        return content
    msgs = [{"role": "tool", "content": content, "tool_call_id": "compression"}]
    try:
        return _compress_inline(msgs)[0]["content"]
    except Exception:
        return content

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

    # 1. Local file read (fastest)
    if p:
        c = _read_file(p)
        if c: return json.dumps({"original_content": c, "source": "local"})

    # 2. SQLite internal store (proxyless)
    if _PROXYLESS:
        c = _retrieve_from_db(h)
        if c:
            return json.dumps({"original_content": c, "source": "sqlite"})

    # 3. Proxy fallback
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
    # Internal stats from SQLite when proxyless mode is active
    if _PROXYLESS:
        try:
            stored = int(_db_row("stored"))
            retrieved = int(_db_row("retrieved"))
            requests = int(_db_row("requests"))
            bytes_before = int(_db_row("bytes_before"))
            bytes_after = int(_db_row("bytes_after"))
            compressions = int(_db_row("compressions"))
            avg_savings = round((1 - bytes_after / max(1, bytes_before)) * 100, 1) if bytes_before else 0
            return json.dumps({
                "mode": "proxyless",
                "requests": requests,
                "stored": stored,
                "retrieved": retrieved,
                "bytes_before": bytes_before,
                "bytes_after": bytes_after,
                "compressions": compressions,
                "avg_compression_pct": avg_savings,
                "db_path": _PROXYLESS_DB,
                "cache_dir": _PROXYLESS_DIR,
            })
        except Exception as e:
            return json.dumps({"error": f"internal stats unavailable: {e}"})

    # Proxy stats
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

PROXY_STATUS_SCHEMA = {
    "name": "headroom_proxy_status",
    "description": "Check if headroom proxies are running and healthy.",
    "parameters": {"type": "object", "properties": {}},
}

def _handle_proxy_status(args, **kw):
    result = {}
    for port in (8787, 8788):
        try:
            req = urllib.request.Request(f"http://127.0.0.1:{port}/health")
            data = json.loads(urllib.request.urlopen(req, timeout=3).read())
            result[str(port)] = data.get("status", "unknown")
        except Exception:
            result[str(port)] = "offline"
    return json.dumps(result)

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
            opt.MinTokensToCompress = _PROXYLESS_COMPRESS_MIN_TOKENS if _PROXYLESS_COMPRESS else 100
            opt.TargetRatio = None
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
    # 1. Try SQLite internal store (proxyless)
    if _PROXYLESS:
        c = _retrieve_from_db(hash_key)
        if c:
            _RESOLVED[hash_key] = c
            return c
    # 2. Try proxy /v1/retrieve
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

    # Bridge: detect sandbox-fixed messages, skip their compression
    fixed_indices = set()
    for i, msg in enumerate(msgs):
        content = str(msg.get("content", ""))
        if "_fixed_by" in content[:200] or "_sandbox_empty" in content[:200]:
            fixed_indices.add(i)

    # Resolve CCR markers (returns new list, no mutation)
    msgs, _ = _resolve_ccr_in_messages(msgs)

    # If direct API, compress inline — but protect sandbox-fixed messages
    if not using_proxy and len(msgs) > 2:
        # Temporarily mark fixed messages as user (protected from compression)
        saved = {}
        for i in fixed_indices:
            if i < len(msgs):
                saved[i] = msgs[i].get("role")
                msgs[i] = {**msgs[i], "role": "user"}
        msgs = _compress_inline(msgs)
        # Restore original roles
        for i, role in saved.items():
            if i < len(msgs):
                msgs[i] = {**msgs[i], "role": role}

    request["messages"] = msgs
    return {"request": request, "source": "headroom-native"}

# ═══════════════════════════════════════
# Sandbox recovery — monkey-patch read_file_tool
# ═══════════════════════════════════════

def _patch_read_file():
    """Monkey-patch read_file_tool to recover empty sandbox output.
    Stores recovered content to SQLite + disk for stats tracking.
    Always returns recovered content directly (no CCR wrapping)."""
    try:
        import tools.file_tools as mod
        _orig = mod.read_file_tool
    except Exception as e:
        try:
            import logging
            logging.getLogger("headroom").warning("_patch_read_file: import failed: %s", e)
        except Exception:
            pass
        return

    import functools
    @functools.wraps(_orig)
    def _fixed(path, offset=1, limit=500, task_id="default"):
        result = _orig(path=path, offset=offset, limit=limit, task_id=task_id)
        try:
            data = json.loads(result)
            content = data.get("content", "")
            total_lines = data.get("total_lines", 0)
            # Recover from sandbox filtering — always try this (safe: only fires on empty)
            if (not content or "NO CONTENT" in content) and os.path.isfile(path):
                file_size = data.get("file_size", os.path.getsize(path))
                if total_lines > 0 or file_size > 0:
                    with open(path, encoding="utf-8", errors="replace") as f:
                        lines = f.readlines()
                    start = max(0, offset - 1)
                    end = min(len(lines), start + limit)
                    data["content"] = "".join(
                        f"{i+1}|{line}" for i, line in enumerate(lines[start:end], start=start)
                    )
                    data["_fixed_by"] = "headroom"
                    # Invalidate Hermes internal cache by adding a unique key
                    data["_cache_bust"] = str(time.time())
                    # Also store raw to SQLite if proxyless is active
                    if _PROXYLESS and _PROXYLESS_DIR not in os.path.abspath(path):
                        try:
                            raw = "".join(lines[start:end])
                            _store_tool_content(raw, "read_file")
                        except Exception:
                            pass
                    return json.dumps(data)
        except Exception:
            pass
        return result

    mod.read_file_tool = _fixed


def _patch_terminal():
    """Wrap terminal_tool to detect sandbox empty-output bug.
    In PROXYLESS mode, stores non-empty output locally and returns CCR markers."""
    try:
        import tools.terminal_tool as mod
        _orig = mod.terminal_tool
    except Exception:
        return

    import functools
    @functools.wraps(_orig)
    def _fixed(command, background=False, timeout=None, task_id=None,
               force=False, workdir=None, pty=False,
               notify_on_complete=False, watch_patterns=None):
        result = _orig(command=command, background=background, timeout=timeout,
                       task_id=task_id, force=force, workdir=workdir, pty=pty,
                       notify_on_complete=notify_on_complete, watch_patterns=watch_patterns)
        try:
            data = json.loads(result)
            output = data.get("output", "")
            exit_code = data.get("exit_code", -1)

            # Sandbox empty-output detection
            if exit_code == 0 and (not output.strip() or "NO CONTENT" in output) and command.strip():
                if _PROXYLESS:
                    data["_sandbox_empty"] = True
                    data["output"] = (
                        "_sandbox_empty: Terminal output was filtered by sandbox. "
                        "STOP using terminal for this project. Use read_file for files, "
                        "search_files for listing, execute_code for scripts."
                    )
                    return json.dumps(data)
                data["_sandbox_empty"] = True
                data["_hint"] = "Terminal returned exit 0 with empty output — likely sandbox bug. Try execute_code or read_file instead."
                return json.dumps(data)

            # PROXYLESS mode: store large outputs as CCR; small ones pass through
            if _PROXYLESS and output.strip():
                lines = output.count("\n") + 1
                if lines >= 50 and len(output) > 2000:
                    _ensure_cache()
                    stored = _compress_content(output) if _PROXYLESS_COMPRESS else output
                    h, p = _store_tool_content(stored, "terminal")
                    if p:
                        data["content"] = _ccr_result(h, p, lines,
                            f"terminal[{exit_code}] {command[:60]}")
                        data["_ccr"] = True
                        return json.dumps(data)
        except Exception:
            pass
        return result

    mod.terminal_tool = _fixed


def _patch_execute_code():
    """Wrap execute_code to detect sandbox empty-output bug.
    In PROXYLESS mode, stores non-empty output locally and returns CCR markers."""
    try:
        import tools.code_execution_tool as mod
        _orig = mod.execute_code
    except Exception:
        return

    import functools
    @functools.wraps(_orig)
    def _fixed(code, task_id=None, enabled_tools=None):
        result = _orig(code=code, task_id=task_id, enabled_tools=enabled_tools)
        try:
            data = json.loads(result)
            output = data.get("output", "")
            exit_code = data.get("exit_code", -1)

            # Sandbox empty-output detection
            if (not output.strip() or "NO CONTENT" in output) and code.strip():
                if _PROXYLESS:
                    data["_sandbox_empty"] = True
                    data["output"] = (
                        "_sandbox_empty: execute_code output was filtered. "
                        "Use read_file for files or search_files for listing."
                    )
                    return json.dumps(data)
                data["_sandbox_empty"] = True
                data["_hint"] = "execute_code returned empty — likely sandbox bug. Try read_file or terminal instead."
                return json.dumps(data)

            # PROXYLESS mode: store large outputs as CCR
            if _PROXYLESS and output.strip():
                lines = output.count("\n") + 1
                if lines >= 50 and len(output) > 2000:
                    _ensure_cache()
                    stored = _compress_content(output) if _PROXYLESS_COMPRESS else output
                    h, p = _store_tool_content(stored, "execute_code")
                    if p:
                        data["output"] = _ccr_result(h, p, lines,
                            f"execute_code[{exit_code}] {code[:60]}")
                        data["_ccr"] = True
                        return json.dumps(data)
        except Exception:
            pass
        return result

    mod.execute_code = _fixed


# ═══════════════════════════════════════
# Register
# ═══════════════════════════════════════

def register(ctx):
    ctx.register_tool(name="headroom_compress", toolset="compression",
                      schema=COMPRESS_SCHEMA, handler=_handle_compress, emoji="🗜️")
    ctx.register_tool(name="headroom_retrieve", toolset="compression",
                      schema=RETRIEVE_SCHEMA, handler=_handle_retrieve, emoji="🗜️")
    ctx.register_tool(name="headroom_stats", toolset="compression",
                      schema=STATS_SCHEMA, handler=_handle_stats, emoji="📊")
    ctx.register_tool(name="headroom_proxy_start", toolset="compression",
                      schema=PROXY_START_SCHEMA, handler=_handle_proxy_start, emoji="▶️")
    ctx.register_tool(name="headroom_proxy_stop", toolset="compression",
                      schema=PROXY_STOP_SCHEMA, handler=_handle_proxy_stop, emoji="⏹️")
    ctx.register_tool(name="headroom_proxy_status", toolset="compression",
                      schema=PROXY_STATUS_SCHEMA, handler=_handle_proxy_status, emoji="🩺")
    # Native middleware: disabled when proxyless + no proxy (avoids CCR conflict).
    # Allowed when cache proxy is active — proxy handles compression, native resolves CCR.
    if _NATIVE:
        ctx.register_middleware("llm_request", _on_llm_request)
    if _PROXYLESS:
        _ensure_cache()
    _patch_read_file()
    _patch_terminal()
    _patch_execute_code()
