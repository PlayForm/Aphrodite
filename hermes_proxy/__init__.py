"""
hermes-proxy-auto v1.2.0 — Auto-start headroom CACHE proxy on session start.
Reads keys from ~/.hermes/.env and passes them to proxy subprocess.
Port :8787, --no-optimize (prefix-freeze only).
"""
import os, subprocess, urllib.request, json, logging

PORT = 8787
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAUNCHER = os.path.join(REPO, "scripts", "proxy-cache.sh")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("hermes-proxy-auto")

def _load_env():
    """Parse ~/.hermes/.env and return a dict of exported vars."""
    env = {}
    try:
        with open(ENV_FILE) as f:
            for line in f:
                line = line.strip()
                if line.startswith("export "):
                    kv = line[7:].split("=", 1)
                    if len(kv) == 2:
                        env[kv[0]] = kv[1].strip('"').strip("'")
    except Exception:
        pass
    return env

def _alive():
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2)
        return json.loads(r.read()).get("status") == "healthy"
    except Exception:
        return False

def on_start(**kw):
    if _alive():
        return
    _log.info("starting cache proxy on :%s", PORT)
    env = {**os.environ, **_load_env()}
    try:
        subprocess.Popen(["bash", LAUNCHER], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True, env=env)
    except Exception as e:
        _log.warning("failed: %s", e)

def register(ctx):
    ctx.register_hook("session_start", on_start)
    _log.info("cache proxy plugin registered")
