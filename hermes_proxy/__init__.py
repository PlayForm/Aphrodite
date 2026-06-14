"""
hermes-proxy-auto v1.0.0 — Start headroom cache proxy on session start.

Minimal plugin: no monkey-patching, no tools, just lifecycle hooks.
Symlinked from ~/.hermes/plugins/hermes-proxy-auto/ → this directory.
"""
import os, subprocess, urllib.request, json, logging

PROXY_PORT = 8787
LAUNCHER = os.path.join(os.path.dirname(__file__), "..", "..", "scripts", "proxy-cache.sh")
_log = logging.getLogger("hermes-proxy-auto")

def _proxy_alive():
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{PROXY_PORT}/health", timeout=2)
        return json.loads(r.read()).get("status") == "healthy"
    except Exception:
        return False

def on_session_start(**kwargs):
    if _proxy_alive():
        _log.info("headroom proxy already running on :%s", PROXY_PORT)
        return
    _log.info("starting headroom proxy on :%s", PROXY_PORT)
    try:
        subprocess.Popen(
            ["bash", LAUNCHER],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception as e:
        _log.warning("failed to start proxy: %s", e)

def on_session_end(**kwargs):
    pass  # keep proxy alive across sessions

def register(ctx):
    ctx.register_hook("session_start", on_session_start)
    ctx.register_hook("session_end", on_session_end)
    _log.info("proxy-auto plugin registered")
