#!/usr/bin/env python3
"""
HermesCompress Test Suite — keyed, silent, sequential benchmarks.
Uses Hermes CLI (-z) to run identical prompts under different configs.

Each run produces a keyed JSON artifact for inspection.

Usage:
  python3 tests/bench.py              # Run all configs silently
  python3 tests/bench.py --verbose    # Stream output live
  python3 tests/bench.py --key max    # Run only the 'max' config
  python3 tests/bench.py --list       # List available config keys

Keys:
  max        protect_recent=1, target_ratio=0.10, all features on
  off        compression disabled (baseline)
  moderate   protect_recent=5, target_ratio=null, features off
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERMES = "hermes"
CWD = Path(__file__).resolve().parent.parent
RESULTS_DIR = CWD / ".hermes" / "tests"
RESULTS_DIR.mkdir(parents=True, exist_ok=True)

# ── Benchmark prompt ──────────────────────────────────────────────
PROMPT = (
    "Search hermes_compress/ for 'def '. "
    "Run 'wc -l hermes_compress/*.py'. "
    "Read hermes_compress/_strategies.py. "
    "Run headroom_stats after each step. "
    "Output: | step | calls | saved | by_tool delta |"
)

# ── Config keys ───────────────────────────────────────────────────
CONFIGS = {
    "max": {
        "compression.headroom.enabled": "true",
        "compression.headroom.protect_recent": "1",
        "compression.headroom.target_ratio": "0.10",
        "compression.headroom.precompress_tools": "true",
        "compression.headroom.aggressive_kompress": "true",
        "compression.headroom.deduplicate_results": "true",
        "compression.headroom.min_tokens_to_compress": "150",
    },
    "off": {
        "compression.headroom.enabled": "false",
    },
    "moderate": {
        "compression.headroom.enabled": "true",
        "compression.headroom.protect_recent": "5",
        "compression.headroom.target_ratio": "null",
        "compression.headroom.precompress_tools": "false",
        "compression.headroom.aggressive_kompress": "false",
        "compression.headroom.deduplicate_results": "false",
        "compression.headroom.min_tokens_to_compress": "250",
    },
}


def set_config(cfg: dict) -> None:
    """Apply config via hermes config set."""
    for key, value in cfg.items():
        subprocess.run(
            [HERMES, "config", "set", key, value],
            capture_output=True,
        )


def run_bench(key: str, cfg: dict, verbose: bool) -> dict:
    """Run one benchmark key. Returns result dict."""
    ts = time.strftime("%Y%m%d_%H%M%S")
    result_file = RESULTS_DIR / f"{key}_{ts}.json"
    log_file = RESULTS_DIR / f"{key}_{ts}.log"

    # Apply config
    set_config(cfg)

    # Run Hermes with the prompt
    cmd = [HERMES, "chat", "-z", PROMPT]

    start = time.time()
    proc = subprocess.run(
        cmd,
        capture_output=not verbose,
        text=True,
        cwd=str(CWD),
        env={**os.environ, "HERMES_COMPRESS_DEV": "1"},
    )
    elapsed = time.time() - start

    # Collect artifacts
    result = {
        "key": key,
        "timestamp": ts,
        "config": cfg,
        "exit_code": proc.returncode,
        "duration_s": round(elapsed, 2),
        "stdout_lines": len(proc.stdout.splitlines()) if proc.stdout else 0,
        "stderr_lines": len(proc.stderr.splitlines()) if proc.stderr else 0,
    }

    # Save log for inspection
    log_file.write_text(proc.stdout + "\n--- STDERR ---\n" + proc.stderr)

    # Try to extract headroom_stats from output
    if proc.stdout:
        result["output_preview"] = proc.stdout[:500]
        # Find token savings if present
        for line in proc.stdout.splitlines():
            if "tokens_saved" in line or "total_tokens_saved" in line:
                result["has_stats"] = True
                break

    result_file.write_text(json.dumps(result, indent=2))
    return result


def main():
    verbose = "--verbose" in sys.argv
    key_filter = None

    for arg in sys.argv[1:]:
        if arg.startswith("--key="):
            key_filter = arg.split("=", 1)[1]
        elif arg.startswith("--key"):
            idx = sys.argv.index(arg)
            if idx + 1 < len(sys.argv):
                key_filter = sys.argv[idx + 1]

    if "--list" in sys.argv:
        print("Available keys:", ", ".join(CONFIGS))
        return

    keys = [key_filter] if key_filter else list(CONFIGS)
    results = {}

    for key in keys:
        cfg = CONFIGS.get(key)
        if not cfg:
            print(f"Unknown key: {key}")
            continue

        if verbose:
            print(f"\n=== {key} ({time.strftime('%H:%M:%S')}) ===")
        else:
            print(f"  {key}...", end=" ", flush=True)

        try:
            result = run_bench(key, cfg, verbose)
            results[key] = result
            if not verbose:
                print(f"done ({result['duration_s']:.1f}s, exit={result['exit_code']})")
        except Exception as e:
            results[key] = {"key": key, "error": str(e)}
            if not verbose:
                print(f"FAIL: {e}")

    # Summary
    if not verbose and len(results) > 1:
        print("\n--- SUMMARY ---")
        print(f"{'Key':<12} {'Exit':>5} {'Time':>6}s  Stats")
        print("-" * 40)
        for key, r in results.items():
            stats = "✓ stats" if r.get("has_stats") else "—"
            print(f"{key:<12} {r.get('exit_code',-1):>5} {r.get('duration_s',0):>6.1f}s  {stats}")

    print(f"\nResults: {RESULTS_DIR}")


if __name__ == "__main__":
    main()
