"""Chart: total tokens per scenario per conversation (bar charts)."""

from __future__ import annotations

import matplotlib

matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np

from .constants import COLORS, SCENARIO_LABELS


def render_token_comparison(manifest: dict, output_path: Path):
    """Bar chart: total tokens per scenario per conversation."""
    results = manifest.get("results", [])
    if not results:
        return

    conversations = sorted(set(r["conversation"] for r in results))
    scenarios = sorted(set(r["scenario"] for r in results))
    n_conv = len(conversations)
    n_scen = len(scenarios)

    fig, axes = plt.subplots(1, 3, figsize=(18, 8))
    fig.suptitle(
        "Aphrodite Conversational Benchmark - Token Analysis", fontsize=16, fontweight="bold"
    )

    bar_width = 0.2
    x = np.arange(n_conv)

    # Chart 1: Total tokens
    ax = axes[0]
    for i, scenario in enumerate(scenarios):
        values = []
        for conv in conversations:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv
            ]
            val = matches[0].get("total_tokens", 0) if matches else 0
            values.append(val)
        bars = ax.bar(
            x + i * bar_width,
            values,
            bar_width,
            label=SCENARIO_LABELS.get(scenario, scenario),
            color=COLORS.get(scenario, "#888"),
        )
        # Add value labels on bars
        for bar in bars:
            height = bar.get_height()
            if height > 0:
                ax.annotate(
                    f"{height:,}",
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha="center",
                    va="bottom",
                    fontsize=8,
                )

    ax.set_title("Total Tokens per Conversation")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha="right")
    ax.set_ylabel("Tokens")
    ax.legend(fontsize=8, loc="upper left")
    ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f"{x:,.0f}"))

    # Chart 2: Prompt vs Completion breakdown
    ax = axes[1]
    prompt_data = {}
    comp_data = {}
    for scenario in scenarios:
        prompt_data[scenario] = []
        comp_data[scenario] = []
        for conv in conversations:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv
            ]
            if matches:
                prompt_data[scenario].append(matches[0].get("total_prompt_tokens", 0))
                comp_data[scenario].append(matches[0].get("total_completion_tokens", 0))
            else:
                prompt_data[scenario].append(0)
                comp_data[scenario].append(0)

    for i, scenario in enumerate(scenarios):
        bottom = np.zeros(n_conv)
        p = np.array(prompt_data[scenario])
        c = np.array(comp_data[scenario])
        ax.bar(
            x + i * bar_width,
            p,
            bar_width,
            bottom=bottom,
            color=COLORS.get(scenario, "#888"),
            alpha=0.7,
            label=f"{SCENARIO_LABELS.get(scenario, scenario)} (prompt)",
        )
        ax.bar(
            x + i * bar_width,
            c,
            bar_width,
            bottom=p,
            color=COLORS.get(scenario, "#888"),
            alpha=0.4,
            label=f"{SCENARIO_LABELS.get(scenario, scenario)} (completion)",
        )

    ax.set_title("Prompt vs Completion Tokens")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha="right")
    ax.set_ylabel("Tokens")
    ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f"{x:,.0f}"))

    # Chart 3: Token efficiency (relative to baseline)
    ax = axes[2]
    baseline_totals = {}
    for conv in conversations:
        matches = [r for r in results if r["scenario"] == "baseline" and r["conversation"] == conv]
        baseline_totals[conv] = matches[0].get("total_tokens", 1) if matches else 1

    for i, scenario in enumerate(scenarios):
        if scenario == "baseline":
            continue
        values = []
        for conv in conversations:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv
            ]
            total = matches[0].get("total_tokens", 0) if matches else 0
            baseline = baseline_totals.get(conv, 1)
            ratio = (total / baseline * 100) if baseline > 0 else 100
            values.append(ratio)
        bars = ax.bar(
            x + i * bar_width,
            values,
            bar_width,
            label=SCENARIO_LABELS.get(scenario, scenario),
            color=COLORS.get(scenario, "#888"),
        )
        for bar in bars:
            height = bar.get_height()
            ax.annotate(
                f"{height:.0f}%",
                xy=(bar.get_x() + bar.get_width() / 2, height),
                xytext=(0, 3),
                textcoords="offset points",
                ha="center",
                va="bottom",
                fontsize=8,
            )

    ax.set_title("Tokens vs Baseline (%)")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha="right")
    ax.set_ylabel("% of Baseline")
    ax.axhline(y=100, color="red", linestyle="--", alpha=0.5, label="Baseline (100%)")
    ax.legend(fontsize=8, loc="upper left")

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ token_comparison.png saved")
