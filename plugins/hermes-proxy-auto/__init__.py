"""
hermes-proxy-auto v2.0.0 — Auto-start ALL headroom proxies on session start.
- :8787 — Python cache (code-aware)
- :8788 — Rust token (CCR + tool injection)
- :9797 — Rust cache (in-memory CCR)
- :9798 — Rust token (CCR + tool relay)

Reads keys from ~/.hermes/.env and passes them to proxy subprocess.
"""
import os, subprocess, urllib.request, json, logging, time

PORTS = {
    "cache_8787": 8787,
    "token_8788": 8788,
    "cache_9797": 9797,
    "token_9798": 9798,
}

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LAUNCHERS = {
    "cache_8787": (os.path.join(REPO, "scripts", "proxy-start.py"), ["--port", "8787", "--mode", "cache"]),
    "token_8788": (os.path.join(REPO, "scripts", "proxy-token.sh"), []),
    "cache_9797": (os.path.join(REPO, "scripts", "proxy-9797.sh"), []),
    "token_9798": (os.path.join(REPO, "scripts", "proxy-9798.sh"), []),
}
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("hermes-proxy-auto")


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
        body = r.read().decode().strip()
        return body == "ok"
    except Exception:
        return False


def _start(name, env):
    script, args = LAUNCHERS[name]
    port = PORTS[name]
    _log.info("starting %s on :%s", name, port)
    try:
        if script.endswith(".py"):
            subprocess.Popen(
                ["python3", script] + args,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True, env=env, cwd=REPO,
            )
        else:
            subprocess.Popen(
                ["bash", script] + args,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True, env=env, cwd=REPO,
            )
    except Exception as e:
        _log.warning("%s launch failed: %s", name, e)


def on_start(**kw):
    env = {**os.environ, **_load_env()}
    for name in PORTS:
        if not _alive(PORTS[name]):
            _start(name, env)
    time.sleep(0.5)
    status = " ".join(
        f"{n}=UP" if _alive(p) else f"{n}=DOWN"
        for n, p in PORTS.items()
    )
    _log.info("hermes-proxy-auto: %s", status)


def register(ctx):
    ctx.register_hook("session_start", on_start)
    _log.info("hermes-proxy-auto v2 registered (4 proxies)")
