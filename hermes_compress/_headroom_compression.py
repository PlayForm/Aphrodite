"""
Headroom compression integration for Hermes Agent.

Thin wrapper that delegates to @playform/hermes-compress plugin.
All features (pre-processing, dev mode, double-pass, strategies,
truncation, dedup, hot-reload) are provided by the plugin's Compress class.

Install: hermes-compress install
"""

from hermes_compress import Compress as _Compress
from hermes_compress import CompressResult as _CompressResult
from hermes_compress._option import CompressOption


class HeadroomCompressor:
    """Thin wrapper around hermes_compress.Compress.

    Accepts the kwargs that agent_init.py passes and translates them
    to a CompressOption. All compression logic lives in the plugin.
    """

    def __init__(
        self,
        model: str = "",
        enabled: bool = False,
        mode: str = "token",
        protect_recent: int = 4,
        target_ratio: float | None = None,
        min_tokens_to_compress: int = 250,
        precompress_tools: bool = False,
        aggressive_kompress: bool = False,
        deduplicate_results: bool = False,
        verbose_stats: bool = False,
        **kwargs,
    ):
        plugin_mode = "inline" if mode in ("token", "inline") else mode

        option = CompressOption(
            Enabled=enabled,
            Mode=plugin_mode,
            ProtectRecent=protect_recent,
            TargetRatio=target_ratio,
            MinTokensToCompress=min_tokens_to_compress,
            PrecompressTools=precompress_tools,
            AggressiveKompress=aggressive_kompress,
            DeduplicateResults=deduplicate_results,
            VerboseStats=verbose_stats,
        )
        self._compressor = _Compress(model=model, option=option)

    def __getattr__(self, name):
        return getattr(self._compressor, name)


HeadroomCompressionResult = _CompressResult

__all__ = ["HeadroomCompressor", "HeadroomCompressionResult"]
