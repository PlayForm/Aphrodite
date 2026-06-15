"""
aphrodite v1.8.0 — Auto-install + launch aphrodite proxies.
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold
- Recursive CCR resolution, session-scoped stores
- Context engine (ContextEngine subclass), extensible hooks
- 4 tools: headroom_retrieve/compress/stats + aphrodite_rebuild
- All thresholds configurable via env vars (see _cfg_int)

On install: downloads pre-built binary from GitHub releases.
On session_start: launches aphrodite proxies not already running.
"""
import os, subprocess, urllib.request, time, logging, platform, stat, re, json, hashlib, base64, zlib

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.2.0"          # binary download version
PLUGIN_VERSION = "1.8.0"        # plugin version
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")

# ── Configurable thresholds (env vars) ────────────────────────
def _cfg_int(name, default):
    try: return int(os.environ.get(name, str(default)))
    except: return default

ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 0)  # 0=always compress
ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 2)
ENGINE_PROTECT_LAST  = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 5)
ENGINE_MIN_MSGS      = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 0)
TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 1024)
TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 8192)
TERMINAL_THRESHOLD    = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 2048)
INLINE_THRESHOLD      = _cfg_int("APHRODITEINLINE_THRESHOLD", 4096)
RECURSIVE_DEPTH       = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)
DEBUG_LOGGING         = os.environ.get("APHRODITE_DEBUG", "") == "1"

# Dev mode: skip all proxy routing
_DEV = os.environ.get("APHRODITE_DEV", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"
if _DEV:
    _log.warning("aphrodite DEV MODE — plugin disabled, use cargo watch for proxies")
if DEBUG_LOGGING:
    _log.setLevel(logging.DEBUG)
    _log.debug("aphrodite v%s debug logging enabled | engine_threshold=%s protect_first=%s protect_last=%s min_msgs=%s tool_token=%s tool_cache=%s term=%s inline=%s",
        PLUGIN_VERSION, ENGINE_THRESHOLD_PCT, ENGINE_PROTECT_FIRST, ENGINE_PROTECT_LAST,
        ENGINE_MIN_MSGS, TOOL_THRESHOLD_TOKEN, TOOL_THRESHOLD_CACHE, TERMINAL_THRESHOLD, INLINE_THRESHOLD)

# ── CCR regex (shared) ───────────────────────────────────────
_CCR_RE = re.compile(r'\[CCR:([^\]]+)\]')

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


def _resolve_one(hash_val, timeout=4):
    """Resolve a single CCR hash. Checks inline store first, then proxy."""
    # Check inline store first
    content = _inline_retrieve(hash_val)
    if content is not None:
        return content
    # Fall through to proxy
    try:
        data = json.dumps({"hash": hash_val}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/retrieve",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=timeout) as r:
            result = json.loads(r.read())
        if result.get("found"):
            return result["content"]
    except Exception:
        pass
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
    base = f"[CCR:{hash_val}|{ccr_type}|{size}"
    if mode:
        base += f"|{mode}"
    base += "]"
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
    
    download_url = (
        f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite"
    )
    
    _log.info("downloading aphrodite %s from %s", BIN_VERSION, download_url)
    
    try:
        urllib.request.urlretrieve(download_url, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("aphrodite binary installed to %s", BINARY)
        return True
    except Exception as e:
        _log.warning("download failed: %s — falling back to cargo build", e)
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
    
    _log.error("no binary found — install cargo or download manually from %s/releases", REPO)
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


def _alive(port, timeout=3):
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=timeout)
        body = r.read().decode().strip()
        return body == "ok" or '"status":"healthy"' in body
    except Exception:
        return False


def _start(name, env):
    port = PORTS[name]
    key = env.get("APHRODITE_API_KEY", "")
    if not key:
        _log.warning("APHRODITE_API_KEY not set in env — proxy won't authenticate")
        return
    
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--api-key", key]
    if name == "token":
        args += ["--mode", "token", "--tool-relay"]
    else:
        args += ["--mode", "cache"]
    
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
        _log.error("cannot start — binary not available")
        return
    
    env = {**os.environ, **_load_env()}
    for name in ("cache", "token"):
        if not _alive(PORTS[name]):
            _start(name, env)
    time.sleep(0.5)
    cache_ok = _alive(9797)
    token_ok = _alive(9798)
    _log.info("aphrodite: cache=%s token=%s", "UP" if cache_ok else "DOWN", "UP" if token_ok else "DOWN")


# ── Tools ─────────────────────────────────────────────────────

def _resolve_recursive(hash_val, depth=0, resolved=None):
    """Recursively resolve CCR markers in content, up to max depth.
    
    After retrieving content, scans for nested [CCR:...] markers
    and resolves them in parallel, replacing markers with resolved content.
    """
    if resolved is None:
        resolved = {}
    
    if depth >= RECURSIVE_DEPTH or hash_val in resolved:
        return resolved.get(hash_val, "")
    
    content = _resolve_one(hash_val)
    if content is None:
        return f'[CCR:{hash_val}|unresolved]'
    
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
            replacements[f'[CCR:{marker}]'] = nested_content
    
    # Replace markers with resolved content
    for marker_str, replacement in replacements.items():
        content = content.replace(marker_str, replacement)
    
    return content


def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers with recursive depth. Scans for nested markers."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "")
    if not hash_val:
        return '{"error": "missing hash parameter"}'
    try:
        content = _resolve_recursive(hash_val)
        if content and not content.startswith("[CCR:"):
            return content
        return f'{{"error": "CCR entry not found: {hash_val}"}}'
    except Exception as e:
        return f'{{"error": "retrieve failed: {str(e)}"}}'


def _compress_handler(args=None, **kwargs):
    """Compress content into CCR via aphrodite proxy."""
    args = args if isinstance(args, dict) else {}
    content = args.get("content", "")
    if not content:
        return '{"error": "missing content parameter"}'
    try:
        data = json.dumps({"content": content}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        return json.dumps({"hash": result.get("hash"), "compression_ratio": result.get("compression_ratio")})
    except Exception as e:
        return f'{{"error": "compress failed: {str(e)}"}}'


COMPRESS_SCHEMA = {
    "name": "headroom_compress",
    "description": "Compress content into CCR via aphrodite proxy for later retrieval.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "Content to compress and store in CCR"}
        },
        "required": ["content"]
    }
}
RETRIEVE_SCHEMA = {
    "name": "headroom_retrieve",
    "description": "Resolve CCR markers to original content via aphrodite proxy. Recursively resolves nested CCR markers up to 3 levels deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve"}
        },
        "required": ["hash"]
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
    Works without proxy — no provider switch required.
    """
    if not result or not isinstance(result, str) or not result.strip():
        return result

    if _DEV: return result  # dev mode: passthrough
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    skip = {"read_file", "read_terminal", "headroom_retrieve", "headroom_stats"} if token_alive else {"read_file", "read_terminal", "execute_code", "memory", "patch", "write_file", "search_files", "todo", "headroom_retrieve", "headroom_stats"}
    if tool_name in skip:
        return result

    threshold = 1024 if token_alive else 8192 if cache_alive else INLINE_THRESHOLD
    if len(result) < threshold:
        return result

    preview = result[:120].replace('\\n', ' ').strip()
    
    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(result, target)
        if ccr:
            h, sz = ccr
            label = "token" if token_alive else "cache"
            return _ccr_marker(h, "tool", len(result), label, preview)
    
    # Fallback: inline compression (works without proxy)
    if len(result) >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(result)
            return _ccr_marker(h, "tool", len(result), "inline", preview)
        except Exception:
            pass
    return result  # soft-fail


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate and copy binary to ~/.hermes/aphrodite/."""
    repo = "REPLACED/Developer/Application/PlayForm/HermesCompress"
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

        _log.debug("conv-cache: stored T%d → %s (%d total)", tnum, ccr["hash"][:8], len(_conv_index))
    except Exception:
        pass


def _parse_ccr_markers(text):
    """Parse [CCR:hash|type|size|mode] markers from text. Returns list of dicts."""
    markers = []
    for m in _CCR_RE.findall(text):
        parts = m.split('|')
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                markers.append({
                    'hash': parts[0],
                    'type': parts[1],
                    'size': sz,
                    'mode': parts[3] if len(parts) > 3 else '?'
                })
            except ValueError:
                pass
    return markers


def _pre_llm_hook(conversation_history=None, user_message=None, **kwargs):
    """Before LLM call: build navigable compression catalog.

    CANNOT mutate conversation_history (Hermes passes a copy). Instead:

    WRAPPING PATTERN visible to LLM:
    ┌─ Last ~10 messages: raw, fully in context
    ├─ Tool/terminal outputs >1KB: [CCR:hash|type|size] markers inline
    ├─ Old turn summaries: compressed to CCR, cataloged here
    └─ Everything else: raw user/assistant text (Hermes keeps it)

    STRATEGY: Provide catalog so LLM uses headroom_retrieve(hash)
    instead of scanning 300+ raw messages. Each CCR item below is
    retrievable — the LLM should fetch only what's relevant.
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

    # ── 2. Compress old turns to CCR (structured summaries) ───
    compress_hint = ""
    if proxy_available and target and ctx_len > 30:
        turns = _group_into_turns(conversation_history)
        if len(turns) > 6:
            old_turns = turns[:-6]
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
                        f"  [TURN ARCHIVE] CCR:{ccr['hash'][:12]} | "
                        f"turns T{turns[0]['id']}–T{old_turns[-1]['id']} "
                        f"({len(old_turns)} turns compressed, last {kept} raw)\n"
                        f"  retrieve: headroom_retrieve({ccr['hash'][:12]})"
                    )
            except Exception:
                pass

    # ── 3. Build the catalog ──────────────────────────────────
    parts = []
    if markers or _conv_index or compress_hint:
        parts.append("[APHRODITE]")
        
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
        
        # CCR catalog: grouped by type
        if markers:
            by_type = {}
            for m in markers:
                by_type.setdefault(m['type'], []).append(m)
            
            parts.append(f"  catalog ({len(markers)} items):")
            for ctype, items in sorted(by_type.items()):
                visible = min(len(items), 3)
                parts.append(f"    [{ctype}] {len(items)} items:")
                for i, m in enumerate(items[:visible]):
                    preview = _extract_preview(m, conversation_history)
                    parts.append(f"      CCR:{m['hash'][:8]} | {_fmt_size(m['size'])} | {preview}")
                if len(items) > visible:
                    parts.append(f"      ... +{len(items)-visible} more (use headroom_retrieve)")
        
        # Conversation memory
        if _conv_index:
            recent = sorted(_conv_index.items(), reverse=True)[:3]
            parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in recent))
        
        # Context hint
        if ctx_len > 20:
            if ctx_len > 100:
                parts.append(f"  ⚠ context={ctx_len} msgs — prefer headroom_retrieve over scanning")
            else:
                parts.append(f"  context={ctx_len} msgs")

    if parts:
        return "\n".join(parts)


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
            if after.startswith('|'):
                after = after.split(']', 1)[-1] if ']' in after else after
            return after[:80].strip()
    return ""


def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
    """Compress terminal output via CCR on-the-fly. Proxy first, inline fallback."""
    if _DEV: return output  # dev mode: passthrough
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    proxy_available = token_alive or cache_alive

    if len(output) < 2048:  # 2KB min for terminal compression
        return output

    preview = output[:200].replace('\n', ' ').strip()
    
    # Try proxy compression first
    if proxy_available:
        target = PORTS["token"] if token_alive else PORTS["cache"]
        ccr = _compress_via_proxy(output, target)
        if ccr:
            h, _ = ccr
            return f'[CCR:{h}|terminal|{len(output)}] {preview}…(use headroom_retrieve)'
    
    # Fallback: inline compression
    if len(output) >= INLINE_THRESHOLD:
        try:
            h, _ = _inline_compress(output)
            return f'[CCR:{h}|terminal|{len(output)}|inline] {preview}…(use headroom_retrieve)'
        except Exception:
            pass
    return output


# ── Inline compression store (session-scoped, fallback when proxy down) ────
# Maps hash → original content. Cleared on session reset.
_inline_store = {}
INLINE_THRESHOLD = 4096


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
                "ccr_entries": ccr.get("entries", "?"),
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
    "name": "headroom_stats",
    "description": "Check aphrodite proxy health, CCR stats, engine compression status. Use when debugging compression or checking if proxy is alive.",
    "parameters": {"type": "object", "properties": {}}
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
    )
    ctx.register_tool(
        name="headroom_compress",
        schema=COMPRESS_SCHEMA,
        handler=_compress_handler,
    )
    ctx.register_tool(
        name="headroom_retrieve",
        schema=RETRIEVE_SCHEMA,
        handler=_retrieve_handler,
    )
    ctx.register_tool(
        name="headroom_stats",
        schema=STATS_SCHEMA,
        handler=_stats_handler,
    )
    # Register context engine (plugs into Hermes' compress() pipeline)
    try:
        ctx.register_context_engine(AphroditeContextEngine())
        _log.info("aphrodite context engine registered")
    except Exception as e:
        _log.debug("context engine registration skipped: %s", e)
    _log.info("aphrodite v1.7.0 registered — %d tools + context engine + hooks", 4)


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
    Extensible via Hermes hooks — other plugins can listen to:
      - ``aphrodite_engine_compressed`` — fired after each compression

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
        return True  # always compress — emulate token proxy internally

    def compress(self, messages, current_tokens=None, focus_topic=None):
        """Offload middle messages to CCR, keep head+tail raw.

        Dual-mode: proxy CCR preferred, inline zlib fallback.
        Tool-chain safe: won't split assistant tool_call from its tool_result.
        """
        if len(messages) <= self.min_messages_to_compress:
            return messages

        head_n = self.protect_first_n
        tail_n = self.protect_last_n

        # ── Tool-chain safety: extend tail to include full chains ──
        # If the tail boundary splits a tool_call→tool_result pair,
        # extend tail backwards to include the orphan tool_result.
        if len(messages) > tail_n:
            boundary = len(messages) - tail_n
            # Check if we'd split a tool chain: if messages[boundary] is a
            # tool result, backtrack through preceding tool_call messages
            while boundary < len(messages) and messages[boundary].get("role") == "tool":
                boundary += 1
                tail_n += 1
            tail_n = min(tail_n, len(messages) - head_n)

        head = messages[:head_n]
        middle = messages[head_n:-tail_n] if tail_n > 0 else messages[head_n:]
        tail = messages[-tail_n:] if tail_n > 0 else []

        if len(middle) < 3:
            return messages  # too few to compress

        # Pack middle messages
        packed = json.dumps([{
            "role": m.get("role", ""),
            "content": str(m.get("content", ""))[:2000],
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
            f"Retrieve with: headroom_retrieve({hash_val}).\n"
            f"The {self.protect_last_n} messages below are your active context."
        )

        self.compression_count += 1
        _log.info(
            "context_engine: compressed %d msgs → CCR:%s (%s)",
            len(middle), hash_val[:8], size_str
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
        global _conv_index, _turn_counter
        _conv_index.clear()
        _turn_counter = 0
        _log.info("v1.7.0: session reset — inline store + memory cleared")

    def on_session_start(self, session_id="", **kw):
        self.session_id = session_id
        _log.info("context_engine: session %s started", session_id[:16] if session_id else "?")
