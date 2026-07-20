"""
Aphrodite Conversational Benchmark Visualization

Reads benchmark results and generates:
  1. S2 context-shape maps (ASCII + PNG via matplotlib)
  2. Token comparison charts across scenarios
  3. Per-turn token usage timelines
  4. Compression efficiency dashboards
  5. CCR event analysis
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Optional

import matplotlib
matplotlib.use("Agg")  # Non-interactive backend for PNG rendering
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np

# Add parent for s2-probe if available
_S2_AVAILABLE = False
try:
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "crates" / "s2-probe" / "src"))
    # We import s2-probe Rust concepts but reproduce them in Python for the visualization
    # since s2-probe is Rust, we implement the S2 shape logic directly in Python
    _S2_AVAILABLE = True
except Exception:
    pass


# ═══════════════════════════════════════════════════════════════════════════════
# S2 Context Shape Generator (Python port of s2-probe concepts)
# ═══════════════════════════════════════════════════════════════════════════════

S2_MAX_LEVEL = 16

# Context block definitions (matching s2-probe's task_profiles)
CONTEXT_BLOCKS = [
    ("system",     120,  3,  "System prompt"),
    ("directives", 420,  5,  "Behavioral directives"),
    ("nudges",      80,  7,  "Per-turn nudges"),
    ("plain",     1000,  8,  "Plain data / tool outputs"),
    ("recall",    1200, 10,  "CCR recall / catalog"),
    ("hint",        60,  6,  "Retrieve hint"),
    ("convo",     1600, 12,  "Conversation history"),
]

# Per-scenario block token adjustments (scenarios skew the context differently)
SCENARIO_BLOCKS = {
    "baseline": {
        # No compression — full tool outputs in plain data
        "plain": 4000,   # Much larger: full tool outputs inline
        "recall": 0,     # No CCR recall needed
    },
    "full": {
        # Full compression — tool outputs compressed, messages offloaded
        "plain": 600,    # Smaller: markers replace tool outputs
        "recall": 1800,  # Larger: CCR catalog + retrieval
    },
    "hermes_proxy": {
        # Cache proxy only — tool outputs compressed, no message offloading
        "plain": 800,    # Compressed tool outputs
        "recall": 1400,  # CCR catalog
    },
    "proxy_api": {
        # Token proxy only — messages offloaded, tool outputs inline
        "plain": 3000,   # Still large (full tool outputs)
        "recall": 1200,  # Offloaded messages need retrieval
    },
}


def generate_context_shape(scenario: str, total_tokens: int = 4500) -> dict:
    """Generate a context-shape descriptor for a scenario.

    Returns a dict with block-level coverage data suitable for rendering.
    Each block is a latitudinal band; its longitudinal span = token share.
    Resolution level maps to visual density.
    """
    adjustments = SCENARIO_BLOCKS.get(scenario, {})
    blocks = []
    for name, tokens, level, desc in CONTEXT_BLOCKS:
        adj_tokens = adjustments.get(name, tokens)
        blocks.append({
            "name": name,
            "description": desc,
            "tokens": adj_tokens,
            "level": level,
            "share": adj_tokens / max(total_tokens, 1),
        })

    # Normalize shares
    total = sum(b["tokens"] for b in blocks)
    for b in blocks:
        b["share"] = b["tokens"] / max(total, 1)

    return {
        "scenario": scenario,
        "total_tokens": total,
        "blocks": blocks,
    }


def render_s2_ascii(shape: dict) -> str:
    """Render an ASCII S2 context-shape map for a scenario."""
    blocks = shape["blocks"]
    n = len(blocks)
    grid_cols = 72
    rows_per_block = 2

    lines = []
    lines.append(f"=== {shape['scenario']}: {shape['total_tokens']} tok ===")

    for i, b in enumerate(blocks):
        lat_lo = 60.0 - 120.0 * i / n
        lat_hi = lat_lo - 120.0 / n
        span_deg = b["share"] * 360.0
        lng_lo = -span_deg / 2
        lng_hi = span_deg / 2

        # Simulate S2 cells at this level: approximate cell count
        # At level L, each cell covers ~ 8.5e10 / 4^L square meters
        # For our purpose: cell_width_deg ≈ 360 / (2^(L+1) * sqrt(3))
        cells_at_level = int(span_deg / (360.0 / (2 ** (b["level"] + 1))))
        cells_at_level = max(1, min(cells_at_level, grid_cols))

        glyph = format(b["level"], 'x')[-1]

        for row in range(rows_per_block):
            line = ""
            for col in range(grid_cols):
                lng = -180.0 + 360.0 * (col + 0.5) / grid_cols
                if lng_lo <= lng <= lng_hi:
                    line += glyph
                else:
                    line += "."
            if row == 0:
                lines.append(
                    f"{b['name']:>10} L{b['level']:<2}|{line}| "
                    f"{cells_at_level:>3}c {b['tokens']:>5}t"
                )
            else:
                lines.append(f"{'':>13}|{line}|")

    # Level histogram
    hist = {}
    for b in blocks:
        lvl = b["level"]
        cells = int(b["share"] * 512)
        hist[lvl] = hist.get(lvl, 0) + max(1, cells)

    hist_str = " ".join(f"L{l}:{c}" for l, c in sorted(hist.items()))
    lines.append(f"  superimposition: {sum(hist.values())} cells [{hist_str}]")

    return "\n".join(lines)


# ═══════════════════════════════════════════════════════════════════════════════
# Results reader
# ═══════════════════════════════════════════════════════════════════════════════

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


# ═══════════════════════════════════════════════════════════════════════════════
# Chart generators
# ═══════════════════════════════════════════════════════════════════════════════

# Consistent color palette across all charts
COLORS = {
    "baseline":     "#3498db",  # Blue
    "full":          "#2ecc71",  # Green
    "hermes_proxy": "#e67e22",  # Orange
    "proxy_api":    "#9b59b6",  # Purple
    "s2_navigate":  "#e74c3c",  # Red
}

SCENARIO_LABELS = {
    "baseline":     "Baseline (no proxy)",
    "full":          "Full Compression",
    "hermes_proxy": "Hermes↔Proxy (cache)",
    "proxy_api":    "Proxy↔API (token)",
    "s2_navigate":  "S2 Navigation (cache + index)",
}


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
    fig.suptitle("Aphrodite Conversational Benchmark — Token Analysis",
                 fontsize=16, fontweight="bold")

    bar_width = 0.2
    x = np.arange(n_conv)

    # Chart 1: Total tokens
    ax = axes[0]
    for i, scenario in enumerate(scenarios):
        values = []
        for conv in conversations:
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
            val = matches[0].get("total_tokens", 0) if matches else 0
            values.append(val)
        bars = ax.bar(x + i * bar_width, values, bar_width,
                      label=SCENARIO_LABELS.get(scenario, scenario),
                      color=COLORS.get(scenario, "#888"))
        # Add value labels on bars
        for bar in bars:
            height = bar.get_height()
            if height > 0:
                ax.annotate(f'{height:,}',
                            xy=(bar.get_x() + bar.get_width() / 2, height),
                            xytext=(0, 3), textcoords="offset points",
                            ha='center', va='bottom', fontsize=8)

    ax.set_title("Total Tokens per Conversation")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha='right')
    ax.set_ylabel("Tokens")
    ax.legend(fontsize=8, loc='upper left')
    ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))

    # Chart 2: Prompt vs Completion breakdown
    ax = axes[1]
    prompt_data = {}
    comp_data = {}
    for scenario in scenarios:
        prompt_data[scenario] = []
        comp_data[scenario] = []
        for conv in conversations:
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
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
        ax.bar(x + i * bar_width, p, bar_width, bottom=bottom,
               color=COLORS.get(scenario, "#888"), alpha=0.7,
               label=f"{SCENARIO_LABELS.get(scenario, scenario)} (prompt)")
        ax.bar(x + i * bar_width, c, bar_width, bottom=p,
               color=COLORS.get(scenario, "#888"), alpha=0.4,
               label=f"{SCENARIO_LABELS.get(scenario, scenario)} (completion)")

    ax.set_title("Prompt vs Completion Tokens")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha='right')
    ax.set_ylabel("Tokens")
    ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))

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
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
            total = matches[0].get("total_tokens", 0) if matches else 0
            baseline = baseline_totals.get(conv, 1)
            ratio = (total / baseline * 100) if baseline > 0 else 100
            values.append(ratio)
        bars = ax.bar(x + i * bar_width, values, bar_width,
                      label=SCENARIO_LABELS.get(scenario, scenario),
                      color=COLORS.get(scenario, "#888"))
        for bar in bars:
            height = bar.get_height()
            ax.annotate(f'{height:.0f}%',
                        xy=(bar.get_x() + bar.get_width() / 2, height),
                        xytext=(0, 3), textcoords="offset points",
                        ha='center', va='bottom', fontsize=8)

    ax.set_title("Tokens vs Baseline (%)")
    ax.set_xticks(x + bar_width * (n_scen - 1) / 2)
    ax.set_xticklabels(conversations, rotation=15, ha='right')
    ax.set_ylabel("% of Baseline")
    ax.axhline(y=100, color='red', linestyle='--', alpha=0.5, label='Baseline (100%)')
    ax.legend(fontsize=8, loc='upper left')

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ token_comparison.png saved")


def render_timeline(manifest: dict, output_path: Path):
    """Per-turn token usage timeline for each scenario."""
    results = manifest.get("results", [])
    if not results:
        return

    conversations = sorted(set(r["conversation"] for r in results))
    scenarios = sorted(set(r["scenario"] for r in results))
    n_conv = len(conversations)
    n_scen = len(scenarios)

    fig, axes = plt.subplots(n_conv, 1, figsize=(16, 4 * n_conv),
                             squeeze=False)
    fig.suptitle("Per-Turn Token Usage Timeline", fontsize=16, fontweight="bold")

    for ci, conv_name in enumerate(conversations):
        ax = axes[ci][0]

        for scenario in scenarios:
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv_name]
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

            ax.plot(indices, prompt, 'o-', markersize=4, linewidth=1.5,
                    color=COLORS.get(scenario, "#888"), alpha=0.5,
                    label=f"{SCENARIO_LABELS.get(scenario, scenario)} prompt")
            ax.plot(indices, total, 's-', markersize=5, linewidth=2,
                    color=COLORS.get(scenario, "#888"),
                    label=f"{SCENARIO_LABELS.get(scenario, scenario)} total")

        ax.set_title(f"{conv_name}")
        ax.set_xlabel("Turn")
        ax.set_ylabel("Tokens")
        ax.legend(fontsize=7, loc='upper left')
        ax.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))
        ax.grid(True, alpha=0.3)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ token_timeline.png saved")


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
    fig.suptitle("Compression Efficiency: Prompt Token Savings vs Baseline",
                 fontsize=14, fontweight="bold")

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
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
            if matches:
                prompt = matches[0].get("total_prompt_tokens", 0)
                bl = baseline_prompts.get(conv, 1)
                pct = (1 - prompt / bl) * 100 if bl > 0 else 0
                savings_pct.append(max(0, pct))
            else:
                savings_pct.append(0)

        bars = ax.bar(x + i * bar_width, savings_pct, bar_width,
                      label=SCENARIO_LABELS.get(scenario, scenario),
                      color=COLORS.get(scenario, "#888"))
        for bar in bars:
            height = bar.get_height()
            if abs(height) > 0.5:
                ax.annotate(f'{height:.1f}%',
                            xy=(bar.get_x() + bar.get_width() / 2, height),
                            xytext=(0, 3), textcoords="offset points",
                            ha='center', va='bottom', fontsize=9)

    ax.set_xticks(x + bar_width)
    ax.set_xticklabels(conversations, rotation=15, ha='right')
    ax.set_ylabel("Prompt Token Savings (%)")
    ax.axhline(y=0, color='black', linewidth=0.5)
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3, axis='y')

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ compression_efficiency.png saved")


def render_s2_context_maps(manifest: dict, output_path: Path):
    """Render S2 context-shape maps for each scenario as a multi-panel figure."""
    scenarios = sorted(set(r["scenario"] for r in manifest.get("results", [])))
    if not scenarios:
        return

    n = len(scenarios)
    cols = min(2, n)
    rows = math.ceil(n / cols)

    fig, axes = plt.subplots(rows, cols, figsize=(8 * cols, 5 * rows),
                             squeeze=False)
    fig.suptitle("Context Shape Maps — S2 Level Distribution per Scenario",
                 fontsize=16, fontweight="bold")

    for i, scenario in enumerate(scenarios):
        ax = axes[i // cols][i % cols]
        shape = generate_context_shape(scenario)

        blocks = shape["blocks"]
        block_names = [b["name"] for b in blocks]
        block_tokens = [b["tokens"] for b in blocks]
        block_levels = [b["level"] for b in blocks]
        colors_list = [plt.cm.viridis(b["level"] / 16) for b in blocks]

        # Horizontal bars: token allocation per block, colored by S2 level
        y_pos = range(len(blocks))
        bars = ax.barh(y_pos, block_tokens, color=colors_list, edgecolor='white')

        # Add level labels
        for j, (bar, level) in enumerate(zip(bars, block_levels)):
            ax.text(bar.get_width() + 20, bar.get_y() + bar.get_height() / 2,
                    f'L{level}', va='center', fontsize=9, fontweight='bold')

        ax.set_yticks(y_pos)
        ax.set_yticklabels(block_names)
        ax.set_xlabel("Token Budget")
        ax.set_title(f"{SCENARIO_LABELS.get(scenario, scenario)}")
        ax.invert_yaxis()

        # Add a colorbar showing S2 level scale
        sm = plt.cm.ScalarMappable(cmap='viridis', norm=plt.Normalize(0, 16))
        sm.set_array([])
        cbar = plt.colorbar(sm, ax=ax, shrink=0.8)
        cbar.set_label("S2 Level (coarse → fine)")

    # Hide empty subplots
    for j in range(i + 1, rows * cols):
        axes[j // cols][j % cols].set_visible(False)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ s2_context_maps.png saved")


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
    fig.suptitle("Context Shape Comparison — Radar View", fontsize=14, fontweight="bold")

    for scenario in scenarios:
        shape = generate_context_shape(scenario)
        values = [b["tokens"] for b in shape["blocks"]]
        # Normalize to 0-1 for radar
        max_val = max(values) if max(values) > 0 else 1
        values = [v / max_val for v in values]
        values += values[:1]

        ax.fill(angles, values, alpha=0.15, color=COLORS.get(scenario, "#888"))
        ax.plot(angles, values, 'o-', linewidth=2, label=SCENARIO_LABELS.get(scenario, scenario),
                color=COLORS.get(scenario, "#888"))

    ax.set_xticks(angles[:-1])
    ax.set_xticklabels(dimensions, fontsize=9)
    ax.set_yticklabels([])
    ax.legend(loc='upper right', bbox_to_anchor=(1.3, 1.1), fontsize=9)
    ax.set_title("Token allocation per context block\n(normalized per scenario)", pad=20)

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ radar_chart.png saved")


def render_summary_dashboard(manifest: dict, output_path: Path):
    """Single-page summary dashboard with key metrics."""
    results = manifest.get("results", [])
    if not results:
        return

    fig = plt.figure(figsize=(22, 14))
    fig.suptitle(f"Aphrodite Conversational Benchmark — Run {manifest.get('run_id', 'unknown')}",
                 fontsize=16, fontweight="bold")

    # Use GridSpec: top row 3 cols, bottom row 4 cols
    from matplotlib.gridspec import GridSpec
    gs = GridSpec(2, 4, figure=fig, height_ratios=[1, 1.2], hspace=0.35, wspace=0.3)

    # ── Top-left: Summary table ──
    ax_table = fig.add_subplot(gs[0, :2])
    ax_table.axis('off')

    table_data = []
    table_cols = ["Scenario", "Conversation", "Turns", "Prompt", "Completion", "Total", "Errors"]
    for r in results:
        table_data.append([
            r["scenario"],
            r["conversation"],
            str(r.get("turns", len(r.get("detail", {}).get("turns", [])))),
            f'{r.get("total_prompt_tokens", 0):,}',
            f'{r.get("total_completion_tokens", 0):,}',
            f'{r.get("total_tokens", 0):,}',
            str(r.get("errors", 0)),
        ])

    if table_data:
        tbl = ax_table.table(
            cellText=table_data, colLabels=table_cols,
            cellLoc='center', loc='center',
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
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
            values.append(matches[0].get("total_tokens", 0) if matches else 0)
        ax_bars.bar(x + i * bar_width, values, bar_width,
                    label=scenario, color=COLORS.get(scenario, "#888"))

    ax_bars.set_title("Total Tokens by Scenario")
    ax_bars.set_xticks(x + bar_width * (len(scenarios) - 1) / 2)
    ax_bars.set_xticklabels(conversations, rotation=15, ha='right', fontsize=8)
    ax_bars.legend(fontsize=7)
    ax_bars.yaxis.set_major_formatter(mticker.FuncFormatter(lambda x, _: f'{x:,.0f}'))

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
            matches = [r for r in results if r["scenario"] == scenario and r["conversation"] == conv]
            total = matches[0].get("total_tokens", 0) if matches else 0
            bl = baseline_totals.get(conv, 1)
            values.append((1 - total / bl) * 100 if bl > 0 else 0)
        ax_savings.bar(x + i * bar_width, values, bar_width,
                       label=scenario, color=COLORS.get(scenario, "#888"))

    ax_savings.set_title("Token Savings vs Baseline (%)")
    ax_savings.set_xticks(x + bar_width)
    ax_savings.set_xticklabels(conversations, rotation=15, ha='right', fontsize=8)
    ax_savings.axhline(y=0, color='black', linewidth=0.5)
    ax_savings.legend(fontsize=7)

    # ── Bottom row: S2 Context Shape Maps (one per scenario) ──
    for i, scenario in enumerate(scenarios):
        if i >= 4:
            break
        ax_s2 = fig.add_subplot(gs[1, i])
        shape = generate_context_shape(scenario)

        blocks = shape["blocks"]
        names = [b["name"] for b in blocks]
        tokens = [b["tokens"] for b in blocks]
        levels = [b["level"] for b in blocks]
        colors_list = [plt.cm.viridis(b["level"] / 16) for b in blocks]

        ax_s2.barh(range(len(blocks)), tokens, color=colors_list, edgecolor='white')
        for j, (_, lvl) in enumerate(zip(range(len(blocks)), levels)):
            ax_s2.text(tokens[j] + 10, j, f'L{lvl}', va='center', fontsize=7, fontweight='bold')
        ax_s2.set_yticks(range(len(blocks)))
        ax_s2.set_yticklabels(names, fontsize=7)
        ax_s2.set_title(f"{SCENARIO_LABELS.get(scenario, scenario)}\nS2 Context Shape", fontsize=9)
        ax_s2.invert_yaxis()

    plt.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  ✓ summary_dashboard.png saved")


# ═══════════════════════════════════════════════════════════════════════════════
# ASCII context maps (text output)
# ═══════════════════════════════════════════════════════════════════════════════

def generate_ascii_maps(results_dir: Path):
    """Generate and save ASCII S2 context maps for each scenario."""
    maps_dir = results_dir / "context_maps"
    maps_dir.mkdir(exist_ok=True)

    for scenario in ["baseline", "full", "hermes_proxy", "proxy_api"]:
        shape = generate_context_shape(scenario)
        ascii_map = render_s2_ascii(shape)

        map_path = maps_dir / f"{scenario}.txt"
        with open(map_path, "w") as f:
            f.write(ascii_map)

    print(f"  ✓ ASCII context maps saved to {maps_dir}")


# ═══════════════════════════════════════════════════════════════════════════════
# Main visualization orchestrator
# ═══════════════════════════════════════════════════════════════════════════════

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
    render_s2_context_maps(manifest, viz_dir / "s2_context_maps.png")
    render_radar_chart(manifest, viz_dir / "radar_chart.png")
    render_summary_dashboard(manifest, viz_dir / "summary_dashboard.png")

    # ASCII context maps
    generate_ascii_maps(viz_dir)

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


# ═══════════════════════════════════════════════════════════════════════════════
# CLI
# ═══════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Aphrodite Benchmark Visualization")
    parser.add_argument("results_dir", help="Path to benchmark results directory")
    parser.add_argument("--ascii-only", action="store_true",
                        help="Only generate ASCII context maps (no PNGs)")
    args = parser.parse_args()

    results_dir = Path(args.results_dir)
    if args.ascii_only:
        generate_ascii_maps(results_dir)
    else:
        visualize_run(results_dir)
