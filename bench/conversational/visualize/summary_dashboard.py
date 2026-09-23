"""Chart: single-page summary dashboard with key metrics."""

from __future__ import annotations

import matplotlib

matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np

from .constants import COLORS, SCENARIO_LABELS


def render_summary_dashboard(manifest: dict, output_path: Path):
    """Single-page summary dashboard with key metrics."""
    results = manifest.get("results", [])
    if not results:
        return

    fig = plt.figure(figsize=(22, 14))
    fig.suptitle(
        f"Aphrodite Conversational Benchmark - Run {manifest.get('run_id', 'unknown')}",
        fontsize=16,
        fontweight="bold",
    )

    # Use GridSpec: top row 4 cols
    from matplotlib.gridspec import GridSpec

    gs = GridSpec(1, 4, figure=fig, wspace=0.3)

    # ── Top-left: Summary table ──
    ax_table = fig.add_subplot(gs[0, :2])
    ax_table.axis("off")

    table_data = []
    table_cols = ["Scenario", "Conversation", "Turns", "Prompt", "Completion", "Total", "Errors"]
    for r in results:
        table_data.append(
            [
                r["scenario"],
                r["conversation"],
                str(r.get("turns", len(r.get("detail", {}).get("turns", [])))),
                f"{r.get('total_prompt_tokens', 0):,}",
                f"{r.get('total_completion_tokens', 0):,}",
                f"{r.get('total_tokens', 0):,}",
                str(r.get("errors", 0)),
            ]
        )

    if table_data:
        tbl = ax_table.table(
            cellText=table_data,
            colLabels=table_cols,
            cellLoc="center",
            loc="center",
            colWidths=[0.12, 0.14, 0.06, 0.12, 0.12, 0.12, 0.06],
        )
        tbl.auto_set_font_size(False)
        tbl.set_fontsize(8)
        tbl.scale(1.2, 1.4)
        # Color rows by scenario
        for i, row_data in enumerate(table_data):
            scenario = row_data[0]
            color = COLORS.get(scenario, "#fff")
            for j in range(len(table_cols)):
                tbl[(i + 1, j)].set_facecolor(color + "22")  # 13% opacity
        ax_table.set_title("Results Summary", fontweight="bold")

    # ── Top-center: Token totals bar chart ──
    ax_bars = fig.add_subplot(gs[0, 2])
    scenarios = sorted(set(r["scenario"] for r in results))
    conversations = sorted(set(r["conversation"] for r in results))
    x = np.arange(len(conversations))
    bar_width = 0.2

    for i, scenario in enumerate(scenarios):
        values = []
        for conv in conversations:
            matches = [
                r for r in results if r["scenario"] == scenario and r["conversation"] == conv
            ]
            values.append(matches[0].get("total_tokens", 0) if matches else 0)
        ax_bars.bar(
            x + i * bar_width, values, bar_width, label=scenario, color=COLORS.get(scenario, "#888")
        )

    ax_bars.set_title("Total Tokens by Scenario")
    ax_bars.set_xticks(x + bar_width * (len(scenarios) - 1) / 2)
    ax_bars.set_xticklabels(conversations, rotation=15, ha="right", fontsize=8)
    ax_bars.legend(fontsize=7)
    ax_bars.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f"{x:,.0f}"))

    # ── Top-right: Token savings % ──
    ax_savings = fig.add_subplot(gs[0, 3])
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
            bl = baseline_totals.get(conv, 1)
            values.append((1 - total / bl) * 100 if bl > 0 else 0)
        ax_savings.bar(
            x + i * bar_width, values, bar_width, label=scenario, color=COLORS.get(scenario, "#888")
        )

    ax_savings.set_title("Token Savings vs Baseline (%)")
    ax_savings.set_xticks(x + bar_width)
    ax_savings.set_xticklabels(conversations, rotation=15, ha="right", fontsize=8)
    ax_savings.axhline(y=0, color="black", linewidth=0.5)
    ax_savings.legend(fontsize=7)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ summary_dashboard.png saved")
