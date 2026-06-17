"""aphrodite — rebuild handler: build crate, kill proxies, replace binary, restart.

Dev mode: if Cargo.toml exists, builds from Rust source.
User mode (standalone install): re-downloads binary from GitHub Releases.
"""

import json
import logging
import os
import shutil
import subprocess
import time as _time

from .._core import BINARY, PORTS

_log = logging.getLogger("aphrodite.hooks.rebuild")


REBUILD_SCHEMA = {
    "name": "aphrodite_rebuild",
    "description": "Rebuild aphrodite crate from source and install binary. Use after code changes.",
    "parameters": {"type": "object", "properties": {}},
}


def _find_cargo_toml():
    """Walk up from this file to find Cargo.toml (Rust workspace root)."""
    d = os.path.dirname(os.path.abspath(__file__))
    for _ in range(6):  # _hooks → aphrodite → plugins → repo root (max 4, +2 safety)
        candidate = os.path.join(d, "Cargo.toml")
        if os.path.isfile(candidate):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            break
        d = parent
    return None


def _rebuild_handler(args=None, **kwargs):
    """Rebuild or re-download aphrodite binary."""
    repo = _find_cargo_toml()

    if repo is None:
        # Standalone install — no Rust source, re-download from releases
        _log.info("no Cargo.toml found — standalone install, downloading from releases")
        return _download_rebuild()

    # Dev mode — build from source
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "aphrodite"],
        cwd=repo,
        capture_output=True,
        text=True,
        timeout=300,
        env={**os.environ, "PATH": f"{os.path.expanduser('~/.cargo/bin')}:{os.environ.get('PATH', '')}"},
    )
    if result.returncode != 0:
        return f'{{"error": "build failed: {result.stderr[-200:]}"}}'

    src = os.path.join(repo, "target/release/aphrodite")
    if not os.path.exists(src):
        return '{"error": "binary not found after build"}'

    return _install_and_restart(src)


def _download_rebuild():
    """Re-download binary from GitHub Releases and restart proxies."""
    from .._binary import _download_binary

    if not _download_binary():
        return '{"error": "download failed — check network or GitHub Releases"}'

    killed = _kill_proxies()
    restarted = _restart_proxies()

    from .._proxy.health import _query_proxy_version
    proxy_ver = _query_proxy_version(PORTS["token"]) or "?"

    return json.dumps({
        "ok": True,
        "size": os.path.getsize(BINARY),
        "path": BINARY,
        "killed": killed,
        "restarted": restarted,
        "proxy_version": proxy_ver,
        "method": "download",
    })


def _kill_proxies():
    """Kill running proxy processes on configured ports."""
    killed = []
    for port in PORTS.values():
        try:
            r = subprocess.run(["lsof", "-ti", f":{port}"], capture_output=True, text=True, timeout=5)
            if r.stdout.strip():
                for pid in r.stdout.strip().split("\n"):
                    try:
                        os.kill(int(pid), 9)
                        killed.append(f":{port}({pid})")
                    except (OSError, ProcessLookupError):
                        pass
        except FileNotFoundError:
            killed.append(f":{port}(lsof-missing)")
        except Exception:
            pass
    return killed


def _restart_proxies():
    """Restart both proxy instances."""
    _time.sleep(0.3)
    restarted = []
    from .._proxy.lifecycle import _start as _proxy_start
    for name in ("cache", "token"):
        try:
            _proxy_start(name, os.environ.copy())
            restarted.append(name)
        except Exception:
            pass
    return restarted


def _install_and_restart(src):
    """Copy binary from build output to BINARY path, kill old proxies, restart."""
    killed = _kill_proxies()

    shutil.copy2(src, BINARY)
    os.chmod(BINARY, 0o755)

    _time.sleep(0.3)
    restarted = _restart_proxies()

    _time.sleep(0.3)
    from .._proxy.health import _query_proxy_version
    proxy_ver = _query_proxy_version(PORTS["token"]) or "?"

    return json.dumps({
        "ok": True,
        "size": os.path.getsize(BINARY),
        "path": BINARY,
        "killed": killed,
        "restarted": restarted,
        "proxy_version": proxy_ver,
        "method": "cargo",
    })
