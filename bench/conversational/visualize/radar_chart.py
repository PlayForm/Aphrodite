"""Chart: context-shape comparison radar view."""

from __future__ import annotations

import matplotlib

matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import math

from .constants import COLORS, SCENARIO_LABELS
from .context_shape import CONTEXT_BLOCKS, generate_context_shape


def render_radar_chart(manifest: dict, output_path: Path):
    """Radar chart comparing context shape dimensions across scenarios."""
    scenarios = sorted(set(r["scenario"] for r in manifest.get("results", [])))
    if len(scenarios) < 2:
        return

    dimensions = [b[0] for b in CONTEXT_BLOCKS]
    n_dims = len(dimensions)
    angles = [n / n_dims * 2 * math.pi for n in range(n_dims)]
    angles += angles[:1]  # Close the polygon

    fig, ax = plt.subplots(figsize=(10, 10), subplot_kw=dict(polar=True))
    fig.suptitle("Context Shape Comparison - Radar View", fontsize=14, fontweight="bold")

    for scenario in scenarios:
        shape = generate_context_shape(scenario)
        values = [b["tokens"] for b in shape["blocks"]]
        # Normalize to 0-1 for radar
        max_val = max(values) if max(values) > 0 else 1
        values = [v / max_val for v in values]
        values += values[:1]

        ax.fill(angles, values, alpha=0.15, color=COLORS.get(scenario, "#888"))
        ax.plot(
            angles,
            values,
            "o-",
            linewidth=2,
            label=SCENARIO_LABELS.get(scenario, scenario),
            color=COLORS.get(scenario, "#888"),
        )

    ax.set_xticks(angles[:-1])
    ax.set_xticklabels(dimensions, fontsize=9)
    ax.set_yticklabels([])
    ax.legend(loc="upper right", bbox_to_anchor=(1.3, 1.1), fontsize=9)
    ax.set_title("Token allocation per context block\n(normalized per scenario)", pad=20)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ radar_chart.png saved")
