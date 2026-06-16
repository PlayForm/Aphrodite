"""aphrodite - proxy lifecycle (env loading, health checks, launch)."""

import json
import logging
import os
import subprocess
import time
import urllib.request
from pathlib import Path

from ._core import BINARY, ENV_FILE, PORTS

_log = logging.getLogger("aphrodite")

# ── Alive cache (5-second TTL) ──────────────────────────────
_alive_cache = {}  # {port: (result, timestamp)}


def _load_env():
    """Load .env file into a dict."""
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
    except Exception as exc:
        _log.warning("_load_env: failed to read %s - %s", ENV_FILE, exc)
    return env


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
    """Launch the aphrodite proxy binary."""
    port = PORTS[name]

    # ── Port conflict resolution ────────────────────────────
    try:
        r = subprocess.run(["lsof", "-ti", f":{port}"], capture_output=True, text=True, timeout=5)
        if r.stdout.strip():
            pid = r.stdout.strip()
            _log.warning("port %s in use by PID %s - killing it", port, pid)
            subprocess.run(["kill", pid], capture_output=True, timeout=3)
            time.sleep(0.2)  # brief grace for the port to free
    except FileNotFoundError:
        _log.warning("lsof not available - skipping port conflict check")
    except Exception as exc:
        _log.warning("port conflict check failed for :%s - %s", port, exc)

    # ── Launch ──────────────────────────────────────────────
    key = env.get("APHRODITE_API_KEY", "")
    if not key:
        _log.warning("APHRODITE_API_KEY not set in env - proxy won't authenticate")
        return
    mode_flag = "cache" if name == "cache" else "token"
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--api-key", key, "--mode", mode_flag, "--tool-relay"]
    _log.info("starting aphrodite %s on :%s", name, port)
    try:
        proc = subprocess.Popen(
            args,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception as e:
        _log.warning("aphrodite %s launch failed: %s", name, e)
        return

    # ── Write PID file ──────────────────────────────────────
    try:
        Path(f"/tmp/aphrodite-{name}.pid").write_text(str(proc.pid))
    except Exception as exc:
        _log.warning("failed to write PID file for %s - %s", name, exc)


def on_start(**kw):
    """Hermes session_start hook - ensure binary + launch proxy."""
    from ._binary import _ensure_binary

    if not _ensure_binary():
        _log.error("cannot start - binary not available")
        return
    env = {**os.environ, **_load_env()}
    for name in ("cache", "token"):
        if not _alive(PORTS[name]):
            _start(name, env)
    # Retry loop for proxy readiness
    cache_ok = _wait_alive(PORTS["cache"], retries=10, delay=0.3)
    token_ok = _wait_alive(PORTS["token"], retries=10, delay=0.3)
    _log.info("aphrodite: cache=%s token=%s", "UP" if cache_ok else "DOWN", "UP" if token_ok else "DOWN")


def _wait_alive(port, retries=10, delay=0.3):
    """Wait for proxy port to become alive, with retries."""
    for _ in range(retries):
        if _alive(port):
            return True
        time.sleep(delay)
    return False
