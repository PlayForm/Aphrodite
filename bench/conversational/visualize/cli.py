"""CLI entry for benchmark visualization."""

from __future__ import annotations

import argparse
from pathlib import Path

from .orchestrator import visualize_run


def main():
    parser = argparse.ArgumentParser(description="Aphrodite Benchmark Visualization")
    parser.add_argument("results_dir", help="Path to benchmark results directory")
    args = parser.parse_args()

    results_dir = Path(args.results_dir)
    visualize_run(results_dir)


if __name__ == "__main__":
    main()
