"""
Core compression integration - the main pipeline that runs headroom
on Hermes agent messages before they reach the LLM.

Mirrors the TypeScript Function/Integration.ts pattern (342 lines).
Adapted from Astro build integration to Hermes agent loop integration.

Pipeline:
  1. CacheAligner    - stabilizes message prefixes for KV cache hits
  2. ContentRouter   - auto-detects content type per message
  3. SmartCrusher    - JSON arrays, structured data
  4. CodeCompressor  - source code (AST-aware, 6 languages)
  5. Kompress        - prose, logs, ML-based text compression
"""

from __future__ import annotations

import logging
import time
from typing import Any, Optional

from hermes_compress._option import CompressOption
from hermes_compress._function._bytes import Bytes
from hermes_compress._variable._option import ToolHints, DefaultOption

logger = logging.getLogger(__name__)


class Integration:
    """Compression integration - the bridge between Hermes and headroom.

    Loaded once per session. Handles config resolution, headroom
    availability probing, and per-call compression with observability.

    Parameters:
        model: The LLM model name (for token counting).
        option: Compression options override (merges with defaults).
    """

    def __init__(
        self,
        model: str = "",
        option: Optional[CompressOption] = None,
    ) -> None:
        self.model = model
        self._option = option or DefaultOption
        self._available: Optional[bool] = None

        # Accumulated stats for the session
        self.Total: dict[str, Any] = {
            "calls": 0,
            "messages_processed": 0,
            "tokens_before": 0,
            "tokens_after": 0,
            "tokens_saved": 0,
            "duration_ms": 0.0,
            "errors": 0,
            "by_tool": {},
        }

    # ── Public API ──────────────────────────────────────────────────────

    @property
    def enabled(self) -> bool:
        return self._option.Enabled

    def compress(
        self, messages: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        """Compress messages and return the (possibly compressed) list.

        On any failure, returns original messages unchanged.
        """
        if not self.enabled or not messages:
            return messages

        if self._available is None:
            self._available = self._probe()

        if not self._available:
            return messages

        start = time.monotonic()
        before = sum(len(str(m)) for m in messages)
        self.Total["messages_processed"] += len(messages)

        try:
            from headroom import compress as _headroom_compress

            kwargs = {
                "protect_recent": self._option.ProtectRecent,
                "min_tokens_to_compress": self._option.MinTokensToCompress,
            }
            if self._option.TargetRatio is not None:
                kwargs["target_ratio"] = self._option.TargetRatio

            result = _headroom_compress(
                messages,
                model=self.model or "gpt-4o",
                **kwargs,
            )

            duration_ms = (time.monotonic() - start) * 1000
            after = sum(len(str(m)) for m in result.messages)

            # Update totals
            self.Total["calls"] += 1
            self.Total["tokens_before"] += result.tokens_before
            self.Total["tokens_after"] += result.tokens_after
            self.Total["tokens_saved"] += result.tokens_saved
            self.Total["duration_ms"] += duration_ms

            if result.tokens_saved > 0:
                pct = result.compression_ratio * 100
                logger.info(
                    "hermes-compress: %s → %s (-%s, %.1f%%) in %.0fms [%d msgs]",
                    Bytes(before), Bytes(after),
                    Bytes(before - after), pct,
                    duration_ms, len(messages),
                )

            return result.messages

        except ImportError:
            self._available = False
            logger.debug("headroom not available")
            return messages
        except Exception as exc:
            self.Total["errors"] += 1
            logger.warning("hermes-compress failed: %s", exc)
            return messages

    def stats_summary(self) -> str:
        """Return a human-readable stats summary."""
        t = self.Total
        if t["calls"] == 0:
            return "No compression calls yet."

        avg_ms = t["duration_ms"] / t["calls"] if t["calls"] else 0
        saved = t["tokens_saved"]
        pct = (saved / t["tokens_before"] * 100) if t["tokens_before"] else 0

        lines = [
            f"Calls: {t['calls']}",
            f"Messages processed: {t['messages_processed']}",
            f"Tokens: {t['tokens_before']:,} → {t['tokens_after']:,}",
            f"Saved: {saved:,} tokens ({pct:.1f}%)",
            f"Avg latency: {avg_ms:.0f}ms",
        ]
        if t["errors"]:
            lines.append(f"Errors: {t['errors']}")
        return "\n".join(lines)

    @staticmethod
    def _probe() -> bool:
        """Check if headroom is importable."""
        try:
            import headroom  # noqa: F401
            return True
        except ImportError:
            return False
