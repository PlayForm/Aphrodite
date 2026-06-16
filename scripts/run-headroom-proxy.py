#!/usr/bin/env python3
"""
Launch headroom as a standalone caching proxy for Hermes Agent.

Two modes:
    cache — response caching only (:9799), minimizes API costs
    token — full compression + CCR (:9800)

Usage:
    python3 scripts/run-headroom-proxy.py cache
    python3 scripts/run-headroom-proxy.py token

Auth:
    Uses APHRODITE_API_KEY from environment (same key Hermes uses)
    Falls back to DEEPSEEK_API_KEY, then HEADROOM_DEEPSEEK_KEY

Optimized for Hermes:
    --openai-api-url → DeepSeek OpenAI-compatible endpoint
    --no-subscription-tracking → removes subscription polling overhead
    --no-optimize (cache mode) → pure caching, zero compression overhead
    --workers 2 → balanced for local dev
"""
import os
import sys
import subprocess

HEADROOM_BIN = "headroom"
DEEPSEEK_URL = "https://api.deepseek.com/v1"


def get_api_key():
    return (
        os.environ.get("APHRODITE_API_KEY")
        or os.environ.get("DEEPSEEK_API_KEY")
        or os.environ.get("HEADROOM_DEEPSEEK_KEY", "")
    )


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""

    configs = {
        "cache": {
            "port": 9799,
            "desc": "headroom-cache — response caching only (:9799)",
            "flags": ["--no-optimize", "--no-ccr-marker"],
        },
        "token": {
            "port": 9800,
            "desc": "headroom-token — full compression + CCR (:9800)",
            "flags": [],
        },
    }

    if mode not in configs:
        print(f"Usage: {sys.argv[0]} [cache|token]")
        for m, c in configs.items():
            print(f"  {m:<7} — {c['desc']}")
        sys.exit(1)

    cfg = configs[mode]
    api_key = get_api_key()
    if not api_key:
        print("ERROR: Set APHRODITE_API_KEY, DEEPSEEK_API_KEY, or HEADROOM_DEEPSEEK_KEY", file=sys.stderr)
        sys.exit(1)

    print(f"=== {cfg['desc']} ===")
    print(f"  Upstream: {DEEPSEEK_URL}")
    print()

    cmd = [
        HEADROOM_BIN, "proxy",
        "--port", str(cfg["port"]),
        "--host", "127.0.0.1",
        "--openai-api-url", DEEPSEEK_URL,
        "--mode", "token",
        "--workers", "2",
        "--no-subscription-tracking",
        *cfg["flags"],
    ]

    env = os.environ.copy()
    env["APHRODITE_API_KEY"] = api_key

    proc = subprocess.Popen(cmd, env=env)
    print(f"PID: {proc.pid} | Health: curl http://127.0.0.1:{cfg['port']}/health")
    try:
        proc.wait()
    except KeyboardInterrupt:
        print("\nShutting down...")
        proc.terminate()
        proc.wait()


if __name__ == "__main__":
    main()
