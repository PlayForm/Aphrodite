"""Chart: prompt-token savings vs baseline per scenario."""

from __future__ import annotations

import matplotlib

matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import numpy as np

from .constants import COLORS, SCENARIO_LABELS

def render_compression_efficiency(manifest: dict, output_path: Path):
    """Compression efficiency: prompt tokens saved vs baseline."""
    results = manifest.get("results", [])
    if not results:
        return

    conversations = sorted(set(r["conversation"] for r in results))
    scenarios = [s for s in sorted(set(r["scenario"] for r in results)) if s != "baseline"]

    if not scenarios:
        return

    fig, ax = plt.subplots(figsize=(12, 6))
    fig.suptitle(
        "Compression Efficiency: Prompt Token Savings vs Baseline", fontsize=14, fontweight="bold"
    )

    x = np.arange(len(conversations))
    bar_width = 0.25

    # Get baseline prompt tokens per conversation
    baseline_prompts = {}
    for conv in conversations:
        matches = [r for r in results if r["scenario"] == "baseline" and r["conversation"] == conv]
        baseline_prompts[conv] = matches[0].get("total_prompt_tokens", 1) if matches else 1

    for i, scenario in enumerate(scenarios):
        savings_pct = []
        for conv in conversations:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv
            ]
            if matches:
                prompt = matches[0].get("total_prompt_tokens", 0)
                bl = baseline_prompts.get(conv, 1)
                pct = (1 - prompt / bl) * 100 if bl > 0 else 0
                savings_pct.append(max(0, pct))
            else:
                savings_pct.append(0)

        bars = ax.bar(
            x + i * bar_width,
            savings_pct,
            bar_width,
            label=SCENARIO_LABELS.get(scenario, scenario),
            color=COLORS.get(scenario, "#888"),
        )
        for bar in bars:
            height = bar.get_height()
            if abs(height) > 0.5:
                ax.annotate(
                    f"{height:.1f}%",
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha="center",
                    va="bottom",
                    fontsize=9,
                )

    ax.set_xticks(x + bar_width)
    ax.set_xticklabels(conversations, rotation=15, ha="right")
    ax.set_ylabel("Prompt Token Savings (%)")
    ax.axhline(y=0, color="black", linewidth=0.5)
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3, axis="y")

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ compression_efficiency.png saved")
