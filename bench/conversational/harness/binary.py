"""Aphrodite binary resolution (env var, target dirs, cargo build fallback)."""

from __future__ import annotations
import os
import subprocess
import sys
from pathlib import Path

APHRODITE_BINARY = None  # Resolved at runtime


def resolve_aphrodite_binary() -> str:
    """Find the aphrodite binary. Checks: env var, target/release, target/debug, cargo build."""
    # Check env var
    if "APHRODITE_BIN" in os.environ:
        return os.environ["APHRODITE_BIN"]

    # Check target directories relative to the workspace root
    workspace = (
        Path(__file__).resolve().parent.parent.parent.parent
    )  # harness/ -> bench/conversational -> bench -> repo root
    for profile in ["release", "debug"]:
        candidate = workspace / "target" / profile / "aphrodite"
        if candidate.exists():
            return str(candidate)

    # Try cargo build --release
    print("Building aphrodite binary (cargo build --release)...")
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=str(workspace),
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"Build failed:\n{result.stderr}")
        sys.exit(1)

    candidate = workspace / "target" / "release" / "aphrodite"
    if candidate.exists():
        return str(candidate)

    raise FileNotFoundError("Cannot find aphrodite binary. Build it first or set APHRODITE_BIN.")
