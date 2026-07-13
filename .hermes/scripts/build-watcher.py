#!/usr/bin/env python3
"""Monitor pane 17 cargo watch, write build-status.json every 5s."""

import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone

STATE_DIR = os.path.expanduser("~/.hermes")
OUTPUT_FILE = os.path.join(STATE_DIR, "build-status.json")
PANE_ID = int(os.environ.get("APHRODITE_BUILD_WATCHER_PANE_ID", "17"))
POLL_INTERVAL = 5

STATUS_IDLE = "idle"
STATUS_COMPILING = "compiling"
STATUS_RUNNING = "running"
STATUS_ERROR = "error"


def log(msg: str) -> None:
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}", flush=True)


def get_pane_buffer(pane_id: int, lines: int = 8) -> str:
    try:
        result = subprocess.run(
            ["wezterm", "cli", "get-text", "--pane-id", str(pane_id), "--start-line", str(-lines)],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return result.stdout
        else:
            log(f"wezterm cli error (rc={result.returncode}): {result.stderr.strip()}")
            return ""
    except FileNotFoundError:
        log("ERROR: wezterm not found on PATH")
        return ""
    except subprocess.TimeoutExpired:
        log("WARN: wezterm cli timed out")
        return ""
    except Exception as e:
        log(f"WARN: wezterm cli exception: {e}")
        return ""


def parse_buffer(buffer: str) -> dict:
    errors = []
    compiling = False
    running = False
    finished_ok = False

    for line in buffer.splitlines():
        line_stripped = line.strip()

        # Detect errors (but not INFO log lines mentioning "error")
        if re.search(r"\berror\b", line_stripped, re.IGNORECASE):
            # Skip false positives like "listening" or INFO noise
            if not re.search(r"(INFO|listening|address)", line_stripped, re.IGNORECASE):
                errors.append(line_stripped)

        if re.search(r"^\s*Compiling\s", line_stripped):
            compiling = True
        if re.search(r"^\s*Running\s", line_stripped):
            # Only "Running `target/..." lines from cargo, not the "[Running 'cargo ...']" marker
            if re.search(r"`[^`]+`", line_stripped):
                running = True
        if re.search(r"^\s*Finished\s", line_stripped):
            if "error" not in line_stripped.lower():
                finished_ok = True

    # Determine composite status
    if errors:
        status = STATUS_ERROR
    elif compiling and not finished_ok:
        status = STATUS_COMPILING
    elif running:
        status = STATUS_RUNNING
    else:
        status = STATUS_IDLE

    return {
        "status": status,
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "errors": errors[:10],  # cap at 10
        "compiling": compiling,
        "running": running,
        "finished_ok": finished_ok,
    }


def write_status(data: dict) -> None:
    os.makedirs(STATE_DIR, exist_ok=True)
    tmp = OUTPUT_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(data, f, indent=2)
    os.replace(tmp, OUTPUT_FILE)


def main():
    os.makedirs(STATE_DIR, exist_ok=True)

    # Initial write
    write_status(
        {
            "status": STATUS_IDLE,
            "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "errors": [],
            "compiling": False,
            "running": False,
            "finished_ok": False,
        }
    )
    log(f"Starting pane {PANE_ID} monitor → {OUTPUT_FILE} (every {POLL_INTERVAL}s)")

    consecutive_failures = 0

    while True:
        time.sleep(POLL_INTERVAL)

        buffer = get_pane_buffer(PANE_ID, lines=10)
        if not buffer:
            consecutive_failures += 1
            if consecutive_failures > 6:  # 30s of failures -> idle
                write_status(
                    {
                        "status": STATUS_IDLE,
                        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                        "errors": [],
                        "compiling": False,
                        "running": False,
                        "finished_ok": False,
                    }
                )
            continue

        consecutive_failures = 0
        data = parse_buffer(buffer)

        # Only write on status change or every 30s
        try:
            with open(OUTPUT_FILE) as f:
                prev = json.load(f)
            status_changed = prev.get("status") != data["status"]
        except (FileNotFoundError, json.JSONDecodeError):
            status_changed = True

        if status_changed or int(time.time()) % 30 < POLL_INTERVAL:
            write_status(data)
            if data["status"] == STATUS_ERROR:
                log(f"ERROR: {len(data['errors'])} error(s)")
            elif data["status"] == STATUS_COMPILING:
                log("Compiling...")
            elif data["status"] == STATUS_RUNNING:
                log("Running")
            elif status_changed and data["status"] == STATUS_IDLE:
                log("Idle")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        log("Monitor stopped")
        sys.exit(0)
