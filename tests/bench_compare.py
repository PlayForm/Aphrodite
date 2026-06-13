#!/usr/bin/env python3
"""
HermesCompress vs HermesProxy — Comprehensive Comparison Benchmark

Compares three compression approaches with identical prompts:
  1. hook   — hermes-compress plugin (per-tool _transform_tool_result)
  2. proxy-cache — hermes-proxy plugin (headroom --mode cache on :8787)
  3. proxy-token — hermes-proxy plugin (headroom --mode token on :8788)

Shared logging via `hermes_compress` logger for unified monitoring.

Usage:
  python3 tests/bench_compare.py              # Run all three
  python3 tests/bench_compare.py --mode hook  # Single mode
  python3 tests/bench_compare.py --verbose    # Stream output

Output: .hermes/tests/compare_{key}_{ts}.json + .log per run
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERMES = "hermes"
CWD = Path(__file__).resolve().parent.parent
RESULTS_DIR = CWD / ".hermes" / "tests" / "compare"
RESULTS_DIR.mkdir(parents=True, exist_ok=True)

LOG_CHANNEL = "hermes_compress"

PROMPT = (
    "Search hermes_compress/ for 'def '. "
    "Read hermes_compress/_strategies.py. "
    "Read hermes_compress/__init__.py. "
    "Run 'wc -l hermes_compress/*.py'. "
    "Run headroom_stats after each step. "
    "Report: | step | calls | tokens_saved | by_tool | latency_ms |"
)

# ── Config presets ──────────────────────────────────────────────────
HOOK_CONFIG = {
    "compression.headroom.enabled": "true",
    "compression.headroom.integration": "hook",
    "compression.headroom.protect_recent": "1",
    "compression.headroom.target_ratio": "0.10",
    "compression.headroom.min_tokens_to_compress": "150",
}

PROXY_CONFIG = {
    "compression.headroom.enabled": "true",
    "compression.headroom.integration": "proxy",
    "compression.headroom.proxy_port": "8787",
    "compression.headroom.proxy_auto_start": "true",
}


def set_config(cfg: dict) -> None:
    for key, value in cfg.items():
        subprocess.run([HERMES, "config", "set", key, value], capture_output=True)


def start_proxy(mode: str, port: int) -> bool:
    """Start headroom proxy, return True if healthy."""
    hb = subprocess.run(["which", "headroom"], capture_output=True, text=True)
    hb_path = hb.stdout.strip() or "headroom"

    # Kill existing
    subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)
    time.sleep(0.5)

    subprocess.Popen(
        [hb_path, "proxy", "--port", str(port), "--mode", mode],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    time.sleep(1.5)

    try:
        import urllib.request
        url = f"http://127.0.0.1:{port}/health"
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=3) as resp:
            return resp.status == 200
    except Exception:
        return False


def stop_proxy(port: int) -> None:
    subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)


def run_session(mode: str, extra_env: dict = None) -> dict:
    """Run one benchmark session. Returns result dict."""
    ts = time.strftime("%Y%m%d_%H%M%S")
    key = f"{mode}_{ts}"
    log_file = RESULTS_DIR / f"{key}.log"
    json_file = RESULTS_DIR / f"{key}.json"

    env = {**os.environ, **(extra_env or {})}

    start = time.time()
    proc = subprocess.run(
        [HERMES, "chat", "-z", PROMPT],
        capture_output=True,
        text=True,
        cwd=str(CWD),
        env=env,
        timeout=120,
    )
    elapsed = time.time() - start

    result = {
        "key": key,
        "mode": mode,
        "timestamp": ts,
        "exit_code": proc.returncode,
        "duration_s": round(elapsed, 2),
        "stdout_len": len(proc.stdout) if proc.stdout else 0,
    }

    # Extract headroom_stats
    saved = 0
    calls = 0
    by_tool = {}
    for line in proc.stdout.splitlines():
        if '"total_tokens_saved"' in line:
            try:
                saved = int(line.split(":")[1].strip().rstrip(","))
            except Exception:
                pass
        if '"calls"' in line:
            try:
                calls = int(line.split(":")[1].strip().rstrip(","))
            except Exception:
                pass

    result["calls"] = calls
    result["tokens_saved"] = saved
    result["avg_per_call"] = round(saved / max(calls, 1))

    # Save artifacts
    log_file.write_text(proc.stdout + "\n--- STDERR ---\n" + proc.stderr)
    json_file.write_text(json.dumps(result, indent=2))

    return result


def main():
    verbose = "--verbose" in sys.argv
    mode_filter = None
    for arg in sys.argv[1:]:
        if arg.startswith("--mode="):
            mode_filter = arg.split("=", 1)[1]

    modes = ["hook", "proxy-cache", "proxy-token"]
    if mode_filter:
        modes = [m for m in modes if m == mode_filter]

    results = {}

    for mode in modes:
        label = f"[{mode}]"
        print(f"\n{'='*60}")
        print(f"  {label} starting ({time.strftime('%H:%M:%S')})")

        try:
            if mode == "hook":
                set_config(HOOK_CONFIG)
                stop_proxy(8787)
                stop_proxy(8788)
                r = run_session("hook", {"HERMES_COMPRESS_DEV": "1"})

            elif mode == "proxy-cache":
                set_config(PROXY_CONFIG)
                if not start_proxy("cache", 8787):
                    print(f"  {label} FAILED: proxy did not start")
                    results[mode] = {"error": "proxy did not start"}
                    continue
                r = run_session("proxy-cache")

            elif mode == "proxy-token":
                set_config(PROXY_CONFIG)
                stop_proxy(8787)
                if not start_proxy("token", 8788):
                    print(f"  {label} FAILED: proxy did not start")
                    results[mode] = {"error": "proxy did not start"}
                    continue
                r = run_session("proxy-token")

            results[mode] = r
            print(f"  {label} done: {r.get('calls', 0)} calls, "
                  f"{r.get('tokens_saved', 0):,} saved, "
                  f"{r.get('duration_s', 0):.1f}s")

        except Exception as e:
            results[mode] = {"error": str(e)}
            print(f"  {label} FAILED: {e}")

    # Cleanup
    stop_proxy(8787)
    stop_proxy(8788)

    # Summary table
    if len(results) > 1:
        print(f"\n{'='*60}")
        print(f"  COMPARISON SUMMARY")
        print(f"{'='*60}")
        print(f"  {'Mode':<15} {'Calls':>6} {'Saved':>10} {'Avg/Call':>10} {'Time':>7}")
        print(f"  {'-'*48}")
        for mode in modes:
            r = results.get(mode, {})
            if "error" in r:
                print(f"  {mode:<15} {'—':>6} {'—':>10} {'—':>10} {'—':>7}")
            else:
                print(f"  {mode:<15} {r.get('calls',0):>6} "
                      f"{r.get('tokens_saved',0):>10,} "
                      f"{r.get('avg_per_call',0):>10,} "
                      f"{r.get('duration_s',0):>6.1f}s")

    print(f"\n  Results: {RESULTS_DIR}")
    print(f"  Logger:  {LOG_CHANNEL}")


if __name__ == "__main__":
    main()
