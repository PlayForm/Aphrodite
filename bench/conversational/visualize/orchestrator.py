"""Main visualization orchestrator (visualize_run + file listing)."""

from __future__ import annotations

from pathlib import Path

from .compression_efficiency import render_compression_efficiency
from .radar_chart import render_radar_chart
from .results import load_run_results
from .summary_dashboard import render_summary_dashboard
from .timeline import render_timeline
from .token_comparison import render_token_comparison

def visualize_run(results_dir: Path) -> Path:
    """Generate all visualizations for a benchmark run.

    Args:
        results_dir: Path to the run's results directory (contains manifest.json)

    Returns:
        Path to the generated viz directory.
    """
    results_dir = Path(results_dir)
    if not results_dir.exists():
        raise FileNotFoundError(f"Results directory not found: {results_dir}")

    manifest = load_run_results(results_dir)
    if not manifest.get("results"):
        print("No results to visualize.")
        return results_dir

    viz_dir = results_dir / "visualizations"
    viz_dir.mkdir(exist_ok=True)

    print(f"\n[visualize] Generating charts for run {manifest.get('run_id', 'unknown')}...")

    # Generate all charts
    render_token_comparison(manifest, viz_dir / "token_comparison.png")
    render_timeline(manifest, viz_dir / "token_timeline.png")
    render_compression_efficiency(manifest, viz_dir / "compression_efficiency.png")
    render_radar_chart(manifest, viz_dir / "radar_chart.png")
    render_summary_dashboard(manifest, viz_dir / "summary_dashboard.png")

    print(f"\n[visualize] All charts saved to {viz_dir}/")
    _print_file_list(viz_dir)

    return viz_dir

def _print_file_list(directory: Path):
    """List generated files with sizes."""
    for f in sorted(directory.rglob("*")):
        if f.is_file():
            size = f.stat().st_size
            if size > 1024:
                size_str = f"{size / 1024:.1f} KB"
            else:
                size_str = f"{size} B"
            print(f"    {f.relative_to(directory)} ({size_str})")
