"""Configuration helpers for hermes-compress — hot-reload safe.

Reads compression.headroom.* from Hermes config on each call.
Values are cached for _CACHE_TTL seconds to avoid filesystem
overhead. Config changes take effect without restart.
"""

from __future__ import annotations

import os
import time
import logging
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

# ── Cache ───────────────────────────────────────────────────────────
_CACHE_TTL = 5.0  # seconds — re-read config at most every 5s
_cache: dict = {"ts": 0.0, "data": None}

# ── Defaults ────────────────────────────────────────────────────────
_DEFAULT_INTEGRATION = "hybrid"  # hooks + patcher, safest default
_VALID_INTEGRATIONS = frozenset({"hook", "hybrid", "waterfall", "proxy"})
_ENV_OVERRIDE = "HERMES_COMPRESS_INTEGRATION"  # for testing


def _read_config(reload: bool = False) -> dict:
    """Read the headroom section from config.yaml. Cached for _CACHE_TTL."""
    global _cache
    now = time.monotonic()
    if not reload and _cache["data"] is not None and (now - _cache["ts"]) < _CACHE_TTL:
        return _cache["data"]

    try:
        config_path = Path.home() / ".hermes" / "config.yaml"
        if not config_path.exists():
            _cache = {"ts": now, "data": {}}
            return {}

        # Inline YAML parsing to avoid dependency issues
        text = config_path.read_text()
        headroom_cfg = _parse_yaml_headroom(text)
        _cache = {"ts": now, "data": headroom_cfg}
        return headroom_cfg
    except Exception:
        if _cache["data"] is not None:
            return _cache["data"]
        return {}


def _parse_yaml_headroom(text: str) -> dict:
    """Parse just the compression.headroom section from YAML text.
    
    Avoids yaml module dependency. Simple line-based parser
    that handles the nested compression.headroom section.
    """
    in_compression = False
    in_headroom = False
    indent = ""
    result = {}

    for line in text.split("\n"):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        # Track section nesting
        if not in_compression:
            if stripped == "compression:" or stripped.startswith("compression:"):
                in_compression = True
                indent = line[:len(line) - len(line.lstrip())]
            continue

        if not in_headroom:
            if stripped == "headroom:" or stripped.startswith("headroom:"):
                in_headroom = True
                indent = line[:len(line) - len(line.lstrip())]
            elif not line.startswith(indent + " ") and not line.startswith(indent + "\t"):
                # Left the compression section
                if line and not line[0].isspace():
                    in_compression = False
            continue

        # Inside headroom section
        if not line.startswith(indent + " ") and not line.startswith(indent + "\t"):
            if line and not line[0].isspace():
                break  # Left headroom section
            continue

        # Parse key: value
        kv = stripped.split(":", 1)
        if len(kv) == 2:
            key = kv[0].strip()
            value = kv[1].strip().strip('"').strip("'")
            result[key] = value

    return result


def _get_integration_mode() -> str:
    """Get the active integration mode from config.
    
    Reads compression.headroom.integration from config.yaml.
    Falls back to _DEFAULT_INTEGRATION ("hybrid") if not set or invalid.
    Environment variable HERMES_COMPRESS_INTEGRATION overrides for testing.
    """
    # Env override (for testing)
    env_val = os.environ.get(_ENV_OVERRIDE)
    if env_val:
        env_val = env_val.strip().lower()
        if env_val in _VALID_INTEGRATIONS:
            return env_val

    cfg = _read_config()
    mode = str(cfg.get("integration", "")).strip().lower()

    if mode in _VALID_INTEGRATIONS:
        return mode

    if mode:
        logger.debug("hermes-compress: unknown integration mode '%s', using default", mode)

    return _DEFAULT_INTEGRATION


def _get_integration_mode_safe() -> str:
    """Same as _get_integration_mode but never raises. For tests."""
    try:
        return _get_integration_mode()
    except Exception:
        return _DEFAULT_INTEGRATION


def get_headroom_config() -> dict:
    """Get all headroom config values as a dict.
    
    Returns values with defaults applied:
      - enabled: bool
      - mode: str ("inline" | "token" | "proxy")
      - integration: str ("hook" | "hybrid" | "waterfall" | "proxy")
      - protect_recent: int
      - target_ratio: float | None
      - min_tokens_to_compress: int
      - precompress_tools: bool
      - aggressive_kompress: bool
      - deduplicate_results: bool
      - verbose_stats: bool
    """
    cfg = _read_config()

    enabled = str(cfg.get("enabled", "false")).lower() in {"true", "1", "yes"}
    mode = str(cfg.get("mode", "inline")).lower()
    integration = str(cfg.get("integration", "")).strip().lower()
    if integration not in _VALID_INTEGRATIONS:
        integration = _DEFAULT_INTEGRATION

    protect_recent = int(cfg.get("protect_recent", 4))
    target_ratio = cfg.get("target_ratio")
    if target_ratio is not None:
        try:
            target_ratio = float(target_ratio)
        except (TypeError, ValueError):
            target_ratio = None

    min_tokens = int(cfg.get("min_tokens_to_compress", 250))
    precompress = str(cfg.get("precompress_tools", "false")).lower() in {"true", "1", "yes"}
    aggressive = str(cfg.get("aggressive_kompress", "false")).lower() in {"true", "1", "yes"}
    dedup = str(cfg.get("deduplicate_results", "false")).lower() in {"true", "1", "yes"}
    verbose = str(cfg.get("verbose_stats", "false")).lower() in {"true", "1", "yes"}

    return {
        "enabled": enabled,
        "mode": mode,
        "integration": integration,
        "protect_recent": protect_recent,
        "target_ratio": target_ratio,
        "min_tokens_to_compress": min_tokens,
        "precompress_tools": precompress,
        "aggressive_kompress": aggressive,
        "deduplicate_results": dedup,
        "verbose_stats": verbose,
    }
