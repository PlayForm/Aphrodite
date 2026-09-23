"""Benchmark results reader (manifest + per-conversation summaries)."""

from __future__ import annotations

import json
from pathlib import Path

def load_run_results(results_dir: Path) -> dict:
    """Load all results from a benchmark run directory."""
    manifest_path = results_dir / "manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"No manifest found at {manifest_path}")

    with open(manifest_path) as f:
        manifest = json.load(f)

    # Load per-conversation data
    for r in manifest.get("results", []):
        conv_dir = results_dir / r["scenario"] / r["conversation"]
        summary_path = conv_dir / "summary.json"
        if summary_path.exists():
            with open(summary_path) as f:
                r["detail"] = json.load(f)

        turns_dir = conv_dir / "turns"
        if turns_dir.exists():
            r["turn_files"] = sorted(turns_dir.glob("*.json"))

    return manifest
