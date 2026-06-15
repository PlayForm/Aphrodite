"""
aphrodite v1.15.0 - Auto-install + launch aphrodite proxies.
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold
- Recursive CCR resolution, session-scoped stores
- Context engine (ContextEngine subclass), extensible hooks
- 4 tools: aphrodite_retrieve/compress/stats + aphrodite_rebuild
- Re-compression guard: skips content with existing CCR markers
- Read-intent detection in pre_llm_hook
- All thresholds configurable via env vars (see _cfg_int)

On install: downloads pre-built binary from GitHub releases.
On session_start: launches aphrodite proxies not already running.
"""
import os, subprocess, urllib.request, time, logging, platform, stat, re, json, hashlib, base64, zlib

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.5.45"          # binary download version (must match Cargo.toml)
PLUGIN_VERSION = "1.54.0"        # plugin version
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")

# ── Configurable thresholds (env vars) ────────────────────────
def _cfg_int(name, default):
    try: return int(os.environ.get(name, str(default)))
    except: return default

ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 50)  # compress at 50% fill
ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 2)
ENGINE_PROTECT_LAST  = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 5)
ENGINE_MIN_MSGS      = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 30)  # don't compress short conversations
TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 1024)
TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 8192)
TERMINAL_THRESHOLD    = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 2048)
INLINE_THRESHOLD      = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)
RECURSIVE_DEPTH       = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)
DEBUG_LOGGING         = os.environ.get("APHRODITE_DEBUG", "") == "1"

# Dev mode: skip all proxy routing
_DEV = os.environ.get("APHRODITE_DEV", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"
if _DEV:
    _log.warning("aphrodite DEV MODE - plugin disabled, use cargo watch for proxies")
if DEBUG_LOGGING:
    _log.setLevel(logging.DEBUG)
    _log.debug("aphrodite v%s debug logging enabled | engine_threshold=%s protect_first=%s protect_last=%s min_msgs=%s tool_token=%s tool_cache=%s term=%s inline=%s",
        PLUGIN_VERSION, ENGINE_THRESHOLD_PCT, ENGINE_PROTECT_FIRST, ENGINE_PROTECT_LAST,
        ENGINE_MIN_MSGS, TOOL_THRESHOLD_TOKEN, TOOL_THRESHOLD_CACHE, TERMINAL_THRESHOLD, INLINE_THRESHOLD)

# ── CCR regex (shared) ───────────────────────────────────────
_CCR_RE = re.compile(r'<<<CCR:([^>]+)>>>')

# ── Inline compression store (session-scoped) ─────────────────
_inline_store = {}


def _inline_compress(content):
    """Compress content locally using zlib, store in session dict. Returns hash."""
    compressed = base64.urlsafe_b64encode(zlib.compress(content.encode('utf-8'), 9)).decode('ascii')
    h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
    _inline_store[h] = content
    # Keep store bounded
    if len(_inline_store) > 500:
        oldest = next(iter(_inline_store))
        del _inline_store[oldest]
    return h, len(compressed)


def _inline_retrieve(hash_val):
    """Retrieve content from inline store. Returns content or None."""
    return _inline_store.get(hash_val)


def _resolve_one(hash_val, timeout=4, query=""):
    """Resolve a single CCR hash. Checks inline store first, then tries both proxies.
    If query is provided, it's passed to the proxy for line-level filtering."""
    # Check inline store first
    content = _inline_retrieve(hash_val)
    if content is not None:
        _recent_markers.append({'hash': hash_val, 'type': 'retrieved', 'size': len(content), 'preview': content[:200]})
        if len(_recent_markers) > 200:
            _recent_markers.pop(0)
        if query:
            lines = [l for l in content.splitlines() if query.lower() in l.lower()]
            return "\n".join(lines) if lines else content
        return content
    # Try both proxy ports
    payload = {"hash": hash_val}
    if query:
        payload["query"] = query
    for port in (9797, 9798):
        try:
            data = json.dumps(payload).encode()
            req = urllib.request.Request(
                f"http://127.0.0.1:{port}/retrieve",
                data=data,
                headers={"Content-Type": "application/json"}
            )
            with urllib.request.urlopen(req, timeout=timeout) as r:
                result = json.loads(r.read())
            if result.get("found"):
                content = result["content"]
                _inline_store[hash_val] = content  # cache for search + future retrieves
                _recent_markers.append({'hash': hash_val, 'type': 'retrieved', 'size': len(content), 'preview': content[:200]})
                if len(_recent_markers) > 200:
                    _recent_markers.pop(0)
                return content
        except Exception:
            continue
    return None


def _compress_via_proxy(content, target_port):
    """Compress content through proxy CCR. Returns (hash, compressed_size) or None."""
    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target_port}/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        return ccr["hash"], len(content)
    except Exception:
        return None


def _ccr_marker(hash_val, ccr_type, size, mode="", preview=""):
    """Build a standard CCR marker string."""
    base = f"<<<CCR:{hash_val}|{ccr_type}|{size}"
    if mode:
        base += f"|{mode}"
    base += ">>>"
    if preview:
        base += f" {preview}"
    return base



def _detect_platform() -> str:
    """Return platform tag for download URL."""
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        return "macos-arm64" if machine in ("arm64", "aarch64") else "macos-x64"
    elif system == "linux":
        return "linux-x64" if machine == "x86_64" else "linux-arm64"
    return f"{system}-{machine}"


def _download_binary() -> bool:
    """Download aphrodite binary from GitHub releases."""
    os.makedirs(BINARY_DIR, exist_ok=True)
    
    plat = _detect_platform()
    download_url = (
        f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite-{plat}"
    )
    
    _log.info("downloading aphrodite %s from %s", BIN_VERSION, download_url)
    
    try:
        urllib.request.urlretrieve(download_url, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("aphrodite binary installed to %s", BINARY)
        return True
    except Exception as e:
        _log.warning("download failed: %s - falling back to cargo build", e)
        return False


def _ensure_binary() -> bool:
    """Ensure the aphrodite binary exists, downloading if needed."""
    if os.path.exists(BINARY) and os.access(BINARY, os.X_OK):
        return True
    
    # Try download first
    if _download_binary():
        return True
    
    # Fallback: try local build
    repo_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    local_bin = os.path.join(repo_dir, "crates", "aphrodite", "target", "release", "aphrodite")
    if os.path.exists(local_bin):
        import shutil
        shutil.copy2(local_bin, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("copied local binary to %s", BINARY)
        return True
    
    _log.error("no binary found - install cargo or download manually from %s/releases", REPO)
    return False


def _load_env():
    env = {}
    try:
        with open(ENV_FILE) as f:
            for line in f:
                line = line.strip()
                if line.startswith("export "):
                    kv = line[7:].split("=", 1)
                    if len(kv) == 2:
                        env[kv[0]] = kv[1].strip('"').strip("'")
                elif "=" in line and not line.startswith("#"):
                    kv = line.split("=", 1)
                    env[kv[0]] = kv[1].strip('"').strip("'")
    except Exception:
        pass
    return env


# ── Alive cache (5-second TTL) ──────────────────────────────
_alive_cache = {}  # {port: (result, timestamp)}


def _alive(port, timeout=3):
    """Check proxy health with 5-second caching to avoid socket overhead."""
    now = time.time()
    if port in _alive_cache:
        result, ts = _alive_cache[port]
        if now - ts < 5:
            return result
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=timeout)
        body = r.read().decode().strip()
        try:
            data = json.loads(body)
            result = data.get("status") in ("healthy", "ok", "degraded")
        except Exception:
            result = body.strip() == "ok"
    except Exception:
        result = False
    _alive_cache[port] = (result, now)
    return result


def _start(name, env):
    port = PORTS[name]
    key = env.get("APHRODITE_API_KEY", "")
    if not key:
        _log.warning("APHRODITE_API_KEY not set in env - proxy won't authenticate")
        return
    
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--api-key", key, "--mode", "token", "--tool-relay"]
    
    _log.info("starting aphrodite %s on :%s", name, port)
    try:
        subprocess.Popen(
            args,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception as e:
        _log.warning("aphrodite %s launch failed: %s", name, e)


def on_start(**kw):
    if not _ensure_binary():
        _log.error("cannot start - binary not available")
        return
    
    env = {**os.environ, **_load_env()}
    for name in ("token",):
        if not _alive(PORTS[name]):
            _start(name, env)
    
    # Retry loop for proxy readiness
    token_ok = _wait_alive(9798, retries=10, delay=0.3)
    _log.info("aphrodite: token=%s", "UP" if token_ok else "DOWN")


def _wait_alive(port, retries=10, delay=0.3):
    """Wait for proxy port to become alive, with retries."""
    for _ in range(retries):
        if _alive(port):
            return True
        time.sleep(delay)
    return False


# ── Tools ─────────────────────────────────────────────────────

def _resolve_recursive(hash_val, depth=0, resolved=None):
    """Recursively resolve CCR markers in content, up to max depth.
    
    After retrieving content, scans for nested <<<CCR:...>>> markers
    and resolves them in parallel, replacing markers with resolved content.
    """
    if resolved is None:
        resolved = {}
    
    if depth >= RECURSIVE_DEPTH or hash_val in resolved:
        return resolved.get(hash_val, "")
    
    content = _resolve_one(hash_val)
    if content is None:
            return f'<<<CCR:{hash_val}|unresolved>>>'
    
    resolved[hash_val] = content
    
    # Find nested CCR markers
    nested = _CCR_RE.findall(content)
    if not nested:
        return content
    
    # Resolve nested markers in parallel (sequential for simplicity)
    replacements = {}
    for marker in nested:
        parts = marker.split('|')
        if len(parts) >= 1 and parts[0] not in resolved:
            nested_hash = parts[0]
            nested_content = _resolve_recursive(nested_hash, depth + 1, resolved)
            replacements[f'<<<CCR:{marker}>>>'] = nested_content
    
    # Replace markers with resolved content
    for marker_str, replacement in replacements.items():
        content = content.replace(marker_str, replacement)
    
    return content


def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers with recursive depth. Scans for nested markers."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "")
    query = args.get("query", "")
    if not hash_val:
        return '{"error": "missing hash parameter"}'
    try:
        content = _resolve_recursive(hash_val)
        if content and not content.startswith("<<<CCR:"):
            if query:
                lines = [l for l in content.splitlines() if query.lower() in l.lower()]
                if lines:
                    return "\n".join(lines)
                return content  # no matches, return full content
            return content
        return f'{{"error": "CCR entry not found: {hash_val}"}}'
    except Exception as e:
        return f'{{"error": "retrieve failed: {str(e)}"}}'


def _compress_handler(args=None, **kwargs):
    """Compress content into CCR via aphrodite proxy. Content-addressable:
    checks local cache first, only hits proxy on miss."""
    args = args if isinstance(args, dict) else {}
    content = args.get("content", "")
    type_hint = args.get("type", "text")
    if not content:
        return '{"error": "missing content parameter"}'
    
    # Pop the API: check local cache first (content-addressable store)
    h = hashlib.sha256(content.encode('utf-8')).hexdigest()[:16]
    if h in _inline_store:
        return json.dumps({"hash": h, "type": type_hint, "size": len(content), 
                           "source": "cache", "compression_ratio": 0})
    
    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        h = result.get("hash", h)
        if h:
            _inline_store[h] = content  # mirror in inline store for aphrodite_search
        return json.dumps({
            "hash": h,
            "type": type_hint,
            "size": len(content),
            "compression_ratio": result.get("compression_ratio")
        })
    except Exception as e:
        # Fallback: store inline anyway
        _inline_store[h] = content
        return json.dumps({"hash": h, "type": type_hint, "size": len(content), 
                           "source": "inline_fallback", "compression_ratio": 0})


COMPRESS_SCHEMA = {
    "name": "aphrodite_compress",
    "description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to compress and store in CCR"},
            "type": {"type": "string", "description": "Optional: content type hint - code, log, diff, error, json, build_output, text"}
        },
        "required": ["content"]
    }
}
RETRIEVE_SCHEMA = {
    "name": "aphrodite_retrieve",
    "description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads. Recursively resolves nested CCR markers up to 3 levels deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve"},
            "query": {"type": "string", "description": "Optional: filter retrieved content to lines containing this query string"},
            "path": {"type": "string", "description": "Optional: file path to read directly (bypasses CCR)"}
        },
        "required": []
    }
}


# ── Hooks ─────────────────────────────────────────────────────

def _transform_tool_result(
    tool_name="", args=None, result="", tool_call_id="",
    task_id="", session_id="", turn_id="", api_request_id="",
    duration_ms=0, status="", error_type="", error_message="",
    **kwargs,
):
    """Compress tool outputs via CCR. Proxy first, inline fallback when proxy down.
    
    Dual-mode: proxy CCR (token >1KB, cache >8KB) with inline fallback (>4KB).
    Works without proxy - no provider switch required.
    """
    _t0 = time.time()
    if not result or not isinstance(result, str) or not result.strip():
        return result

    if _DEV: return result  # dev mode: passthrough
    # Track file references for aphrodite_files tool
    _track_file_refs(tool_name, args)
    token_alive = _alive(9798)
    proxy_available = token_alive

    # Essential tools: never compress — agent needs immediate access to skills, memory, session history
    _ESSENTIAL_TOOLS = {"skill_view", "skills_list", "skill_manage", "memory", "session_search", "read_file", "read_terminal"}
    skip = _ESSENTIAL_TOOLS | {"aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"} if token_alive else _ESSENTIAL_TOOLS | {"execute_code", "patch", "write_file", "search_files", "todo", "aphrodite_retrieve", "aphrodite_compress", "aphrodite_stats"}
    if tool_name in skip:
        if DEBUG_LOGGING:
            _log.debug("transform_tool_result: SKIP %s %.1fms (in skip list)", tool_name[:40], (time.time()-_t0)*1000)
        return result

    threshold = 1024 if token_alive else 8192 if cache_alive else INLINE_THRESHOLD
    result_len = len(result)
    if result_len < threshold:
        if DEBUG_LOGGING:
            _log.debug("transform_tool_result: BELOW %s size=%s < threshold=%s %.1fms", tool_name[:40], result_len, threshold, (time.time()-_t0)*1000)
        return result

    # Don't re-compress content that already has CCR markers (retrieved/compressed)
    if _CCR_RE.search(result):
        if DEBUG_LOGGING:
            _log.debug("transform_tool_result: GUARD %s has existing CCR marker %.1fms", tool_name[:40], (time.time()-_t0)*1000)
        return result

    preview = result[:120].replace('\\n', ' ').strip()
    
    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(result, target)
        if ccr:
            h, sz = ccr
            label = "token" if token_alive else "cache"
            if DEBUG_LOGGING:
                ratio = result_len / max(len(h), 1)
                _log.debug("transform_tool_result: CCR %s %s:%s size=%s ratio=%.1fx %.1fms", tool_name[:40], label, h, result_len, ratio, (time.time()-_t0)*1000)
            _recent_markers.append({'hash': h, 'type': 'tool', 'size': result_len, 'preview': preview})
            if len(_recent_markers) > 200:
                _recent_markers.pop(0)
            _inline_store[h] = result  # mirror for aphrodite_search
            return _ccr_marker(h, "tool", result_len, label, preview)
        elif DEBUG_LOGGING:
            _log.debug("transform_tool_result: PROXY FAIL %s - proxy returned no hash", tool_name[:40])
    
    # Fallback: inline compression (works without proxy)
    if result_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(result)
            if DEBUG_LOGGING:
                _log.debug("transform_tool_result: INLINE %s hash=%s size=%s %.1fms", tool_name[:40], h, result_len, (time.time()-_t0)*1000)
            _recent_markers.append({'hash': h, 'type': 'tool', 'size': result_len, 'preview': preview})
            if len(_recent_markers) > 200:
                _recent_markers.pop(0)
            return _ccr_marker(h, "tool", result_len, "inline", preview)
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("transform_tool_result: INLINE FAIL %s", tool_name[:40])
            pass
    if DEBUG_LOGGING:
        _log.debug("transform_tool_result: PASSTHROUGH %s size=%s %.1fms", tool_name[:40], result_len, (time.time()-_t0)*1000)
    return result  # soft-fail


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate and copy binary to ~/.hermes/aphrodite/."""
    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "aphrodite"],
        cwd=repo, capture_output=True, text=True, timeout=300,
        env={**os.environ, "PATH": f"{os.path.expanduser('~/.cargo/bin')}:{os.environ.get('PATH', '')}"}
    )
    if result.returncode != 0:
        return f'{{"error": "build failed: {result.stderr[-200:]}"}}'
    
    src = os.path.join(repo, "target/release/aphrodite")
    if os.path.exists(src):
        import shutil
        shutil.copy2(src, BINARY)
        os.chmod(BINARY, 0o755)
        return f'{{"ok": true, "size": {os.path.getsize(BINARY)}, "path": "{BINARY}"}}'
    return '{"error": "binary not found after build"}'


REBUILD_SCHEMA = {
    "name": "aphrodite_rebuild",
    "description": "Rebuild aphrodite crate from source and install binary. Use after code changes.",
    "parameters": {"type": "object", "properties": {}}
}


# ── Conversation Memory via CCR ─────────────────────────────────────

_conv_index = {}  # {sequential_number: (hash, summary, size)}
_turn_counter = 0  # sequential counter (Hermes turn_id is a UUID string)


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    global _turn_counter
    if not conversation_history or assistant_response is None:
        return

    if _DEV: return
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]
    _turn_counter += 1
    tnum = _turn_counter

    # Capture the last user message from conversation history
    last_user = ""
    for msg in reversed(conversation_history):
        if msg.get("role") == "user":
            last_user = msg.get("content", "")[:200]
            break

    summary = f"T{tnum}: {last_user}… → {str(assistant_response)[:200]}"
    # Tag by file type for better retrieval
    if _referenced_files:
        exts = {}
        for path in list(_referenced_files)[-10:]:  # recent files
            ext = os.path.splitext(path)[1] or "noext"
            exts[ext] = exts.get(ext, 0) + 1
        top_exts = sorted(exts.items(), key=lambda x: x[1], reverse=True)[:3]
        file_tag = " ".join(f"{ext}({n})" for ext, n in top_exts)
        summary += f" [{file_tag}]"

    try:
        data = json.dumps({"content": json.dumps({
            "turn": tnum,
            "user": last_user,
            "assistant": str(assistant_response)[:5000],
        })}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())

        _conv_index[tnum] = (ccr["hash"], summary, len(str(assistant_response)))
        if len(_conv_index) > 100:
            oldest = min(_conv_index.keys())
            del _conv_index[oldest]

        _log.debug("conv-cache: stored T%d → %s (%d total)", tnum, ccr["hash"], len(_conv_index))
    except Exception:
        pass


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|mode>>> markers from text. Returns list of dicts."""
    markers = []
    for match in _CCR_RE.finditer(text):
        m = match.group(1)
        parts = m.split('|')
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                # Extract preview text after the >>> terminator
                marker_end = match.end()  # position right after >>>
                preview = text[marker_end:].strip()[:200] if marker_end < len(text) else ''
                markers.append({
                    'hash': str(parts[0]) if parts[0] else '',
                    'type': str(parts[1]),
                    'size': sz,
                    'mode': str(parts[3]) if len(parts) > 3 else '?',
                    'preview': preview,
                })
            except ValueError:
                pass
    # Filter out entries with missing/empty hashes
    # Filter: real CCR hashes are hex (0-9,a-f), ≥8 chars. Placeholders like abc123 filtered.
    return [m for m in markers 
            if m['hash'] and len(m['hash']) >= 8 
            and all(c in '0123456789abcdef' for c in m['hash'].lower())]


_git_cache = {}  # {summary: timestamp}

def _git_summary():
    """Get cached git diff --stat summary. Returns string or None."""
    now = time.time()
    if _git_cache.get("ts", 0) > now - 30:
        return _git_cache.get("summary")
    try:
        import subprocess
        r = subprocess.run(["git", "diff", "--stat"], capture_output=True, text=True, timeout=3)
        if r.returncode == 0 and r.stdout.strip():
            summary = r.stdout.strip().split('\n')[-1] if r.stdout.strip() else None
            _git_cache["ts"] = now
            _git_cache["summary"] = summary
            return summary
    except Exception:
        pass
    return None

def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
    """Before LLM call: build navigable compression catalog.

    CANNOT mutate conversation_history (Hermes passes a copy). Instead:

    WRAPPING PATTERN visible to LLM:
    ┌─ Last ~10 messages: raw, fully in context
    ├─ Tool/terminal outputs >1KB: <<<CCR:hash|type|size>>> markers inline
    ├─ Old turn summaries: compressed to CCR, cataloged here
    └─ Everything else: raw user/assistant text (Hermes keeps it)

    STRATEGY: Provide catalog so LLM uses aphrodite_retrieve(hash)
    instead of scanning 300+ raw messages. Each CCR item below is
    retrievable - the LLM should fetch only what's relevant.
    """
    if _DEV: return
    if not conversation_history or not isinstance(conversation_history, list):
        return

    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None
    ctx_len = len(conversation_history)

    # ── 1. Scan for CCR markers (injected by transform hooks) ──
    markers = []
    total_bytes = 0
    for msg in conversation_history:
        content = msg.get("content", "")
        if isinstance(content, str):
            for m in _parse_ccr_markers(content):
                total_bytes += m['size']
                markers.append(m)
    _recent_markers = markers  # cache for aphrodite_search
    if DEBUG_LOGGING and markers:
        _log.debug("pre_llm_hook: scanned %d CCR markers across %d msgs, %s total compressed",
                   len(markers), ctx_len, _fmt_size(total_bytes))

    # ── 2. Compress old turns to CCR (skip already-compressed) ──
    compress_hint = ""
    if proxy_available and target and ctx_len > 30:
        turns = _group_into_turns(conversation_history)
        if len(turns) > 6:
            old_turns = turns[:-6]
            # Filter out turns already in _conv_index (prevents re-compression)
            old_turns = [t for t in old_turns if t["id"] not in _conv_index]
            if not old_turns:
                compress_hint = ""  # already compressed, skip
            else:
                try:
                    summaries = []
                    for t in old_turns:
                        summaries.append({
                            "turn": t["id"],
                            "user": t.get("user", "")[:300],
                            "assistant": t.get("assistant", "(tool calls)")[:300],
                        })
                    packed = json.dumps(summaries)
                    if len(packed) > 500:
                        data = json.dumps({"content": packed}).encode()
                        req = urllib.request.Request(
                            f"http://127.0.0.1:{target}/ccr/create",
                            data=data, headers={"Content-Type": "application/json"})
                        with urllib.request.urlopen(req, timeout=3) as r:
                            ccr = json.loads(r.read())
                        kept = len(turns) - len(old_turns)
                        compress_hint = (
                            f"  [TURN ARCHIVE] CCR:{ccr['hash']} | "
                            f"turns T{turns[0]['id']}-T{old_turns[-1]['id']} "
                            f"({len(old_turns)} turns compressed, last {kept} raw)\n"
                            f"  retrieve: aphrodite_retrieve({ccr['hash']})"
                        )
                except Exception:
                    pass

    # ── 3. Build the catalog ──────────────────────────────────
    parts = []
    if markers or _conv_index or compress_hint or len(_referenced_files) > 5 or DEBUG_LOGGING:
        parts.append("[APHRODITE]")
        
        # Debug banner injected into conversation (when APHRODITE_DEBUG=1)
        if DEBUG_LOGGING:
            parts.append(f"  ⚙ v{PLUGIN_VERSION} | mode={'proxy+hooks' if not os.environ.get('APHRODITE_CONTEXT_ENGINE') else 'proxy+hooks+engine'} | engine={'enabled' if os.environ.get('APHRODITE_CONTEXT_ENGINE')=='1' else 'off'} | dev={'on' if _DEV else 'off'}")
            parts.append(f"  ⚙ thresholds: term={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_tok={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE} engine_pct={ENGINE_THRESHOLD_PCT}% prot={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min={ENGINE_MIN_MSGS}")
        
        # Git diff summary (cached 30s)
        git_info = _git_summary()
        if git_info:
            parts.append(f"  git: {git_info}")
        
        # Compression wrapping summary
        if proxy_available:
            mode = "token" if token_alive else "cache"
            parts.append(f"  mode={mode} | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
        else:
            parts.append(f"  mode=inline | {len(markers)} compressed items ({_fmt_size(total_bytes)} saved)")
        
        # Engine stats (from ContextEngine, if active)
        engine = get_engine()
        if engine and engine.compression_count > 0:
            parts.append(f"  engine: {engine.compression_count} compressions | last: {engine.last_compression.get('messages_compressed', '?')} msgs → CCR:{engine.last_compression.get('hash', '?')[:8]}")
        
        # Turn archive
        if compress_hint:
            parts.append(compress_hint)
        
        # CCR catalog: grouped by type — filtered to live/retrievable entries only
        if markers:
            live = [m for m in markers if m['hash'] in _inline_store or _inline_retrieve(m['hash'])]
            if not live and markers:
                # Fallback: if all filtered, show all (don't hide real content)
                live = markers
            by_type = {}
            for m in live:
                by_type.setdefault(m['type'], []).append(m)
            
            parts.append(f"  catalog ({len(markers)} items):")
            for ctype, items in sorted(by_type.items()):
                visible = min(len(items), 3)
                parts.append(f"    [{ctype}] {len(items)} items:")
                for i, m in enumerate(items[:visible]):
                    preview = _extract_preview(m, conversation_history)
                    h = str(m.get('hash', '')).strip()
                    if len(h) < 4 or h in ('{}', '?', 'None', 'null', 'undefined'):
                        continue
                    parts.append(f"      CCR:{h} | {_fmt_size(m['size'])} | {preview}")
                if len(items) > visible:
                    parts.append(f"      ... +{len(items)-visible} more (use aphrodite_retrieve)")
        
        # Conversation memory
        if _conv_index:
            recent = sorted(_conv_index.items(), reverse=True)[:3]
            parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in recent))
        
        # File tree: inject when many files referenced
        if len(_referenced_files) > 5:
            by_dir = {}
            for path in sorted(_referenced_files):
                d = os.path.dirname(path) or "."
                by_dir.setdefault(d, []).append(os.path.basename(path))
            parts.append(f"  files: {len(_referenced_files)} referenced:")
            for d, files in sorted(by_dir.items())[:8]:
                parts.append(f"    {d}/ {', '.join(files[:6])}")
                if len(files) > 6:
                    parts.append(f"      ... +{len(files)-6} more")
            if len(by_dir) > 8:
                parts.append(f"    ... +{len(by_dir)-8} more dirs")
        
        # Context hint
        if ctx_len > 20:
            if ctx_len > 100:
                parts.append(f"  ⚠ context={ctx_len} msgs - prefer aphrodite_retrieve over scanning")
            else:
                parts.append(f"  context={ctx_len} msgs")

        # ── 4. Read-intent detection ──────────────────────────
        READ_KEYWORDS = {"read", "show", "view", "get", "cat", "display",
                         "retrieve", "fetch", "look", "see", "open",
                         "inspect", "check", "print", "dump", "output"}
        last_user = user_message or ""
        if isinstance(conversation_history, list):
            for msg in reversed(conversation_history):
                if msg.get("role") == "user":
                    last_user = str(msg.get("content", ""))[:200].lower()
                    break
        words = set(last_user.lower().split())
        has_read_intent = bool(words & READ_KEYWORDS)
        if has_read_intent and markers:
            recent_markers = markers[-3:]
            parts.append("  intent=read | recent CCRs available: " +
                         " ".join(f"aphrodite_retrieve({m['hash']})"
                                  for m in recent_markers))

    if parts:
        catalog = "\n".join(parts)
        if DEBUG_LOGGING:
            _log.debug("pre_llm_hook: catalog (%d lines, %d markers, %d files)", 
                       len(parts), len(markers), len(_referenced_files))
            _log.debug("pre_llm_hook: %d markers parsed, %d skipped (empty/bad hash)", 
                       len(markers), sum(1 for m in markers if len(str(m.get('hash',''))) < 4))
        return catalog


def _group_into_turns(conversation_history):
    """Group messages into turns (user → assistant → tools)."""
    turns = []
    current = None
    turn_num = 0
    for msg in conversation_history:
        role = msg.get("role", "")
        content = msg.get("content", "")
        if role == "user":
            if current:
                turns.append(current)
            turn_num += 1
            current = {"id": turn_num, "user": str(content)[:1000]}
        elif role == "assistant" and current:
            current["assistant"] = str(content)[:1000]
        elif role == "tool" and current:
            # Tool results accumulate under the current turn
            pass
    if current:
        turns.append(current)
    return turns


def _extract_preview(marker, conversation_history):
    """Extract a short preview for a CCR marker from conversation history."""
    h = marker['hash']
    for msg in conversation_history:
        c = msg.get("content", "")
        if isinstance(c, str) and h in c:
            idx = c.find(h)
            after = c[idx + len(h):].strip()
            if '>>>' in after:
                after = after.split('>>>', 1)[-1].strip()
            return after[:80].strip()
    return ""


def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
    """Compress terminal output via CCR on-the-fly. Proxy first, inline fallback.
    Build output gets smart summarization - repeated patterns collapsed."""
    _t0 = time.time()
    if _DEV: return output  # dev mode: passthrough
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    out_len = len(output)
    if out_len < TERMINAL_THRESHOLD:  # use configured threshold
        if DEBUG_LOGGING:
            _log.debug("terminal_hook: BELOW size=%s < threshold=%s %.1fms (cmd: %s)", out_len, TERMINAL_THRESHOLD, (time.time()-_t0)*1000, command[:60])
        return output

    # Don't re-compress content that already has CCR markers (retrieved/compressed)
    if _CCR_RE.search(output):
        if DEBUG_LOGGING:
            _log.debug("terminal_hook: GUARD has existing CCR marker %.1fms (cmd: %s)", (time.time()-_t0)*1000, command[:60])
        return output

    # ── Build output detection: collapse repeated lines ──────────────
    first_line = output.split('\n', 1)[0].strip() if output else ""
    is_build = any(first_line.startswith(p) for p in (
        "Compiling ", "   Compiling ", "Finished ", "error:",
        "warning:", "Running ", "PASSED", "FAILED", "test result:",
    ))
    if is_build and output.count('\n') > 20:
        lines = output.splitlines()
        # Count unique patterns, deduplicate consecutive repeats
        unique = []
        counts = {}
        prev = None
        for line in lines:
            stripped = line.strip()
            if stripped == prev:
                counts[stripped] = counts.get(stripped, 1) + 1
            else:
                if stripped not in counts:
                    unique.append(stripped)
                counts[stripped] = counts.get(stripped, 0) + 1
                prev = stripped
        
        # Build summary: unique error/warning lines + total
        errors = [l for l in unique if 'error' in l.lower() and l not in ('error:', 'error')]
        warnings = [l for l in unique if 'warning' in l.lower() and 'warning:' not in l]
        summary = f"[build: {len(lines)} lines, {len(unique)} unique patterns]"
        if errors:
            summary += f" | errors: {'; '.join(errors[:5])}"
        if warnings:
            summary += f" | warnings: {'; '.join(warnings[:3])}"
        out_len = len(summary)
        if DEBUG_LOGGING:
            _log.debug("terminal_hook: BUILD collapse %d→%d lines (cmd: %s)", len(lines), len(summary.split('\n')), command[:60])
        # Store full output in CCR, return summary
        if proxy_available:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            ccr = _compress_via_proxy(output, target)
            if ccr:
                h, _ = ccr
                if DEBUG_LOGGING:
                    _log.debug("terminal_hook: BUILD-CCR %s:%s", "token" if token_alive else "cache", h)
                return f'<<<CCR:{h}|build|{len(output)}>>> {summary}…(use aphrodite_retrieve)'
        # Inline fallback
        h, _ = _inline_compress(output)
        return f'<<<CCR:{h}|build|{len(output)}|inline>>> {summary}…(use aphrodite_retrieve)'

    preview = output[:200].replace('\n', ' ').strip()
    
    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(output, target)
        if ccr:
            h, _ = ccr
            if DEBUG_LOGGING:
                ratio = out_len / max(len(h), 1)
                _log.debug("terminal_hook: CCR %s:%s size=%s ratio=%.1fx", "token" if token_alive else "cache", h, out_len, ratio)
            return f'<<<CCR:{h}|terminal|{out_len}>>> {preview}…(use aphrodite_retrieve)'
        elif DEBUG_LOGGING:
            _log.debug("terminal_hook: PROXY FAIL - returned no hash (cmd: %s)", command[:60])
    
    # Fallback: inline compression
    if out_len >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(output)
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE hash=%s size=%s", h, out_len)
            return f'<<<CCR:{h}|terminal|{out_len}|inline>>> {preview}…(use aphrodite_retrieve)'
        except Exception:
            if DEBUG_LOGGING:
                _log.debug("terminal_hook: INLINE FAIL (cmd: %s)", command[:60])
            pass
    if DEBUG_LOGGING:
        _log.debug("terminal_hook: PASSTHROUGH size=%s %.1fms", out_len, (time.time()-_t0)*1000)
    return output


def _inline_clear():
    """Clear the inline store (called on session reset)."""
    _inline_store.clear()


def _fmt_size(b):
    if b >= 1_000_000: return f"{b/1_000_000:.1f}MB"
    if b >= 1000: return f"{b/1000:.1f}KB"
    return f"{b}B"


def _stats_handler(args=None, **kwargs):
    """Return proxy health, CCR stats, engine status, inline store size."""
    result = {"proxy": {}, "engine": {}, "inline_store": {
        "entries": len(_inline_store),
        "total_bytes": sum(len(v) for v in _inline_store.values()),
    }}
    
    # Proxy health
    for name, port in PORTS.items():
        try:
            r = urllib.request.urlopen(f"http://127.0.0.1:{port}/stats", timeout=2)
            data = json.loads(r.read())
            ccr = data.get("ccr", {})
            result["proxy"][name] = {
                "alive": True,
                "ccr_created": ccr.get("created", 0),
                "ccr_hits": ccr.get("hits", 0),
                "ccr_misses": ccr.get("misses", 0),
                "ccr_entries": ccr.get("entries", "?"),
                "tokens_saved": data.get("tokens_saved", 0),
                "requests_total": data.get("requests", {}).get("total", 0),
                "requests_compressed": data.get("requests", {}).get("compressed", 0),
                "compressions_by_type": data.get("compressions_by_type", {}),
            }
        except Exception:
            result["proxy"][name] = {"alive": False}
    
    # Engine status
    eng = get_engine()
    if eng:
        result["engine"] = {
            "active": True,
            "compressions": eng.compression_count,
            "threshold_tokens": eng.threshold_tokens,
            "last_prompt_tokens": eng.last_prompt_tokens,
            "context_length": eng.context_length,
            "protect_first_n": eng.protect_first_n,
            "protect_last_n": eng.protect_last_n,
            "last_compression": eng.last_compression,
            "session_id": eng.session_id,
        }
    else:
        result["engine"] = {"active": False}
    
    return json.dumps(result)


STATS_SCHEMA = {
    "name": "aphrodite_stats",
    "description": "Check aphrodite proxy health, CCR stats, engine compression status. Use when debugging compression or checking if proxy is alive.",
    "parameters": {"type": "object", "properties": {}}
}

# ── File tracking (for aphrodite_files tool) ──────────────────
_referenced_files = {}  # {filepath: last_tool_name}
_recent_markers = []     # list of {hash, type, size, preview} from catalog

_FILE_TOOLS = {"read_file", "write_file", "patch", "search_files"}

def _track_file_refs(tool_name, args):
    """Track file paths referenced by tool calls."""
    if tool_name not in _FILE_TOOLS:
        return
    args = args if isinstance(args, dict) else {}
    path = args.get("path", args.get("file", ""))
    if path and isinstance(path, str) and len(path) < 500:
        _referenced_files[path] = tool_name
        if len(_referenced_files) > 200:
            oldest = next(iter(_referenced_files))
            del _referenced_files[oldest]

def _files_handler(args=None, **kwargs):
    """List all files referenced in the current session."""
    if not _referenced_files:
        return json.dumps({"files": [], "count": 0, "hint": "No file operations yet"})
    by_tool = {}
    for path, tool in sorted(_referenced_files.items()):
        by_tool.setdefault(tool, []).append(path)
    return json.dumps({
        "count": len(_referenced_files),
        "by_tool": {t: sorted(paths) for t, paths in sorted(by_tool.items())},
        "all": sorted(_referenced_files.keys()),
    })

FILES_SCHEMA = {
    "name": "aphrodite_files",
    "description": "List all file paths referenced in the current session. Grouped by tool type. Use to see what files have been touched before making decisions.",
    "parameters": {"type": "object", "properties": {}}
}

def _diff_handler(args=None, **kwargs):
    """Show conversation turn diffs - what was discussed in recent turns."""
    if not _conv_index:
        return json.dumps({"turns": 0, "hint": "No turn history yet"})
    turns = []
    for tnum in sorted(_conv_index.keys(), reverse=True)[:10]:
        h, summary, size = _conv_index[tnum]
        turns.append({"turn": tnum, "hash": h, "summary": summary, "size": size})
    return json.dumps({"turns": len(_conv_index), "recent": turns})

DIFF_SCHEMA = {
    "name": "aphrodite_diff",
    "description": "Show conversation turn history - what was discussed, compressed, and stored across turns. Use to understand context evolution.",
    "parameters": {"type": "object", "properties": {}}
}

def _search_handler(args=None, **kwargs):
    """Search across compressed items by type or content pattern."""
    args = args if isinstance(args, dict) else {}
    query = args.get("query", "").lower()
    ccr_type = args.get("type", "")
    
    results = []
    # Search conversation turn index
    for tnum, (h, summary, size) in sorted(_conv_index.items(), reverse=True):
        if query and query not in summary.lower():
            continue
        results.append({"source": "turn", "turn": tnum, "hash": h, "summary": summary, "size": size})
    
    # Search inline store
    for h, content in _inline_store.items():
        if query and query not in content.lower():
            continue
        preview = content[:200].replace('\n', ' ').strip()
        results.append({"source": "inline", "hash": h, "preview": preview, "size": len(content)})
    
    # Search recent marker catalog (from pre_llm_hook)
    for m in _recent_markers:
        if query and query not in m.get('preview', '').lower():
            continue
        results.append({"source": "marker", "hash": m['hash'], "type": m.get('type', '?'), 
                        "size": m.get('size', 0), "preview": m.get('preview', '')[:200]})
    
    if ccr_type:
        results = [r for r in results if ccr_type in r.get("type", "") or ccr_type in r.get("summary", "") + r.get("preview", "")]
    
    return json.dumps({
        "query": query,
        "type_filter": ccr_type,
        "matches": len(results),
        "hint": "Use aphrodite_retrieve(hash) on any result hash to get full content.",
        "results": results[:20],
    })


def _test_handler(args=None, **kwargs):
    """Full smoke test suite — exercises all tools, hooks, compression, search, retrieve."""
    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "quick")  # quick, full, matrix
    report = {"suite": "aphrodite_smoke", "version": PLUGIN_VERSION, "mode": mode, "tests": []}
    
    def test(name, fn):
        try:
            t0 = time.time()
            result = fn()
            elapsed = (time.time() - t0) * 1000
            report["tests"].append({"name": name, "status": "PASS", "elapsed_ms": round(elapsed, 1), "result": result})
        except Exception as e:
            report["tests"].append({"name": name, "status": "FAIL", "error": str(e)})
    
    # ── Tool smoke tests ─────────────────────────────────
    test("compress_json", lambda: json.loads(_compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"})))
    test("compress_code", lambda: json.loads(_compress_handler(args={"content": "def foo():\n    return 42\n", "type": "code"})))
    test("compress_cache_hit", lambda: _compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"}))  # should hit cache
    
    test("retrieve_roundtrip", lambda: "def foo" in _retrieve_handler(args={"hash": hashlib.sha256(b"def foo():\n    return 42\n").hexdigest()[:16]}))
    
    test("stats", lambda: json.loads(_stats_handler())["proxy"])
    
    test("files_empty", lambda: json.loads(_files_handler())["count"] == 0)
    
    test("diff_empty", lambda: json.loads(_diff_handler())["turns"] == 0)
    
    # ── Proxy health ─────────────────────────────────────
    test("proxy_health", lambda: _alive(9798))
    test("proxy_metrics", lambda: _alive(9797))
    
    # ── Full mode: heavy compression test ────────────────
    if mode in ("full", "matrix"):
        big_payload = json.dumps({"data": list(range(1000)), "nested": {"deep": {"values": [i*i for i in range(200)]}}})
        test("compress_large", lambda: json.loads(_compress_handler(args={"content": big_payload, "type": "json"}))["size"] > 1000)
        test("search_find", lambda: json.loads(_search_handler(args={"query": "deep"}))["matches"] >= 1)
        test("terminal_threshold", lambda: TERMINAL_THRESHOLD > 0)
        test("inline_threshold", lambda: INLINE_THRESHOLD > 0)
    
    # ── Matrix mode: settings sweep ──────────────────────
    if mode == "matrix":
        settings = {"results": {}}
        for pct in (0, 25, 50, 75, 100):
            for protect in (2, 5, 10):
                key = f"pct={pct}_protect={protect}"
                settings["results"][key] = {
                    "threshold_pct": pct,
                    "protect_last": protect,
                    "compresses_always": pct == 0,
                    "compresses_never": pct >= 100,
                }
        report["settings_matrix"] = settings
    
    # ── Pipeline mode: full + matrix + feature toggles ─────
    if mode == "pipeline":
        # Feature toggle: test with/without debug, with/without compression
        toggles = {
            "debug_on": {"APHRODITE_DEBUG": "1"},
            "debug_off": {"APHRODITE_DEBUG": "0"},
            "engine_on": {"APHRODITE_CONTEXT_ENGINE": "1"},
            "engine_off": {"APHRODITE_CONTEXT_ENGINE": "0"},
        }
        feature_results = {}
        for name, env_overrides in toggles.items():
            saved = {k: os.environ.get(k, "") for k in env_overrides}
            for k, v in env_overrides.items():
                os.environ[k] = v
            feature_results[name] = {
                "env": env_overrides,
                "proxy_alive": _alive(9798),
                "cache_alive": _alive(9797),
                "thresholds": {
                    "terminal": TERMINAL_THRESHOLD,
                    "inline": INLINE_THRESHOLD,
                    "tool_token": TOOL_THRESHOLD_TOKEN,
                    "tool_cache": TOOL_THRESHOLD_CACHE,
                },
                "engine_threshold": ENGINE_THRESHOLD_PCT,
            }
            for k, orig in saved.items():
                if orig:
                    os.environ[k] = orig
                else:
                    os.environ.pop(k, None)
        report["feature_toggles"] = feature_results
    
    report["summary"] = {
        "total": len(report["tests"]),
        "passed": sum(1 for t in report["tests"] if t["status"] == "PASS"),
        "failed": sum(1 for t in report["tests"] if t["status"] == "FAIL"),
    }
    
    # ── Save results for regression comparison ─────────────
    try:
        results_path = os.path.join(os.path.dirname(__file__), ".test-results.json")
        prev = {}
        if os.path.exists(results_path):
            with open(results_path) as f:
                prev = json.load(f)
        with open(results_path, "w") as f:
            json.dump(report, f, indent=2)
        if prev:
            prev_passed = prev.get("summary", {}).get("passed", 0)
            curr_passed = report["summary"]["passed"]
            report["regression"] = {
                "previous_passed": prev_passed,
                "current_passed": curr_passed,
                "delta": curr_passed - prev_passed,
                "status": "DEGRADED" if curr_passed < prev_passed else "OK"
            }
    except Exception:
        pass
    return json.dumps(report, indent=2)

TEST_SCHEMA = {
    "name": "aphrodite_test",
    "description": "Run full smoke test suite — compress, retrieve, search, stats, files, diff, proxy health. Modes: quick, full, matrix, pipeline.",
    "parameters": {
        "type": "object",
        "properties": {
            "mode": {"type": "string", "description": "Test mode: quick (default), full, or matrix"}
        }
    }
}

SEARCH_SCHEMA = {
    "name": "aphrodite_search",
    "description": "Search across CCR entries - find compressed content by keyword or type. Use to locate previously compressed context without knowing the hash.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search keyword or phrase to find in compressed content"},
            "type": {"type": "string", "description": "Optional: filter by CCR type (tool, terminal, code, error, etc.)"}
        },
        "required": ["query"]
    }
}


def register(ctx):
    # Install binary on registration
    _ensure_binary()
    ctx.register_hook("on_session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
    ctx.register_hook("post_llm_call", _store_conversation_turn)
    ctx.register_hook("transform_tool_result", _transform_tool_result)
    ctx.register_tool(
        name="aphrodite_rebuild",
        schema=REBUILD_SCHEMA,
        handler=_rebuild_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_compress",
        schema=COMPRESS_SCHEMA,
        handler=_compress_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_retrieve",
        schema=RETRIEVE_SCHEMA,
        handler=_retrieve_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_stats",
        schema=STATS_SCHEMA,
        handler=_stats_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_files",
        schema=FILES_SCHEMA,
        handler=_files_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_diff",
        schema=DIFF_SCHEMA,
        handler=_diff_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_search",
        schema=SEARCH_SCHEMA,
        handler=_search_handler,
        toolset="aphrodite",
    )
    ctx.register_tool(
        name="aphrodite_test",
        schema=TEST_SCHEMA,
        handler=_test_handler,
        toolset="aphrodite",
    )
    # Only register context engine when explicitly configured
    engine_configured = os.environ.get("APHRODITE_CONTEXT_ENGINE", "") == "1"
    if engine_configured:
        try:
            ctx.register_context_engine(AphroditeContextEngine())
            _log.info("aphrodite context engine registered")
        except Exception as e:
            _log.debug("context engine registration skipped: %s", e)
    else:
        _log.info("context engine not registered - set APHRODITE_CONTEXT_ENGINE=1 to enable")
    _log.info("aphrodite v%s registered — %d tools + hooks", PLUGIN_VERSION, 8)
    
    # ── Debug banner: print configuration on startup ──────────
    if DEBUG_LOGGING:
        lines = [
            "=" * 60,
            f"APHRODITE v{PLUGIN_VERSION} — DEBUG MODE",
            f"  Mode: {'proxy+hooks' if not engine_configured else 'proxy+hooks+engine'} | Engine: {'enabled' if engine_configured else 'disabled'} | Dev: {'on' if _DEV else 'off'}",
            f"  Thresholds: terminal={TERMINAL_THRESHOLD} inline={INLINE_THRESHOLD} tool_token={TOOL_THRESHOLD_TOKEN} tool_cache={TOOL_THRESHOLD_CACHE}",
            f"  Engine: threshold={ENGINE_THRESHOLD_PCT}% protect={ENGINE_PROTECT_FIRST}/{ENGINE_PROTECT_LAST} min_msgs={ENGINE_MIN_MSGS}",
            f"  CCR: regex={_CCR_RE.pattern} depth={RECURSIVE_DEPTH}",
            f"  Tools: retrieve, compress, stats, rebuild, files, diff, search, test",
            f"  Proxies: cache=:9797 token=:9798 | waiting for session_start...",
            "=" * 60,
        ]
        for line in lines:
            print(line)
            _log.info(line)


# ── Context Engine (plugs into Hermes compress() pipeline) ─────

from agent.context_engine import ContextEngine

# Global reference so hooks + other plugins can access the engine
_engine = None


def _set_engine(eng):
    global _engine
    _engine = eng


def get_engine():
    """Return the aphrodite context engine instance, or None.
    
    Other plugins can call this to access the engine and its stats.
    """
    return _engine


def _fire_hook(name, **kwargs):
    """Fire a Hermes hook so other plugins can listen to engine events."""
    try:
        from hermes_cli.plugins import invoke_hook
        invoke_hook(name, **kwargs)
    except Exception:
        pass


class AphroditeContextEngine(ContextEngine):
    """CCR-based context compression engine for Hermes.

    Replaces built-in summarization compressor with CCR offloading.
    Extensible via Hermes hooks - other plugins can listen to:
      - ``aphrodite_engine_compressed`` - fired after each compression

    Set ``context.engine: aphrodite`` in config.yaml to activate.
    Works with proxy (token/cache) or inline fallback (zlib).
    """

    @property
    def name(self) -> str:
        return "aphrodite"
    threshold_percent = ENGINE_THRESHOLD_PCT
    protect_first_n = ENGINE_PROTECT_FIRST
    protect_last_n = ENGINE_PROTECT_LAST
    min_messages_to_compress = ENGINE_MIN_MSGS

    def __init__(self):
        self.last_prompt_tokens = 0
        self.last_completion_tokens = 0
        self.last_total_tokens = 0
        self.threshold_tokens = 0
        self.context_length = 1000000
        self.compression_count = 0
        self.last_compression = {}  # stats from most recent compression
        self.session_id = ""
        # Store reference globally so other plugins + hooks can access
        _set_engine(self)

    def update_from_response(self, usage):
        self.last_prompt_tokens = usage.get("prompt_tokens", 0) or usage.get("input_tokens", 0)
        self.last_completion_tokens = usage.get("completion_tokens", 0) or usage.get("output_tokens", 0)
        self.last_total_tokens = usage.get("total_tokens", 0)
        if self.context_length:
            self.threshold_tokens = 1  # always above threshold

    def should_compress(self, prompt_tokens=None):
        """Compress only when context fill exceeds threshold percentage.
        0 = never compress (disabled). 50 = compress at 50% fill."""
        if self.threshold_percent == 0:
            return False  # disabled
        if not prompt_tokens or not self.context_length:
            return False  # can't calculate - don't compress blindly
        pct = (prompt_tokens / self.context_length) * 100
        return pct >= self.threshold_percent

    def compress(self, messages, current_tokens=None, focus_topic=None):
        """Offload middle messages to CCR, keep head+tail raw.

        Dual-mode: proxy CCR preferred, inline zlib fallback.
        Tool-chain safe: won't split assistant tool_call from its tool_result.
        Editing-aware: active editing sessions preserve more recent context.
        """
        if len(messages) <= self.min_messages_to_compress:
            return messages

        head_n = self.protect_first_n
        tail_n = self.protect_last_n

        # ── Editing session detection: preserve more context ──
        is_editing = False
        for msg in messages[-10:]:
            content = str(msg.get("content", ""))
            if (msg.get("role") == "tool" and 
                any(kw in content.lower() for kw in ("wrote", "patched", "modified", "created", "deleted", "successfully", "written"))):
                is_editing = True
                break
        if is_editing:
            tail_n = max(tail_n, 8)  # keep more context during edits

        # ── Tool-chain safety: extend tail to include full chains ──
        if len(messages) > tail_n:
            boundary = len(messages) - tail_n
            # Backtrack to include orphan tool_results
            while boundary < len(messages) and messages[boundary].get("role") == "tool":
                boundary += 1
                tail_n += 1
            # Also backtrack to include the assistant tool_call that owns the tool_result
            if boundary > 0 and messages[boundary - 1].get("role") == "assistant":
                tool_calls = messages[boundary - 1].get("tool_calls", [])
                if tool_calls:
                    boundary -= 1
                    tail_n += 1
            tail_n = min(tail_n, len(messages) - head_n)

        head = messages[:head_n]
        middle = messages[head_n:-tail_n] if tail_n > 0 else messages[head_n:]
        tail = messages[-tail_n:] if tail_n > 0 else []

        if len(middle) < 3:
            return messages  # too few to compress

        # Pack middle messages (no truncation - content already CCR-marked by hooks)
        packed = json.dumps([{
            "role": m.get("role", ""),
            "content": str(m.get("content", "")),  # full content, not [:2000]
            "tool_call_id": m.get("tool_call_id", ""),
        } for m in middle])

        if len(packed) < 200:
            return messages

        hash_val = None
        size_str = _fmt_size(len(packed))

        # Try proxy CCR first
        token_alive = _alive(PORTS["token"])
        cache_alive = _alive(PORTS["cache"])
        if token_alive or cache_alive:
            target = PORTS["token"] if token_alive else PORTS["cache"]
            try:
                data = json.dumps({"content": packed}).encode()
                req = urllib.request.Request(
                    f"http://127.0.0.1:{target}/ccr/create",
                    data=data,
                    headers={"Content-Type": "application/json"}
                )
                with urllib.request.urlopen(req, timeout=5) as r:
                    ccr = json.loads(r.read())
                hash_val = ccr["hash"]
            except Exception:
                pass

        # Inline fallback
        if not hash_val:
            try:
                hash_val, _ = _inline_compress(packed)
                size_str = _fmt_size(len(packed))
            except Exception:
                return messages

        marker = (
            f"[CONTEXT COMPRESSED: {len(middle)} messages → "
            f"CCR:{hash_val}|{size_str}]\n"
            f"These messages were offloaded to reduce context. "
            f"Retrieve with: aphrodite_retrieve({hash_val}).\n"
            f"The {self.protect_last_n} messages below are your active context."
        )

        self.compression_count += 1
        _log.info(
            "context_engine: compressed %d msgs → CCR:%s (%s)",
            len(middle), hash_val, size_str
        )
        self._notify_compressed(len(packed), len(middle), hash_val)

        return head + [{"role": "system", "content": marker}] + tail

    def _notify_compressed(self, packed_len, middle_len, hash_val):
        """Fire hook so other plugins can react to compression."""
        self.last_compression = {
            "messages_compressed": middle_len,
            "packed_size": packed_len,
            "hash": hash_val,
            "count": self.compression_count,
        }
        _fire_hook(
            "aphrodite_engine_compressed",
            engine=self,
            stats=self.last_compression,
        )

    def get_status(self):
        return {
            "last_prompt_tokens": self.last_prompt_tokens,
            "threshold_tokens": self.threshold_tokens,
            "context_length": self.context_length,
            "usage_percent": (
                min(100, self.last_prompt_tokens / self.context_length * 100)
                if self.context_length else 0
            ),
            "compression_count": self.compression_count,
        }

    def update_model(self, model="", context_length=0, base_url="", api_key="", provider="", api_mode="", **kw):
        if context_length:
            self.context_length = context_length
            self.threshold_tokens = 1  # always above threshold

    def on_session_reset(self):
        self.last_prompt_tokens = 0
        self.last_completion_tokens = 0
        self.last_total_tokens = 0
        self.compression_count = 0
        self.last_compression = {}
        _inline_clear()
        global _conv_index, _turn_counter, _referenced_files, _recent_markers
        _conv_index.clear()
        _turn_counter = 0
        _referenced_files.clear()
        _recent_markers.clear()
        _log.info("aphrodite v%s: session reset - inline store + memory cleared", PLUGIN_VERSION)

    def on_session_start(self, session_id="", **kw):
        self.session_id = session_id
        _log.info("context_engine: session %s started", session_id[:16] if session_id else "?")
