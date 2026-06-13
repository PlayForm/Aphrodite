#!/usr/bin/env python3
"""
Headroom Proxy Starter — launches with compression enabled.

IMPORTANT: headroom proxy compresses Anthropic Messages API and OpenAI
Responses API. It does NOT compress OpenAI Chat Completions API.
For Chat Completions (what Hermes uses), use the inline library:
  from hermes_compress import Compress
  c = Compress(model="deepseek-v4-pro")
  result = c.compress(messages)

Usage:
  python3 scripts/proxy-start.py              # start in background
  python3 scripts/proxy-start.py --foreground  # stay in foreground
  python3 scripts/proxy-start.py --token       # token mode (more aggressive)
  python3 scripts/proxy-start.py --port 9090   # custom port
"""

import argparse, json, os, signal, subprocess, sys, time, urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV = REPO / ".venv"
HEADROOM = VENV / "bin" / "headroom"
DEEPSEEK_URL = "https://api.deepseek.com"
DEFAULT_PORT = 8787


def _load_env():
    env_file = REPO / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("#") or "=" not in line:
                continue
            k, v = line.split("=", 1)
            if k.strip() == "HEADROOM_DEEPSEEK_KEY":
                return v.strip().strip('"').strip("'")
    return os.getenv("HEADROOM_DEEPSEEK_KEY", "")


def _health(port):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=3) as r:
            return json.loads(r.read()).get("ready", False)
    except Exception:
        return False


def start(port, mode, foreground):
    key = _load_env()
    if not key:
        print("ERROR: HEADROOM_DEEPSEEK_KEY not set in .env", file=sys.stderr)
        sys.exit(1)

    # Kill existing
    try:
        subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True, timeout=5)
        time.sleep(0.5)
    except Exception:
        pass

    if not HEADROOM.exists():
        print(f"ERROR: headroom not at {HEADROOM}", file=sys.stderr)
        sys.exit(1)

    env = {
        **os.environ,
        "OPENAI_BASE_URL": DEEPSEEK_URL,
        "OPENAI_API_KEY": key,
        "HEADROOM_CODE_AWARE_ENABLED": "true",
    }

    cmd = [
        str(HEADROOM), "proxy",
        "--port", str(port),
        "--mode", mode,
        "--backend", "openai",
        "--openai-api-url", DEEPSEEK_URL,
        "--code-aware",
    ]

    if foreground:
        print(f"Starting proxy :{port} ({mode} mode, code-aware)")
        print(f"Config env: OPENAI_BASE_URL={DEEPSEEK_URL}")
        print(f"Health:     curl http://127.0.0.1:{port}/health")
        print(f"Stats:      curl http://127.0.0.1:{port}/stats")
        print(f"Compression: Anthropic Messages API / OpenAI Responses API only")
        print(f"             Chat Completions API passes through uncompressed")
        print(f"             Use Python library for Chat Completions compression")
        subprocess.run(cmd, env=env)
    else:
        proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(15):
            time.sleep(1)
            if _health(port):
                print(f"✓ Proxy started :{port} ({mode} mode, code-aware)")
                print(f"  PID: {proc.pid}")
                print(f"  Health: curl http://127.0.0.1:{port}/health")
                print(f"  Stats:  curl http://127.0.0.1:{port}/stats")
                print(f"  Stop:   kill {proc.pid}")
                return
        proc.terminate()
        print("ERROR: Proxy failed to start", file=sys.stderr)
        sys.exit(1)


def stop(port):
    try:
        subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True, timeout=5)
        print(f"Proxy :{port} stopped.")
    except Exception:
        pass


if __name__ == "__main__":
    p = argparse.ArgumentParser(description="Start headroom proxy for compression")
    p.add_argument("--port", type=int, default=DEFAULT_PORT)
    p.add_argument("--mode", choices=["cache", "token"], default="cache")
    p.add_argument("--foreground", action="store_true", help="Stay in foreground")
    p.add_argument("--stop", action="store_true", help="Kill running proxy")
    args = p.parse_args()

    if args.stop:
        stop(args.port)
    elif _health(args.port):
        print(f"Proxy already running on :{args.port}")
    else:
        start(args.port, args.mode, args.foreground)
