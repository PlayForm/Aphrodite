"""aphrodite - proxy launch, kill, and session lifecycle."""

import concurrent.futures
import logging
import os
import signal
import subprocess
import time
from pathlib import Path

from .._core import _DEV, BINARY, BINARY_DIR, BIN_VERSION, DEBUG_LOGGING, PLUGIN_VERSION, PORTS
from .env import _PROXY_ENV_KEYS, _inject_expand_guidance, _load_env
from .health import _alive, _alive_cache, _headroom_context, _query_proxy_version
from .markers import _restore_markers
from .startup import _write_startup_log

_log = logging.getLogger("aphrodite")

# ── Process tracking ─────────────────────────────────────────
_PROCS: dict[int, subprocess.Popen] = {}  # {port: Popen}


def _kill(pid: int | str, timeout: float = 0.3) -> None:
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


def _start(name: str, env: dict[str, str]) -> None:
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


def _wait_alive(port: int, retries: int = 10, delay: float = 0.3) -> bool:
    """Wait for proxy port to become alive, with retries."""
    for _ in range(retries):
        if _alive(port):
            return True
        time.sleep(delay)
    return False


def on_start(**kw) -> str | None:
    """Hermes session_start hook - ensure binary + launch proxy + auto-setup."""
    from .._binary import _ensure_binary

    if not _ensure_binary():
        _log.error("cannot start - binary not available")
        return None
    # Clear stale cache before checking (fixes stale state across session restarts)
    _alive_cache.clear()
    # Reset headroom context for the new session
    _headroom_context.clear()

    env = {**os.environ, **_load_env()}
    if not env.get("APHRODITE_API_KEY"):
        _log.warning("APHRODITE_API_KEY not found in environment - proxy won't start")
        return None
    # Launch both proxies — skip if alive AND version matches expected.
    # If a running proxy's version is stale, kill it and launch the new binary.
    # SQLite CCR store survives restarts (disk-backed), in-memory cache is rebuilt.
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        start_futs = {}
        for name in ("cache", "token"):
            port = PORTS[name]
            if _alive(port):
                running_ver = _query_proxy_version(port)
                if running_ver and BIN_VERSION in running_ver:
                    _log.debug("proxy %s already running expected version %s", name, running_ver)
                    continue
                _log.info(
                    "proxy %s version mismatch (running=%s, expected=%s) — restarting",
                    name, running_ver or "?", BIN_VERSION,
                )
                # Kill stale proxy
                try:
                    r = subprocess.run(
                        ["lsof", "-ti", f":{port}"],
                        capture_output=True, text=True, timeout=5,
                    )
                    if r.stdout.strip():
                        for pid in r.stdout.strip().split("\n"):
                            _kill(pid)
                except Exception as exc:
                    _log.warning("kill stale proxy %s failed: %s", name, exc)
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
    from .._automation import run_all

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

    # ── Restore recent markers from previous session ─────────────
    _restore_markers()

    # Startup observability log
    _write_startup_log(cache_ok, token_ok, auto_summary)

    return f"💋 aphrodite v{PLUGIN_VERSION}  -  cache={'UP' if cache_ok else 'DOWN'} token={'UP' if token_ok else 'DOWN'}"
