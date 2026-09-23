"""Shared chart constants (per-scenario colors + labels)."""

from __future__ import annotations

COLORS = {
    "baseline": "#3498db",  # Blue
    "full": "#2ecc71",  # Green
    "hermes_proxy": "#e67e22",  # Orange
    "proxy_api": "#9b59b6",  # Purple
}

SCENARIO_LABELS = {
    "baseline": "Baseline (no proxy)",
    "full": "Full Compression",
    "hermes_proxy": "Hermes↔Proxy (cache)",
    "proxy_api": "Proxy↔API (token)",
}
