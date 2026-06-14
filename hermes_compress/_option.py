"""
Compression options - mirrors the TypeScript Interface/Option.ts pattern.

All fields use PascalCase (PlayForm convention). Modes:
  "inline" - headroom runs in-process as a library (default, 50-80ms warm)
  "proxy"  - headroom runs as a separate proxy server (port 8787, zero code changes)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CompressOption:
    """Compression configuration.

    Attributes:
        Enabled: Master on/off switch.
        Mode: ``"inline"`` (library, default) or ``"proxy"`` (separate server).
        ProtectRecent: Number of most recent messages to never compress.
        TargetRatio: Kompress keep ratio (0.0-1.0). None = model default (~15%).
        MinTokensToCompress: Messages below this size skip compression.
        Threshold: Minimum conversation token count before compression activates.
        ProxyPort: Port for proxy mode (default 8787).
        ProxyHost: Host for proxy mode (default 127.0.0.1).
        ProxyAutoStart: Start proxy automatically when plugin loads.
        PrecompressTools: Double-pass compression on large tool outputs.
        AggressiveKompress: Use most aggressive Kompress settings.
        DeduplicateResults: Skip identical tool results across turns.
        VerboseStats: Log detailed per-call stats.
    """

    Enabled: bool = False
    Mode: str = "inline"  # "inline" | "proxy"
    ProtectRecent: int = 4
    TargetRatio: Optional[float] = None
    MinTokensToCompress: int = 250
    Threshold: int = 0

    # ── Proxy mode ─────────────────────────────────────────────────
    ProxyPort: int = 8787
    ProxyHost: str = "127.0.0.1"
    ProxyAutoStart: bool = False

    # ── Advanced compression ───────────────────────────────────────
    PrecompressTools: bool = False
    AggressiveKompress: bool = False
    DeduplicateResults: bool = False
    VerboseStats: bool = False


@dataclass
class CompressToolHint:
    """Content type hint for routing tool outputs to the right compressor."""

    Name: str
    Hint: str  # "json" | "code" | "prose" | "mixed" | "skip" | "html"
    MinSize: int = 250
