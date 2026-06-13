"""
Dev mode - testing, simulation, and diagnostics for hermes-compress.

Activate with: HERMES_COMPRESS_DEV=1

Features:
  - Per-message stats tracking
  - Dry-run mode (compress but don't modify messages)
  - Feature flags for experimental optimizations
  - Simulated backpressure testing
  - Stats ingestion and analysis
  - Verbose logging

All dev features are gated behind the env var - zero overhead in production.
"""

from __future__ import annotations

import json
import logging
import os
import time
from dataclasses import dataclass, field
from typing import Any, Optional

logger = logging.getLogger(__name__)

# ── Activation ────────────────────────────────────────────────────────


def is_dev() -> bool:
    """Check if dev mode is active."""
    return os.getenv("HERMES_COMPRESS_DEV", "").strip().lower() in {
        "1", "true", "yes", "on",
    }


# ── Feature flags ─────────────────────────────────────────────────────


@dataclass
class DevFlags:
    """Experimental feature flags - all default False in production."""

    # Pre-processing
    strip_ansi: bool = True
    truncate_repeats: bool = True
    strip_debug: bool = True
    compress_patterns: bool = True
    strip_ccr: bool = True

    # Double-pass compression
    precompress_tool_outputs: bool = False

    # Diagnostics
    dry_run: bool = False       # Compress but don't modify messages
    verbose_stats: bool = False  # Per-message detailed stats
    simulate_backpressure: bool = False
    backpressure_delay_ms: int = 0

    # Advanced
    aggressive_kompress: bool = False
    deduplicate_tool_results: bool = False

    # Zero-fidelity optimization
    optimize_content: bool = False
    round_json_numbers: bool = True
    normalize_paths: bool = True
    shorten_timestamps: bool = True

    @classmethod
    def from_env(cls) -> "DevFlags":
        """Read flags from HERMES_COMPRESS_FLAGS env var.

        Format: comma-separated key=value pairs.
        Example: HERMES_COMPRESS_FLAGS=dry_run=1,verbose_stats=1
        """
        flags = cls()
        raw = os.getenv("HERMES_COMPRESS_FLAGS", "")
        if not raw:
            return flags

        for pair in raw.split(","):
            pair = pair.strip()
            if "=" not in pair:
                continue
            key, val = pair.split("=", 1)
            key = key.strip()
            val = val.strip().lower()

            if hasattr(flags, key):
                if val in {"1", "true", "yes", "on"}:
                    setattr(flags, key, True)
                elif val in {"0", "false", "no", "off"}:
                    setattr(flags, key, False)
                else:
                    try:
                        setattr(flags, key, int(val))
                    except ValueError:
                        pass

        return flags


# ── Stats collection ──────────────────────────────────────────────────


@dataclass
class CallStats:
    """Per-call compression statistics."""
    timestamp: float = field(default_factory=time.time)
    messages_in: int = 0
    chars_before: int = 0
    chars_after: int = 0
    tokens_before: int = 0
    tokens_after: int = 0
    tokens_saved: int = 0
    preprocess_saved: int = 0
    precompress_saved: int = 0
    duration_ms: float = 0.0
    preprocess_ms: float = 0.0
    precompress_ms: float = 0.0
    compress_ms: float = 0.0
    transforms: list[str] = field(default_factory=list)
    tool_types: dict[str, int] = field(default_factory=dict)
    error: Optional[str] = None


class StatsCollector:
    """Collect and analyze per-call compression statistics.

    Only active when HERMES_COMPRESS_DEV=1.
    """

    def __init__(self) -> None:
        self.calls: list[CallStats] = []
        self._started: float = time.time()

    def record(self, stats: CallStats) -> None:
        if is_dev():
            self.calls.append(stats)
            if len(self.calls) % 10 == 0:
                logger.info("dev: %d calls recorded, %.1f%% avg savings",
                            len(self.calls), self.avg_savings * 100)

    @property
    def avg_savings(self) -> float:
        if not self.calls:
            return 0.0
        total_before = sum(c.tokens_before for c in self.calls)
        total_saved = sum(c.tokens_saved for c in self.calls)
        return total_saved / total_before if total_before > 0 else 0.0

    @property
    def avg_latency_ms(self) -> float:
        if not self.calls:
            return 0.0
        return sum(c.duration_ms for c in self.calls) / len(self.calls)

    def summary(self) -> dict[str, Any]:
        if not self.calls:
            return {"calls": 0, "uptime_seconds": time.time() - self._started}

        tokens_before = sum(c.tokens_before for c in self.calls)
        tokens_after = sum(c.tokens_after for c in self.calls)

        # Per-tool breakdown
        tool_savings: dict[str, dict] = {}
        for c in self.calls:
            for tool, count in c.tool_types.items():
                entry = tool_savings.setdefault(tool, {"count": 0, "saved": 0})
                entry["count"] += count
                if c.tokens_saved > 0 and c.messages_in > 0:
                    entry["saved"] += c.tokens_saved * (count / c.messages_in)

        return {
            "calls": len(self.calls),
            "uptime_seconds": time.time() - self._started,
            "tokens_before": tokens_before,
            "tokens_after": tokens_after,
            "tokens_saved": tokens_before - tokens_after,
            "avg_savings_pct": round(self.avg_savings * 100, 1),
            "avg_latency_ms": round(self.avg_latency_ms, 1),
            "peak_savings_pct": round(max(
                (c.tokens_saved / c.tokens_before * 100)
                for c in self.calls if c.tokens_before > 0
            ), 1) if self.calls else 0,
            "preprocess_total_saved": sum(c.preprocess_saved for c in self.calls),
            "precompress_total_saved": sum(c.precompress_saved for c in self.calls),
            "by_tool": tool_savings,
            "errors": sum(1 for c in self.calls if c.error),
        }

    def dump(self) -> str:
        return json.dumps(self.summary(), indent=2)

    def replay_last(self, n: int = 10) -> str:
        """Return the last N calls as a formatted table."""
        if not self.calls:
            return "No calls recorded."

        recent = self.calls[-n:]
        lines = [
            f"{'#':>3} {'msgs':>4} {'before':>8} {'after':>8} {'saved':>7} {'%':>6} {'ms':>6}",
            "-" * 52,
        ]
        for i, c in enumerate(recent, len(self.calls) - len(recent) + 1):
            pct = c.tokens_saved / c.tokens_before * 100 if c.tokens_before > 0 else 0
            lines.append(
                f"{i:3d} {c.messages_in:4d} {c.tokens_before:>8,d} {c.tokens_after:>8,d} "
                f"{c.tokens_saved:>7,d} {pct:5.1f}% {c.duration_ms:5.0f}ms"
            )
        return "\n".join(lines)


# Global singleton
_collector: Optional[StatsCollector] = None


def get_collector() -> StatsCollector:
    global _collector
    if _collector is None:
        _collector = StatsCollector()
    return _collector


# ── Backpressure simulation ───────────────────────────────────────────


def simulate_backpressure(delay_ms: int = 0) -> None:
    """Simulate backpressure by sleeping.

    Used in dev mode to test compression under load.
    Set HERMES_COMPRESS_FLAGS=simulate_backpressure=1,backpressure_delay_ms=50
    """
    if not is_dev():
        return
    if delay_ms > 0:
        time.sleep(delay_ms / 1000)
