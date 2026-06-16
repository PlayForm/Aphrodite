#!/usr/bin/env python3
"""
Launch headroom as a standalone caching proxy for Hermes Agent.

Modes:
    cache       - response caching only (:9799), saves API costs
    token       - full compression + CCR (:9800)
    benchmark   - cache mode with max workers to saturate cache

Usage:
    python3 scripts/run-headroom-proxy.py cache       # 1 worker, full traceability
    python3 scripts/run-headroom-proxy.py benchmark   # 8 workers, cache saturation

Auth:
    source ~/.privateenvsh first (APHRODITE_API_KEY, HEADROOM_DEEPSEEK_KEY, OPENAI_API_KEY)

Worker guidance:
    --workers 1   → full traceability, single worker for debugging savings
    --workers 8   → cache saturation, benchmark mode (note CCR fragmentation warning)
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
            "workers": 1,
            "desc": "headroom-cache - 1 worker, full traceability (:9799)",
            "flags": ["--no-optimize", "--no-ccr-marker", "--no-telemetry"],
        },
        "benchmark": {
            "port": 9799,
            "workers": 8,
            "desc": "headroom-benchmark - 8 workers, cache saturation (:9799)",
            "flags": ["--no-optimize", "--no-ccr-marker", "--no-telemetry"],
        },
        "token": {
            "port": 9800,
            "workers": 1,
            "desc": "headroom-token - full compression + CCR (:9800)",
            "flags": ["--no-telemetry"],
        },
    }

    if mode not in configs:
        print(f"Usage: {sys.argv[0]} [cache|benchmark|token]")
        for m, c in configs.items():
            print(f"  {m:<11} - {c['desc']}")
        sys.exit(1)

    cfg = configs[mode]
    api_key = get_api_key()
    if not api_key:
        print("ERROR: source ~/.privateenvsh first", file=sys.stderr)
        sys.exit(1)

    print(f"=== {cfg['desc']} ===")
    print(f"  Upstream: {DEEPSEEK_URL}")
    print(f"  Workers:  {cfg['workers']}")
    if cfg["workers"] > 1:
        print()
        print("[WARN] CCR fragmentation possible with >1 worker. Use 1 worker for deterministic CCR.")
    print()

    cmd = [
        HEADROOM_BIN, "proxy",
        "--port", str(cfg["port"]),
        "--host", "127.0.0.1",
        "--openai-api-url", DEEPSEEK_URL,
        "--mode", "token",
        "--workers", str(cfg["workers"]),
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
