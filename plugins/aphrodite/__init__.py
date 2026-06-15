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
    key = env.get("DEEPSEEK_API_KEY", "")
    if not key:
        _log.warning("DEEPSEEK_API_KEY not set in env — proxy won't authenticate")
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

def register(ctx):
    # Install binary on registration
    _ensure_binary()
    ctx.register_hook("session_start", on_start)
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
    _log.info("aphrodite v%s registered — proxy + headroom_retrieve tool", VERSION)
