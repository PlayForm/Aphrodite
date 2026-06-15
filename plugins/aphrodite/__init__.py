"""
aphrodite v1.1.0 — Auto-install + launch aphrodite proxies.
- Cache (:9797): in-memory CCR, >8KB threshold
- Token (:9798): SQLite CCR, tool relay, >1KB threshold

On install: downloads pre-built binary from GitHub releases.
On session_start: launches aphrodite proxies not already running.
"""
import os, subprocess, urllib.request, time, logging, platform, stat

PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
VERSION = "v0.2.0"
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")


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


def _alive(port):
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2)
        return r.read().decode().strip() == "ok"
    except Exception:
        return False


def _start(name, env):
    port = PORTS[name]
    key = env.get("APHRODITE_API_KEY", "")
    if not key:
        _log.warning("APHRODITE_API_KEY not set in env — proxy won't authenticate")
        return
    
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--deepseek-key", key]
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



def _retrieve_handler(args=None, **kwargs):
    """Resolve CCR markers to original content via aphrodite proxy."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "")
    if not hash_val:
        return '{"error": "missing hash parameter"}'
    try:
        import urllib.request, json
        data = json.dumps({"hash": hash_val}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/retrieve",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            result = json.loads(r.read())
        if result.get("found"):
            return result["content"]
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
        import urllib.request, json
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
    "description": "Resolve CCR markers to original content via aphrodite proxy.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker hash to retrieve"}
        },
        "required": ["hash"]
    }
}



def _transform_tool_result(
    tool_name="", args=None, result="", tool_call_id="",
    task_id="", session_id="", turn_id="", api_request_id="",
    duration_ms=0, status="", error_type="", error_message="",
    **kwargs,
):
    """Compress large tool outputs at capture time via CCR."""
    if not result or not isinstance(result, str) or not result.strip():
        return result

    # Never compress debug/read tools — keep output visible
    skip_tools = {"read_file", "read_terminal", "execute_code", "memory", "patch", "write_file", "search_files", "todo"}
    if tool_name in skip_tools:
        return result

    # Only compress outputs > 8KB
    if len(result) < 8192:
        return result

    try:
        import urllib.request, json
        data = json.dumps({"content": result}).encode()
        req = urllib.request.Request(
            "http://127.0.0.1:9798/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr_result = json.loads(r.read())
        hash_val = ccr_result["hash"]
        # Return a marker the LLM can retrieve
        preview = result[:120].splitlines()[0].strip() if result else ''
        ct = 'tool_output'
        size = len(result)
        return f'<<CCR:{hash_val}|{ct}|{size}>> {preview}'
    except Exception as e:
        _log.debug("transform_tool_result compression skipped: %s", e)
        return result


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate and copy binary to ~/.hermes/aphrodite/."""
    import subprocess, shutil, os
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

_conv_index = {}  # {turn_number: (hash, summary, size)}

def _store_conversation_turn(api_messages=None, response=None, turn_number=0, **kwargs):
    """Post-LLM-call: store the current exchange in CCR for later retrieval."""
    if not api_messages or response is None:
        return

    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    if not token_alive and not cache_alive:
        return

    target = PORTS["token"] if token_alive else PORTS["cache"]

    # Capture the last exchange (last user msg + assistant response)
    last_user = ""
    for msg in reversed(api_messages):
        if msg.get("role") == "user":
            last_user = msg.get("content", "")[:200]
            break

    summary = f"Turn {turn_number}: {last_user} → {str(response)[:200]}"

    try:
        import urllib.request, json
        data = json.dumps({"content": json.dumps({
            "turn": turn_number,
            "user": last_user,
            "assistant": str(response)[:5000],
        })}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data, headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=2) as r:
            ccr = json.loads(r.read())

        _conv_index[turn_number] = (ccr["hash"], summary, len(str(response)))
        if len(_conv_index) > 100:
            oldest = min(_conv_index.keys())
            del _conv_index[oldest]

        _log.debug("conv-cache: stored turn %d → %s (%d total)", turn_number, ccr["hash"][:8], len(_conv_index))
    except Exception:
        pass  # soft-fail — conversation memory is best-effort


def _pre_llm_hook(api_messages=None, response=None, **kwargs):
    """Before LLM call: compress old messages, inject content map + memory index."""
    if not api_messages or not isinstance(api_messages, list):
        return

    # ── 1. Offload old messages to CCR ──────────────────────────
    token_alive = _alive(PORTS["token"])
    cache_alive = _alive(PORTS["cache"])
    target = PORTS["token"] if token_alive else PORTS["cache"] if cache_alive else None

    # Keep last 6 messages in context, offload older ones to CCR
    if target and len(api_messages) > 8:
        try:
            import urllib.request, json
            old_msgs = api_messages[:-6]  # everything except last 6
            packed = json.dumps([{"role": m.get("role",""), "content": str(m.get("content",""))[:1000]} for m in old_msgs])
            if len(packed) > 200:  # only if worth compressing
                data = json.dumps({"content": packed}).encode()
                req = urllib.request.Request(f"http://127.0.0.1:{target}/ccr/create", data=data, headers={"Content-Type": "application/json"})
                with urllib.request.urlopen(req, timeout=2) as r:
                    ccr = json.loads(r.read())
                # Replace old messages with a single marker
                summary = f"[Context compressed: {len(old_msgs)} messages → CCR:{ccr['hash']}|{ccr['compression_ratio']:.0f}x]"
                for _ in old_msgs:
                    api_messages.pop(0)
                api_messages.insert(0, {"role": "system", "content": summary})
                _log.debug("pre_llm: offloaded %d old msgs → %s", len(old_msgs), ccr['hash'][:8])
        except Exception:
            pass  # soft-fail

    # ── 2. Build markers + memory index ─────────────────────────
    markers = []
    total_bytes = 0
    import re
    for msg in api_messages:
        content = msg.get("content", "")
        if isinstance(content, str):
            found = re.findall(r'\[CCR:([^\]]+)\]', content)
            for m in found:
                parts = m.split('|')
                if len(parts) >= 3:
                    try:
                        sz = int(parts[2])
                        total_bytes += sz
                        markers.append({'hash': parts[0], 'type': parts[1], 'size': sz, 'mode': parts[3] if len(parts) > 3 else '?'})
                    except ValueError: pass

    map_parts = []
    if markers or _conv_index:
        map_parts.append("[APHRODITE]")
        if markers:
            mode_tag = f"token" if token_alive else f"cache" if cache_alive else "off"
            map_parts.append(f"  proxy={mode_tag} | {len(markers)} items @ {_fmt_size(total_bytes)} | retrieval: headroom_retrieve")
        if _conv_index:
            turns = sorted(_conv_index.items(), reverse=True)[:5]
            map_parts.append("  memory: " + " | ".join(f"T{t}" for t, _ in turns))
        if len(api_messages) > 20:
            map_parts.append(f"  context: {len(api_messages)} msgs (auto-compress active)")

    if map_parts:
        msg = "\n".join(map_parts)
        for i in range(len(api_messages)-1, -1, -1):
            if api_messages[i].get("role") == "user":
                api_messages.insert(i, {"role": "system", "content": msg, "ephemeral": True})
                break
def _fmt_size(b):
    if b >= 1_000_000: return f"{b/1_000_000:.1f}MB"
    if b >= 1000: return f"{b/1000:.1f}KB"
    return f"{b}B"



def _transform_terminal_hook(command="", stdout="", stderr="", exit_code=0, **kwargs):
    """Compress terminal output via CCR on-the-fly."""
    if not _alive(PORTS["token"]) and not _alive(PORTS["cache"]):
        return stdout  # no proxy, pass through

    combined = stdout
    if stderr:
        combined += "\n[stderr]\n" + stderr

    if len(combined) < 2048:  # 2KB min for terminal compression
        return stdout

    target = PORTS["token"] if _alive(PORTS["token"]) else PORTS["cache"]
    try:
        import urllib.request, json
        data = json.dumps({"content": combined}).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{target}/ccr/create",
            data=data,
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=3) as r:
            ccr = json.loads(r.read())
        hash_val = ccr["hash"]
        preview = combined[:200].replace('\n', ' ').strip()
        return f'[CCR:{hash_val}|terminal|{len(combined)}] {preview}...(use headroom_retrieve)'
    except Exception as e:
        _log.debug("terminal compress skipped: %s", e)
        return stdout

def register(ctx):
    # Install binary on registration
    _ensure_binary()
    ctx.register_hook("session_start", on_start)
    ctx.register_hook("pre_llm_call", _pre_llm_hook)
    ctx.register_hook("transform_terminal_output", _transform_terminal_hook)
    ctx.register_hook("post_llm_call", _store_conversation_turn)
    ctx.register_hook("transform_tool_result", _transform_tool_result)
    ctx.register_tool(
        name="aphrodite_rebuild",
        toolset="aphrodite",
        schema=REBUILD_SCHEMA,
        handler=_rebuild_handler,
    )
    ctx.register_tool(
        name="headroom_compress",
        toolset="aphrodite",
        schema=COMPRESS_SCHEMA,
        handler=_compress_handler,
    )
    ctx.register_tool(
        name="headroom_retrieve",
        toolset="aphrodite",
        schema=RETRIEVE_SCHEMA,
        handler=_retrieve_handler,
    )
    _log.info("aphrodite v%s registered — proxy + headroom_retrieve tool", VERSION)
