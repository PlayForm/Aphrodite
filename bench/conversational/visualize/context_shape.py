"""Context-shape generator for scenario visualization."""

from __future__ import annotations

CONTEXT_BLOCKS = [
    ("system", 120, 3, "System prompt"),
    ("directives", 420, 5, "Behavioral directives"),
    ("nudges", 80, 7, "Per-turn nudges"),
    ("plain", 1000, 8, "Plain data / tool outputs"),
    ("recall", 1200, 10, "CCR recall / catalog"),
    ("hint", 60, 6, "Retrieve hint"),
    ("convo", 1600, 12, "Conversation history"),
]

# Per-scenario block token adjustments (scenarios skew the context differently)
SCENARIO_BLOCKS = {
    "baseline": {
        # No compression - full tool outputs in plain data
        "plain": 4000,  # Much larger: full tool outputs inline
        "recall": 0,  # No CCR recall needed
    },
    "full": {
        # Full compression - tool outputs compressed, messages offloaded
        "plain": 600,  # Smaller: markers replace tool outputs
        "recall": 1800,  # Larger: CCR catalog + retrieval
    },
    "hermes_proxy": {
        # Cache proxy only - tool outputs compressed, no message offloading
        "plain": 800,  # Compressed tool outputs
        "recall": 1400,  # CCR catalog
    },
    "proxy_api": {
        # Token proxy only - messages offloaded, tool outputs inline
        "plain": 3000,  # Still large (full tool outputs)
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
        blocks.append(
            {
                "name": name,
                "description": desc,
                "tokens": adj_tokens,
                "level": level,
                "share": adj_tokens / max(total_tokens, 1),
            }
        )

    # Normalize shares
    total = sum(b["tokens"] for b in blocks)
    for b in blocks:
        b["share"] = b["tokens"] / max(total, 1)

    return {
        "scenario": scenario,
        "total_tokens": total,
        "blocks": blocks,
    }
