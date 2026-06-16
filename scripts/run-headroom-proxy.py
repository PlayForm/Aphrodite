#!/usr/bin/env python3
"""
Launch headroom as a standalone caching proxy server.

Two modes available, run one or both:

    # Cache mode (port 9799) — response caching, saves API costs
    python3 scripts/run-headroom-proxy.py cache

    # Token mode (port 9800) — full compression pipeline with CCR
    python3 scripts/run-headroom-proxy.py token

Environment:
    HEADROOM_DEEPSEEK_KEY — your DeepSeek API key
    (falls back to DEEPSEEK_API_KEY, then APHRODITE_API_KEY)

This runs the original headroom proxy from vendor/headroom/,
completely separate from the aphrodite Rust proxy on :9797/:9798.
"""
import os
import sys
import subprocess

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HEADROOM_VENDOR = os.path.join(REPO_ROOT, "vendor", "headroom")

def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "cache"

    configs = {
        "cache": {
            "port": 9799,
            "desc": "headroom-cache (response caching only)",
            "flags": ["--no-optimize", "--no-ccr-marker"],
        },
        "token": {
            "port": 9800,
            "desc": "headroom-token (full compression + CCR)",
            "flags": [],
        },
    }

    if mode not in configs:
        print(f"Usage: {sys.argv[0]} [cache|token]")
        print(f"  cache — response caching proxy on :9799")
        print(f"  token — full compression proxy on :9800")
        sys.exit(1)

    cfg = configs[mode]

    # API key
    api_key = (
        os.environ.get("HEADROOM_DEEPSEEK_KEY")
        or os.environ.get("DEEPSEEK_API_KEY")
        or os.environ.get("APHRODITE_API_KEY", "")
    )
    if not api_key:
        print("ERROR: Set HEADROOM_DEEPSEEK_KEY, DEEPSEEK_API_KEY, or APHRODITE_API_KEY", file=sys.stderr)
        sys.exit(1)

    print(f"=== {cfg['desc']} ===")
    print(f"  Port:     {cfg['port']}")
    print(f"  Upstream: https://api.deepseek.com")
    print(f"  Model:    deepseek-v4-pro")
    print()

    cmd = [
        sys.executable, "-m", "headroom", "proxy",
        "--port", str(cfg["port"]),
        "--host", "127.0.0.1",
        "--mode", "token",
        "--workers", "2",
        "--no-subscription-tracking",
        *cfg["flags"],
    ]

    env = os.environ.copy()
    env["HEADROOM_DEEPSEEK_KEY"] = api_key
    env["PYTHONPATH"] = HEADROOM_VENDOR
    env["HEADROOM_NO_METRICS"] = "1"

    proc = subprocess.Popen(cmd, cwd=HEADROOM_VENDOR, env=env)
    print(f"Proxy PID: {proc.pid}")
    print(f"Health check: curl http://127.0.0.1:{cfg['port']}/health")
    print()

    try:
        proc.wait()
    except KeyboardInterrupt:
        print("\nShutting down...")
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    main()
