"""
aphrodite v1.2.0 — Auto-install + launch aphrodite proxies.
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold
- Recursive CCR resolution for nested markers
- Dynamic knowledge map with previews and cutoff

On install: downloads pre-built binary from GitHub releases.
On session_start: launches aphrodite proxies not already running.
"""
import os, subprocess, urllib.request, time, logging, platform, stat, re, json

PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
VERSION = "v0.2.0"
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")
# Dev mode: skip all proxy routing — use cargo watch instead
_DEV = os.environ.get("APHRODITE_DEV", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"
if _DEV:
    _log.warning("aphrodite DEV MODE — plugin disabled, use cargo watch for proxies")

# ── CCR regex (shared) ───────────────────────────────────────
_CCR_RE = re.compile(r'\[CCR:([^\]]+)\]')
_RECURSIVE_DEPTH = 3  # max nesting depth for recursive resolution



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
        f"https://github.com/{REPO}/releases/download/{VERSION}/aphrodite"
    )
    
    _log.info("downloading aphrodite %s from %s", VERSION, download_url)
    
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

def _resolve_one(hash_val, timeout=4):
    """Resolve a single CCR hash. Returns content string or None."""
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


def _resolve_recursive(hash_val, depth=0, resolved=None):
    """Recursively resolve CCR markers in content, up to max depth.
    
    After retrieving content, scans for nested [CCR:...] markers
    and resolves them in parallel, replacing markers with resolved content.
    """
    if resolved is None:
        resolved = {}
    
    if depth >= _RECURSIVE_DEPTH or hash_val in resolved:
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
    """Compress tool outputs at capture time via CCR. Token: >1KB, Cache: >8KB."""
    if not result or not isinstance(result, str) or not result.strip():
        return result

    if _DEV: return result  # dev mode: passthrough
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return result

    skip = {"read_file", "read_terminal"} if token_alive else {"read_file", "read_terminal", "execute_code", "memory", "patch", "write_file", "search_files", "todo"}
    if tool_name in skip:
        return result

    threshold = 1024 if token_alive else 8192
    if len(result) < threshold:
        return result

    target = PORTS["token"] if token_alive else PORTS["cache"]
    try:
        data = json.dumps({"content": result}).encode()
        req = urllib.request.Request(f"http://127.0.0.1:{target}/ccr/create", data=data, headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        h = ccr["hash"]
        p = result[:120].replace('\\n', ' ').strip()
        label = "token" if token_alive else "cache"
        return f'[CCR:{h}|tool|{len(result)}|{label}] {p}'
    except Exception as e:
        return result  # soft-fail


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate and copy binary to ~/.hermes/aphrodite/."""
    repo = "REDACTED/Developer/Application/PlayForm/HermesCompress"
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

_conv_index = {}  # {turn_id: (hash, summary, size)}


def _store_conversation_turn(conversation_history=None, assistant_response=None, turn_id=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    if not conversation_history or assistant_response is None:
        return

    if _DEV: return  # dev mode: passthrough
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]

    # Capture the last exchange (last user msg + assistant response)
    last_user = ""
    for msg in reversed(conversation_history):
        if msg.get("role") == "user":
            last_user = msg.get("content", "")[:200]
            break

    summary = f"Turn {turn_id}: {last_user}… → {str(assistant_response)[:200]}"

    try:
        data = json.dumps({"content": json.dumps({
            "turn": turn_id,
            "user": last_user,
            "assistant": str(assistant_response)[:5000],
        })}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())

        _conv_index[turn_id] = (ccr["hash"], summary, len(str(assistant_response)))
        if len(_conv_index) > 100:
            oldest = min(_conv_index.keys())
            del _conv_index[oldest]

        _log.debug("conv-cache: stored turn %s → %s (%d total)", turn_id, ccr["hash"][:8], len(_conv_index))
    except Exception:
        pass  # soft-fail — conversation memory is best-effort


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
    """Before LLM call: inject content map + memory index as context string.

    Hermes injects the returned string into the user message (not system prompt),
    preserving prompt-cache prefix. Mutations to conversation_history are lost
    (Hermes passes a copy), so we return a context string instead.
    """
    if _DEV: return  # dev mode: skip
    if not conversation_history or not isinstance(conversation_history, list):
        return

    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])

    # ── Build markers with previews + memory index ─────────
    markers = []
    total_bytes = 0
    for msg in conversation_history:
        content = msg.get("content", "")
        if isinstance(content, str):
            for m in _parse_ccr_markers(content):
                total_bytes += m['size']
                markers.append(m)

    map_parts = []
    if markers or _conv_index:
        map_parts.append("[APHRODITE]")
        if markers:
            mode_tag = "token" if token_alive else "cache" if cache_alive else "off"
            # Dynamic cutoff: show first 8 markers with previews, summarize rest
            visible = min(len(markers), 8)
            map_parts.append(f"  proxy={mode_tag} | {len(markers)} items @ {_fmt_size(total_bytes)} | retrieval: headroom_retrieve")
            if visible > 0:
                map_parts.append(f"  catalog ({visible}/{len(markers)}):")
                for i, m in enumerate(markers[:visible]):
                    # Extract a short preview from the marker context
                    # The preview is the text after the [CCR:...] marker in the original message
                    preview = ""
                    for msg in conversation_history:
                        c = msg.get("content", "")
                        if isinstance(c, str) and m['hash'] in c:
                            idx = c.find(m['hash'])
                            after = c[idx + len(m['hash']):].strip()
                            if after.startswith('|'):
                                after = after.split(']', 1)[-1] if ']' in after else after
                            preview = after[:80].strip()
                            break
                    map_parts.append(f"    [{i}] CCR:{m['hash'][:8]} | {m['type']} | {_fmt_size(m['size'])} | {preview}")
        if _conv_index:
            turns = sorted(_conv_index.items(), reverse=True)[:5]
            map_parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in turns))
        ctx_len = len(conversation_history)
        if ctx_len > 20:
            # Dynamic cutoff hint
            hint = "auto-compress active" if ctx_len > 50 else "consider compressing older turns"
            map_parts.append(f"  context: {ctx_len} msgs ({hint})")

    if map_parts:
        return "\n".join(map_parts)


def _api_request_hook(api_messages=None, api_request_id=None, **kwargs):
    """pre_api_request: compress old messages before API call.
    
    Hermes passes the actual API payload (messages list) which CAN be mutated.
    We offload messages older than the last 6 to CCR.
    """
    if _DEV: return  # dev mode: skip
    if not api_messages or not isinstance(api_messages, list):
        return

    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None
    
    if not target or len(api_messages) <= 8:
        return

    # Keep last 6 messages, offload older ones to CCR
    try:
        old_msgs = api_messages[:-6]
        packed = json.dumps([{
            "role": m.get("role", ""),
            "content": str(m.get("content", ""))[:2000]
        } for m in old_msgs])
        
        if len(packed) < 500:  # not worth compressing
            return
            
        data = json.dumps({"content": packed}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        
        # Replace old messages with a single compressed marker
        summary = f"[CCR:{ccr['hash']}|context|{len(packed)}|{len(old_msgs)}msgs] Context compressed: {len(old_msgs)} messages. Retrieve with headroom_retrieve."
        del api_messages[:-6]  # remove all but last 6
        api_messages.insert(0, {"role": "system", "content": summary})
        
        _log.debug("api_request: offloaded %d msgs → %s", len(old_msgs), ccr['hash'][:8])
    except Exception:
        pass  # soft-fail


def _transform_terminal_hook(command="", output="", returncode=0, **kwargs):
    """Compress terminal output via CCR on-the-fly."""
    if _DEV: return output  # dev mode: passthrough
    if not _alive(PORTS["token"]) and not _alive(PORTS["cache"]):
        return output  # no proxy, pass through

    if len(output) < 2048:  # 2KB min for terminal compression
        return output

    target = PORTS["token"] if _alive(PORTS["token"]) else PORTS["cache"]
    try:
        data = json.dumps({"content": output}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        hash_val = ccr["hash"]
        preview = output[:200].replace('\n', ' ').strip()
        return f'[CCR:{hash_val}|terminal|{len(output)}] {preview}…(use headroom_retrieve)'
    except Exception as e:
        _log.debug("terminal compress skipped: %s", e)
        return output


def _fmt_size(b):
    if b >= 1_000_000: return f"{b/1_000_000:.1f}MB"
    if b >= 1000: return f"{b/1000:.1f}KB"
    return f"{b}B"


def register(ctx):
    # Install binary on registration
    _ensure_binary()
    ctx.register_hook("on_session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("pre_api_request", _api_request_hook)
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
    _log.info("aphrodite v1.2.0 registered — recursive CCR + knowledge map + api_request compression")
