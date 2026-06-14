"""
hermes-proxy-token v1.0.0 — Auto-start headroom TOKEN proxy on session start.
Port :8788, full compression (SmartCrusher + Kompress).

Toggle: hermes plugins enable/disable hermes-proxy-token
"""
import os, subprocess, urllib.request, json, logging

PORT = 8788
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LAUNCHER = os.path.join(REPO, "scripts", "proxy-token.sh")
_log = logging.getLogger("hermes-proxy-token")

def _alive():
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2)
        return json.loads(r.read()).get("status") == "healthy"
    except Exception:
        return False

def on_start(**kw):
    if _alive():
        return
    _log.info("starting token proxy on :%s", PORT)
    try:
        subprocess.Popen(["bash", LAUNCHER], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    except Exception as e:
        _log.warning("failed: %s", e)

def register(ctx):
    ctx.register_hook("session_start", on_start)
    _log.info("token proxy plugin registered")
