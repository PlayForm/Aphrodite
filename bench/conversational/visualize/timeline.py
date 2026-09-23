"""Chart: per-turn token usage timelines per scenario."""

from __future__ import annotations

import matplotlib

matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np

from .constants import COLORS, SCENARIO_LABELS


def render_timeline(manifest: dict, output_path: Path):
    """Per-turn token usage timeline for each scenario."""
    results = manifest.get("results", [])
    if not results:
        return

    conversations = sorted(set(r["conversation"] for r in results))
    scenarios = sorted(set(r["scenario"] for r in results))
    n_conv = len(conversations)
    n_scen = len(scenarios)

    fig, axes = plt.subplots(n_conv, 1, figsize=(16, 4 * n_conv), squeeze=False)
    fig.suptitle("Per-Turn Token Usage Timeline", fontsize=16, fontweight="bold")

    for ci, conv_name in enumerate(conversations):
        ax = axes[ci][0]

        for scenario in scenarios:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv_name
            ]
            if not matches or "detail" not in matches[0]:
                continue

            detail = matches[0]["detail"]
            turns = detail.get("turns", [])
            if not turns:
                continue

            indices = [t["index"] for t in turns]
            prompt = [t.get("prompt_tokens", 0) for t in turns]
            completion = [t.get("completion_tokens", 0) for t in turns]
            total = [t.get("total_tokens", 0) for t in turns]

            ax.plot(
                indices,
                prompt,
                "o-",
                markersize=4,
                linewidth=1.5,
                color=COLORS.get(scenario, "#888"),
                alpha=0.5,
                label=f"{SCENARIO_LABELS.get(scenario, scenario)} prompt",
            )
            ax.plot(
                indices,
                total,
                "s-",
                markersize=5,
                linewidth=2,
                color=COLORS.get(scenario, "#888"),
                label=f"{SCENARIO_LABELS.get(scenario, scenario)} total",
            )

        ax.set_title(f"{conv_name}")
        ax.set_xlabel("Turn")
        ax.set_ylabel("Tokens")
        ax.legend(fontsize=7, loc="upper left")
        ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f"{x:,.0f}"))
        ax.grid(True, alpha=0.3)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ token_timeline.png saved")
