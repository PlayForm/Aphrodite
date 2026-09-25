#!/usr/bin/env python3
"""Deterministic probe: what does the Hermes TUI's `complete.path` return for a word?

Diagnostics step for Aphrodite-enabled sessions (dev-aphrodite profile):
spawns a throwaway `tui_gateway.entry` child exactly like the Ink TUI does
(cwd = HERMES_CWD = the launch directory) and asks `complete.path` for each word.
The live gateway is untouched. Run it from the repo root to probe completion
against Aphrodite monorepo paths (crates/, plugins/, vendor/).

Usage: probe_complete_path.py [hermes_root] [launch_dir] [word...]
Defaults: hermes_root = ~/.hermes/hermes-agent, launch_dir = $PWD, words = ./
"""

import json
import os
import subprocess
import sys
import time

ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/.hermes/hermes-agent")
WORKDIR = sys.argv[2] if len(sys.argv) > 2 else os.getcwd()
WORDS = sys.argv[3:] or ["./"]

env = dict(os.environ)
env.pop("TERMINAL_CWD", None)
env["PYTHONPATH"] = ROOT
env["HERMES_PYTHON_SRC_ROOT"] = ROOT
env["HERMES_CWD"] = WORKDIR
env["HERMES_HOME"] = env.get("HERMES_HOME") or os.path.expanduser("~/.hermes")

# Git-install layout has no venv/: the real TUI gateways run under HERMES_PYTHON
# (e.g. ~/.hermes/tools/python-<ver>/bin/python3). Prefer it, fall back to venv.
_PY = os.environ.get("HERMES_PYTHON") or ""
if not _PY or not os.path.exists(_PY):
    _PY = f"{ROOT}/venv/bin/python3"
proc = subprocess.Popen(
    [_PY, "-m", "tui_gateway.entry"],
    cwd=WORKDIR,
    env=env,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
time.sleep(4)  # boot


def rpc(method, params, rid):
    frame = json.dumps({"jsonrpc": "2.0", "id": rid, "method": method, "params": params}) + "\n"
    proc.stdin.write(frame.encode())
    proc.stdin.flush()


try:
    for i, w in enumerate(WORDS):
        rpc("complete.path", {"word": w}, str(i + 1))
    deadline = time.time() + 15
    seen = {}
    ids = {str(i + 1) for i in range(len(WORDS))}
    while time.time() < deadline:
        line = proc.stdout.readline()
        if not line:
            break
        try:
            obj = json.loads(line)
        except Exception:
            continue
        if obj.get("id") in ids and obj.get("id") not in seen:
            seen[obj["id"]] = [
                it.get("display") for it in (obj.get("result") or {}).get("items", [])
            ]
    for i, w in enumerate(WORDS):
        print(f"WORD {w!r}: {json.dumps(seen.get(str(i + 1), []))}")
finally:
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except Exception:
        proc.kill()
