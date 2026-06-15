#!/usr/bin/env python3
"""
Headroom Proxy Dual-Mode Launcher - starts both cache + token proxies.

DeepSeek v4-pro: 1.6T params, 49B active, 1M context, 384K max output.

    cache proxy  :8787   - freeze prior turns for prefix-cache hit rate
    token proxy  :8788   - compress/rewrite prior turns for max savings

Usage:
    python3 scripts/proxy-dual.py                    # start both in background
    python3 scripts/proxy-dual.py --foreground        # foreground (stderr only)
    python3 scripts/proxy-dual.py --stop              # kill both
    python3 scripts/proxy-dual.py --status            # check both

Later: shim into Hermes for side-by-side comparison testing.
"""

import argparse
import json
import os
import signal
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV = REPO / ".venv"
HEADROOM = VENV / "bin" / "headroom"
DEEPSEEK_URL = "https://api.deepseek.com"

# DeepSeek v4-pro specs
MODEL = "deepseek-v4-pro"
MAX_CONTEXT = 1_000_000
MAX_OUTPUT = 384_000

SERVICES = {
    "cache": {"port": 8787, "mode": "cache",
              "desc": "Freeze prior turns - maximise prefix-cache hit rate"},
    "token": {"port": 8788, "mode": "token",
              "desc": "Compress/rewrite prior turns - maximise token savings"},
}


def _load_key() -> str:
    env_file = REPO / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("HEADROOM_DEEPSEEK_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.getenv("HEADROOM_DEEPSEEK_KEY", "")


def _health(port: int) -> bool:
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=3) as r:
            return json.loads(r.read()).get("ready", False)
    except Exception:
        return False


def _stats(port: int) -> dict:
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/stats", timeout=3) as r:
            return json.loads(r.read())
    except Exception:
        return {}


def start(key: str, foreground: bool = False) -> bool:
    if not HEADROOM.exists():
        print(f"ERROR: headroom not found at {HEADROOM}", file=sys.stderr)
        return False

    env = {
        **os.environ,
        "OPENAI_BASE_URL": DEEPSEEK_URL,
        "OPENAI_API_KEY": key,
        "HEADROOM_CODE_AWARE_ENABLED": "true",
    }

    procs = {}
    for name, cfg in SERVICES.items():
        # Kill any existing on this port
        try:
            subprocess.run(["pkill", "-f", f"headroom proxy.*{cfg['port']}"],
                           capture_output=True, timeout=5)
            time.sleep(0.3)
        except Exception:
            pass

    if foreground:
        print(f"╔══════════════════════════════════════════════════╗")
        print(f"║  HermesCompress Dual Proxy - {MODEL}          ║")
        print(f"║  Context: 1M  |  Output: 384K                    ║")
        print(f"╚══════════════════════════════════════════════════╝")
        print()

    for name, cfg in SERVICES.items():
        cmd = [
            str(HEADROOM), "proxy",
            "--port", str(cfg["port"]),
            "--mode", cfg["mode"],
            "--backend", "openai",
            "--openai-api-url", DEEPSEEK_URL,
            "--code-aware",
            "--no-rate-limit",
        ]

        if foreground:
            print(f"[{name}] Starting :{cfg['port']} - {cfg['desc']}")
            proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            procs[name] = proc
        else:
            proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            procs[name] = proc

    # Wait for health
    unhealthy = set(SERVICES.keys())
    for attempt in range(20):
        time.sleep(0.5)
        for name in list(unhealthy):
            if _health(SERVICES[name]["port"]):
                unhealthy.discard(name)
                if foreground:
                    port = SERVICES[name]["port"]
                    print(f"   ✓ Healthy - curl http://127.0.0.1:{port}/stats")
        if not unhealthy:
            break

    if unhealthy:
        for name in unhealthy:
            print(f"ERROR: [{name}] failed to start", file=sys.stderr)
            procs[name].terminate()
        return False

    if foreground:
        print(f"\nBoth proxies running. Ctrl+C to stop.\n")
        try:
            for name, proc in procs.items():
                proc.wait()
        except KeyboardInterrupt:
            print("\nShutting down...")
            for proc in procs.values():
                proc.terminate()
    else:
        ports = ", ".join(f":{cfg['port']}" for cfg in SERVICES.values())
        print(f"✓ Dual proxies started ({ports}) - {MODEL} (1M ctx / 384K out)")
        print(f"  Cache:  curl http://127.0.0.1:8787/stats  (prefix-freeze)")
        print(f"  Token:  curl http://127.0.0.1:8788/stats  (compress)")
        print(f"  Stop:   python3 scripts/proxy-dual.py --stop")

    return True


def stop_all():
    for cfg in SERVICES.values():
        try:
            subprocess.run(["pkill", "-f", f"headroom proxy.*{cfg['port']}"],
                           capture_output=True, timeout=5)
        except Exception:
            pass
    print("All proxies stopped.")


def show_status():
    print(f"╔══════════════════════════════════════╗")
    print(f"║  HermesCompress Dual Proxy Status    ║")
    print(f"║  Model: {MODEL}  |  1M ctx / 384K out ║")
    print(f"╚══════════════════════════════════════╝")
    print()
    for name, cfg in SERVICES.items():
        port = cfg["port"]
        alive = "✓ HEALTHY" if _health(port) else "✗ DOWN"
        print(f"  [{name}] :{port} - {alive}")
        s = _stats(port)
        if s:
            summary = s.get("summary", {})
            comp = summary.get("compression", {})
            unc = summary.get("uncompressed_requests", {})
            print(f"         mode={summary.get('mode')}, "
                  f"requests={summary.get('api_requests')}, "
                  f"compressed={comp.get('requests_compressed')}, "
                  f"avg={comp.get('avg_compression_pct')}%, "
                  f"frozen={unc.get('prefix_frozen', '?')}")
    print()


if __name__ == "__main__":
    p = argparse.ArgumentParser(
        description="HermesCompress Dual Proxy - cache + token mode",
        epilog=f"Model: {MODEL} (1.6T/49B active, 1M ctx, 384K output)"
    )
    p.add_argument("--foreground", action="store_true", help="Stay in foreground")
    p.add_argument("--stop", action="store_true", help="Kill both proxies")
    p.add_argument("--status", action="store_true", help="Check both proxies")
    args = p.parse_args()

    if args.stop:
        stop_all()
    elif args.status:
        show_status()
    else:
        key = _load_key()
        if not key:
            print("ERROR: HEADROOM_DEEPSEEK_KEY not set in .env", file=sys.stderr)
            sys.exit(1)
        start(key, foreground=args.foreground)
