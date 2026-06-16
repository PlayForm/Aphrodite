"""aphrodite - proxy lifecycle (env loading, health checks, launch)."""

import json
import logging
import os
import socket
import subprocess
import time
from pathlib import Path

from ._core import BINARY, BINARY_DIR, ENV_FILE, PORTS

_log = logging.getLogger("aphrodite")

# ── Process tracking ─────────────────────────────────────────
_PROCS: dict[int, subprocess.Popen] = {}  # {port: Popen}

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
                        env[kv[0]] = _env_val(kv[1], kv[0])
                elif "=" in line and not line.startswith("#"):
                    kv = line.split("=", 1)
                    env[kv[0]] = _env_val(kv[1], kv[0])
    except Exception as exc:
        _log.warning("_load_env: failed to read %s - %s", ENV_FILE, exc)
    return env


def _env_val(val, key_name=""):
    """Parse a .env value: extract between matching quotes or strip inline # comment.

    If ``key_name`` is supplied and the value contains ``#`` followed by hex-like
    characters (common in truncated API keys), a warning is logged instead of silently
    stripping what may be part of the credential.
    """
    val = val.strip()
    if val.startswith('"'):
        end = val.find('"', 1)
        if end != -1:
            return val[1:end]
    elif val.startswith("'"):
        end = val.find("'", 1)
        if end != -1:
            return val[1:end]
    # Unquoted: split on # to remove inline comment
    if "#" in val:
        before, after = val.split("#", 1)
        after_stripped = after.strip()
        # If the suffix looks like a credential fragment (≥4 hex-like chars) warn
        if key_name and after_stripped and len(after_stripped) >= 4:
            _log.warning(
                "_env_val: %s contains '#' followed by '%s' - "
                "possible key truncation, consider quoting the value",
                key_name,
                after_stripped[:20],
            )
        return before.strip()
    return val


def _alive(port, timeout=3):
    """Check proxy health with 5-second caching to avoid socket overhead."""
    now = time.time()
    if port in _alive_cache:
        result, ts = _alive_cache[port]
        if now - ts < 5:
            return result
    try:
        sock = socket.create_connection(("127.0.0.1", port), timeout=timeout)
        with sock:
            sock.sendall(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
            body = b""
            while True:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                body += chunk
        # Extract body after the HTTP headers
        if b"\r\n\r\n" in body:
            body = body.split(b"\r\n\r\n", 1)[1].decode().strip()
        else:
            body = body.decode().strip()
        if not body:
            result = True
        else:
            try:
                data = json.loads(body)
                result = data.get("status") in ("healthy", "ok", "degraded")
            except Exception:
                result = body.strip() == "ok"
    except Exception:
        result = False
    _alive_cache[port] = (result, now)
    return result


def _kill(pid, timeout=0.3):
    """Kill a process by PID with SIGTERM, escalate to SIGKILL after timeout."""
    try:
        os.kill(int(pid), 0)  # Probe - still alive?
    except (OSError, ProcessLookupError, ValueError):
        return  # Already dead or bogus PID
    for sig in ("TERM", "KILL"):
        try:
            subprocess.run(["kill", f"-{sig}", str(pid)], capture_output=True, timeout=3)
        except Exception:
            pass
        if sig == "KILL":
            break
        time.sleep(timeout)
        try:
            os.kill(int(pid), 0)
        except (OSError, ProcessLookupError):
            return  # SIGTERM worked
    # SIGKILL sent, brief grace
    time.sleep(0.1)


def _start(name, env):
    """Launch the aphrodite proxy binary."""
    port = PORTS[name]

    # ── Stale PID check ────────────────────────────────────
    try:
        pid_path = Path(os.path.join(BINARY_DIR, f"proxy-{name}.pid"))
        if pid_path.exists():
            old_pid = int(pid_path.read_text().strip())
            try:
                os.kill(old_pid, 0)  # Process alive?
                _log.warning("stale PID %s for %s - killing it", old_pid, name)
                _kill(old_pid)
            except (OSError, ProcessLookupError):
                pass  # already dead
            pid_path.unlink(missing_ok=True)
    except Exception as exc:
        _log.warning("stale PID check failed for %s - %s", name, exc)

    # ── Port conflict resolution ────────────────────────────
    try:
        r = subprocess.run(["lsof", "-ti", f":{port}"], capture_output=True, text=True, timeout=5)
        if r.stdout.strip():
            pid = r.stdout.strip()
            _log.warning("port %s in use by PID %s - killing it", port, pid)
            _kill(pid)
    except FileNotFoundError:
        _log.warning("lsof not available - skipping port conflict check")
    except Exception as exc:
        _log.warning("port conflict check failed for :%s - %s", port, exc)

    # ── Launch ──────────────────────────────────────────────
    # Lazy key resolution: read fresh each call instead of caching at import time.
    # Checks os.environ first (per-request freshness), then env dict (pre-loaded .env),
    # then falls back to loading .env file directly.
    key = os.environ.get("APHRODITE_API_KEY", env.get("APHRODITE_API_KEY", ""))
    if not key:
        key = _load_env().get("APHRODITE_API_KEY", "")
    if not key:
        _log.warning("APHRODITE_API_KEY not set in env - proxy won't authenticate")
        return
    # Pass API key as environment variable instead of CLI arg so it's not visible in ps aux
    env["APHRODITE_API_KEY"] = key
    mode_flag = "cache" if name == "cache" else "token"
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--mode", mode_flag, "--tool-relay"]
    _log.info("starting aphrodite %s on :%s", name, port)

    # ── Binary guard ──────────────────────────────────────
    if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
        _log.warning("aphrodite %s: binary not executable at %s", name, BINARY)
        return

    try:
        proc = subprocess.Popen(
            args,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception as e:
        _log.warning("aphrodite %s launch failed: %s", name, e)
        return

    _PROCS[port] = proc

    # ── Write PID file ──────────────────────────────────────
    try:
        Path(os.path.join(BINARY_DIR, f"proxy-{name}.pid")).write_text(str(proc.pid))
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
