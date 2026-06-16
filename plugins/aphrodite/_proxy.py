"""aphrodite - proxy lifecycle (env loading, health checks, launch)."""

import concurrent.futures
import http.client
import json
import logging
import os
import signal
import subprocess
import time
from pathlib import Path

from ._core import _DEV, BINARY, BINARY_DIR, DEBUG_LOGGING, ENV_FILE, PORTS

_log = logging.getLogger("aphrodite")

# ── Process tracking ─────────────────────────────────────────
_PROCS: dict[int, subprocess.Popen] = {}  # {port: Popen}

# ── Alive cache (5-second TTL) ──────────────────────────────
_alive_cache = {}  # {port: (result, timestamp)}

# ── Turn-scoped alive cache (refreshed by pre_llm_hook each turn) ──
_alive_turn_cache: dict[int, bool] = {}  # {port: bool}

# ── Proxy environment keys (whitelist) ──────────────────────
_PROXY_ENV_KEYS = {"PATH", "HOME", "APHRODITE_API_KEY", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "SSL_CERT_FILE", "TMPDIR", "TMP", "TEMP"}

# ── Auto-expand guidance (set by on_start after proxy launch) ──
_expand_guidance: str = ""


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
                "_env_val: %s contains '#' followed by '%s...' - "
                "possible key truncation, consider quoting the value",
                key_name,
                after_stripped[:3],
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
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
        conn.request("GET", "/health", headers={"Connection": "close"})
        resp = conn.getresponse()
        body = resp.read().decode().strip()
        conn.close()
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


def _alive_cached(port: int) -> bool:
    """Check turn-scoped cache first, fall through to _alive()."""
    if port in _alive_turn_cache:
        return _alive_turn_cache[port]
    return _alive(port)


def _kill(pid, timeout=0.3):
    """Kill a process by PID with SIGTERM, escalate to SIGKILL after timeout."""
    try:
        os.kill(int(pid), 0)  # Probe - still alive?
    except (OSError, ProcessLookupError, ValueError):
        return  # Already dead or bogus PID
    for sig_nr in (signal.SIGTERM, signal.SIGKILL):
        try:  # noqa: SIM105 - intentional kill loop, not a context manager case
            os.kill(int(pid), sig_nr)
        except Exception:
            pass
        if sig_nr == signal.SIGKILL:
            break
        time.sleep(timeout)
        try:
            os.kill(int(pid), 0)
        except (OSError, ProcessLookupError):
            return  # SIGTERM worked
    # SIGKILL sent - reap via waitpid instead of busy-wait polling (up to 1s)
    pid_int = int(pid)
    deadline = time.monotonic() + 1.0
    while time.monotonic() < deadline:
        wpid, _ = os.waitpid(pid_int, os.WNOHANG)
        if wpid == pid_int:
            return  # Reaped
        time.sleep(0.05)


def _start(name, env):
    """Launch the aphrodite proxy binary."""
    port = PORTS[name]

    # ── Ensure log directory exists ──────────────────────────
    os.makedirs(BINARY_DIR, exist_ok=True)

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
    key = os.environ.get("APHRODITE_API_KEY", env.get("APHRODITE_API_KEY", ""))
    if not key:
        raise ValueError(
            "APHRODITE_API_KEY not set in env or .env - proxy can't authenticate"
        )
    env["APHRODITE_API_KEY"] = key
    mode_flag = "cache" if name == "cache" else "token"
    args = [BINARY, "--listen", f"127.0.0.1:{port}", "--mode", mode_flag, "--tool-relay"]
    _log.info("starting aphrodite %s on :%s", name, port)

    # ── Binary guard ──────────────────────────────────────
    if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
        _log.warning("aphrodite %s: binary not executable at %s", name, BINARY)
        return

    log_path = os.path.join(BINARY_DIR, f"proxy-{name}.log")
    try:
        proc = subprocess.Popen(
            args,
            env={k: env[k] for k in _PROXY_ENV_KEYS if k in env},
            stdout=open(log_path, "a"),  # noqa: SIM115 - daemon needs open handle, not context manager
            stderr=subprocess.STDOUT,
            stdin=subprocess.DEVNULL,
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


def _write_startup_log(cache_ok, token_ok, auto_summary):
    """Write structured startup log to ~/.hermes/aphrodite/startup-<ts>.log."""
    from ._core import BIN_VERSION, PLUGIN_VERSION

    ts = int(time.time())
    log_path = os.path.expanduser(f"~/.hermes/aphrodite/startup-{ts}.log")
    try:
        lines = [
            f"=== aphrodite startup [{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(ts))}] ===",
            f"plugin_version={PLUGIN_VERSION}  binary_version={BIN_VERSION}",
            f"proxy_cache={'UP' if cache_ok else 'DOWN'}  proxy_token={'UP' if token_ok else 'DOWN'}",
            f"env: APHRODITE_DEBUG={os.environ.get('APHRODITE_DEBUG', '')}",
            f"env: QUIET={os.environ.get('QUIET', '')}",
            f"env: APHRODITE_CONTEXT_ENGINE={os.environ.get('APHRODITE_CONTEXT_ENGINE', '')}",
        ]
        if auto_summary:
            lines.append("--- auto ---")
            lines.append(auto_summary)
        with open(log_path, "w") as f:
            f.write("\n".join(lines) + "\n")
        _log.debug("startup log written: %s", log_path)
    except Exception as exc:
        _log.warning("failed to write startup log: %s", exc)


def _inject_expand_guidance():
    """Return auto-expand guidance string explaining that tool CCR markers are resolved inline."""
    return (
        "[APHRODITE] Tool outputs are auto-expanded - you see full content inline, "
        "no <<<CCR:...>>> markers for tool results. "
        "If you ever see a CCR marker (for context or terminal output), use "
        "aphrodite_retrieve(hash) to fetch it."
    )


def on_start(**kw):
    """Hermes session_start hook - ensure binary + launch proxy + auto-setup."""
    from ._binary import _ensure_binary

    if not _ensure_binary():
        _log.error("cannot start - binary not available")
        return
    # Clear stale cache before checking (fixes stale state across session restarts)
    _alive_cache.clear()

    env = {**os.environ, **_load_env()}
    if not env.get("APHRODITE_API_KEY"):
        _log.warning("APHRODITE_API_KEY not found in environment - proxy won't start")
        return
    # Launch both proxies concurrently (skip if already alive)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        start_futs = {}
        for name in ("cache", "token"):
            if not _alive(PORTS[name]):
                start_futs[name] = pool.submit(_start, name, env)
        for name, fut in start_futs.items():
            try:
                fut.result()
            except Exception as exc:
                _log.warning("_start(%s) failed: %s", name, exc)
    # Retry loop for proxy readiness (concurrent - cuts worst-case 6s→3s)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        cache_fut = pool.submit(_wait_alive, PORTS["cache"], retries=10, delay=0.3)
        token_fut = pool.submit(_wait_alive, PORTS["token"], retries=10, delay=0.3)
        cache_ok = cache_fut.result()
        token_ok = token_fut.result()

    # ── Auto-setup: run all auto checks and display ────────────
    from ._automation import run_all

    auto_summary = run_all()
    if auto_summary:
        _log.debug(auto_summary)
        if DEBUG_LOGGING:
            print(auto_summary)

    # Auto-expand guidance: explain inline resolution to the LLM,
    # skipped in dev/passthrough mode or if no proxy came up
    global _expand_guidance
    if not _DEV and (cache_ok or token_ok):
        _expand_guidance = _inject_expand_guidance()
        _log.debug("expand guidance set (%d chars)", len(_expand_guidance))

    _log.info("aphrodite: cache=%s token=%s", "UP" if cache_ok else "DOWN", "UP" if token_ok else "DOWN")

    # Startup observability log
    _write_startup_log(cache_ok, token_ok, auto_summary)


def _wait_alive(port, retries=10, delay=0.3):
    """Wait for proxy port to become alive, with retries."""
    for _ in range(retries):
        if _alive(port):
            return True
        time.sleep(delay)
    return False
