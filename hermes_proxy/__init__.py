"""
@playform/hermes-proxy - Headroom proxy manager for Hermes Agent.

Starts/stops/configures the headroom proxy server as a standalone
process that intercepts API calls for transparent compression.

No hooks, no per-tool compression - just proxy lifecycle management
and custom_providers configuration.

Companion to @playform/hermes-compress (hook-based compression).
Both share `hermes_compress` logging channel for unified monitoring.
"""

from __future__ import annotations

import logging
import shutil
import subprocess

logger = logging.getLogger("hermes_compress")

__version__ = "0.1.0"

_PROXY_PORTS: dict[str, int] = {"cache": 8787, "token": 8788}


def _find_headroom_bin() -> str | None:
    path = shutil.which("headroom")
    if path:
        return path
    try:
        import headroom
        return shutil.which("headroom") or "headroom"
    except ImportError:
        return None


def register(ctx):
    """Hermes plugin entry point - registers proxy tools."""

    ctx.register_tool(
        name="headroom_proxy_start",
        toolset="compression",
        schema={
            "type": "function",
            "function": {
                "name": "headroom_proxy_start",
                "description": "Start the headroom proxy server and configure Hermes to route through it",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["cache", "token"],
                            "description": "Proxy mode: cache (full pipeline) or token (SmartCrusher only)",
                            "default": "cache",
                        },
                        "port": {
                            "type": "integer",
                            "description": "Port override (default: 8787 cache, 8788 token)",
                        },
                    },
                },
            },
        },
        handler=_handle_proxy_start,
        description="Start headroom proxy (cache or token mode)",
    )

    ctx.register_tool(
        name="headroom_proxy_stop",
        toolset="compression",
        schema={
            "type": "function",
            "function": {
                "name": "headroom_proxy_stop",
                "description": "Stop any running headroom proxy servers",
                "parameters": {"type": "object", "properties": {}},
            },
        },
        handler=_handle_proxy_stop,
        description="Stop headroom proxy servers",
    )

    ctx.register_tool(
        name="headroom_proxy_status",
        toolset="compression",
        schema={
            "type": "function",
            "function": {
                "name": "headroom_proxy_status",
                "description": "Check headroom proxy health, mode, and configuration",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["cache", "token", "all"],
                            "description": "Which proxy to check",
                            "default": "all",
                        },
                    },
                },
            },
        },
        handler=_handle_proxy_status,
        description="Check proxy health and configuration",
    )


def _handle_proxy_start(args=None, **kwargs) -> str:
    import json
    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "cache")
    port = args.get("port") or _PROXY_PORTS.get(mode, 8787)

    hb = _find_headroom_bin()
    if not hb:
        return json.dumps({"error": "headroom binary not found in PATH"})

    # Kill existing proxy on this port
    subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)

    try:
        proc = subprocess.Popen(
            [hb, "proxy", "--port", str(port), "--mode", mode],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        logger.info("hermes-proxy: started %s mode on :%d (pid=%d)", mode, port, proc.pid)
    except Exception as e:
        return json.dumps({"error": f"Failed to start proxy: {e}"})

    # Wait briefly for readiness
    import time
    time.sleep(1.0)

    try:
        import urllib.request
        url = f"http://127.0.0.1:{port}/health"
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=3) as resp:
            healthy = resp.status == 200
    except Exception:
        healthy = False

    result = {
        "mode": mode,
        "port": port,
        "pid": proc.pid,
        "healthy": healthy,
        "provider": f"custom:headroom-{mode}",
        "base_url": f"http://127.0.0.1:{port}",
    }

    return json.dumps(result, indent=2)


def _handle_proxy_stop(args=None, **kwargs) -> str:
    import json
    import signal
    import time

    stopped = []
    for mode, port in _PROXY_PORTS.items():
        try:
            import urllib.request
            resp = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2)
            healthy = resp.status == 200
        except Exception:
            healthy = False

        if healthy:
            subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)
            time.sleep(0.3)
            stopped.append({"mode": mode, "port": port, "was_running": True})

    logger.info("hermes-proxy: stopped %d proxy(s)", len(stopped))
    return json.dumps({"stopped": stopped}, indent=2)


def _handle_proxy_status(args=None, **kwargs) -> str:
    import json
    args = args if isinstance(args, dict) else {}
    target = args.get("mode", "all")

    result = {}
    for mode, port in _PROXY_PORTS.items():
        if target not in (mode, "all"):
            continue
        try:
            import urllib.request
            resp = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2)
            healthy = resp.status == 200
        except Exception:
            healthy = False
        result[mode] = {"port": port, "healthy": healthy}

    return json.dumps(result, indent=2)
