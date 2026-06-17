"""aphrodite — rebuild handler: build crate, kill proxies, replace binary, restart."""

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


def _rebuild_handler(args=None, **kwargs):
    """Rebuild aphrodite crate, kill running proxies, replace binary, restart."""
    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
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

    # Kill running proxies
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

    # Replace binary
    shutil.copy2(src, BINARY)
    os.chmod(BINARY, 0o755)

    # Restart proxies
    _time.sleep(0.3)
    restarted = []
    from .._proxy import _query_proxy_version
    from .._proxy import _start as _proxy_start
    for name in ("cache", "token"):
        try:
            _proxy_start(name, os.environ.copy())
            restarted.append(name)
        except Exception:
            pass

    _time.sleep(0.3)
    proxy_ver = _query_proxy_version(PORTS["token"]) or "?"

    return json.dumps({
        "ok": True,
        "size": os.path.getsize(BINARY),
        "path": BINARY,
        "killed": killed,
        "restarted": restarted,
        "proxy_version": proxy_ver,
    })
