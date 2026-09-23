"""Output-path constants for the conversational benchmark.

All benchmark output (turn JSON, manifests, proxy stats, charts) resolves
under RESULTS_DIR - bench/conversational/results/ - so the bench never
writes outside its results tree.
"""

from __future__ import annotations

from pathlib import Path

RESULTS_DIR = Path(__file__).resolve().parent.parent / "results"
