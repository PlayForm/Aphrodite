"""
hermes-proxy-aphrodite v1.0.0 — Auto-start aphrodite proxies on session start.
- :9797 — aphrodite cache mode (in-memory CCR, >8KB threshold)
- :9798 — aphrodite token mode (SQLite CCR, tool relay, >1KB threshold)

Reads keys from ~/.hermes/.env.
"""
import os, subprocess, urllib.request, time, logging

PORTS = {"cache": 9797, "token": 9798}
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LAUNCHERS = {
    "cache": os.path.join(REPO, "scripts", "proxy-9797.sh"),
    "token": os.path.join(REPO, "scripts", "proxy-9798.sh"),
}
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("hermes-proxy-aphrodite")


def _load_env():
    env = {}
    try:
        with open(ENV_FILE) as f:
            for line in f:
                line = line.strip()
                if line.startswith("export "):
                    kv = line[7:].split("=", 1)
                    if len(kv) == 2:
                        env[kv[0]] = kv[1].strip('"\').strip("'\"")
                elif "=" in line and not line.startswith("#"):
                    kv = line.split("=", 1)
                    env[kv[0]] = kv[1].strip('"\').strip("'\"")
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
    script = LAUNCHERS[name]
    port = PORTS[name]
    _log.info("starting aphrodite %s on :%s", name, port)
    try:
        subprocess.Popen(
            ["bash", script],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True, env=env, cwd=REPO,
        )
    except Exception as e:
        _log.warning("aphrodite %s launch failed: %s", name, e)


def on_start(**kw):
    env = {**os.environ, **_load_env()}
    for name in ("cache", "token"):
        if not _alive(PORTS[name]):
            _start(name, env)
    time.sleep(0.5)
    cache_ok = _alive(9797)
    token_ok = _alive(9798)
    _log.info("aphrodite: cache=%s token=%s", "UP" if cache_ok else "DOWN", "UP" if token_ok else "DOWN")


def register(ctx):
    ctx.register_hook("session_start", on_start)
    _log.info("hermes-proxy-aphrodite registered (:9797 cache + :9798 token)")
