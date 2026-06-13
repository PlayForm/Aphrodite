"""
Per-tool compression strategies — custom headroom settings per Hermes tool.

Different tool outputs benefit from different compression approaches.
This module defines the optimal headroom kwargs for each tool type,
based on real session benchmarks.

Strategy tiers:
  aggressive — max compression, JSON-heavy tools (search_files, web_search)
  balanced  — moderate, mixed content (terminal, execute_code)
  code      — AST-aware, preserve logic (read_file, patch)
  prose     — ML-based, keep semantics (web_extract, skill_view)
  minimal   — light touch, tiny outputs (memory, write_file)
  skip      — never compress (vision, browser clicks)
"""

from __future__ import annotations

from typing import Any


# ── Strategy definitions ──────────────────────────────────────────────

STRATEGIES: dict[str, dict[str, Any]] = {
    "aggressive": {
        "tier": "aggressive",
        "protect_recent": 0,
        "min_tokens_to_compress": 50,
        "target_ratio": 0.10,  # keep only 10%
    },
    "balanced": {
        "tier": "balanced",
        "protect_recent": 1,
        "min_tokens_to_compress": 150,
        "target_ratio": None,  # model default
    },
    "code": {
        "tier": "code",
        "protect_recent": 0,
        "min_tokens_to_compress": 100,
        "target_ratio": 0.20,  # keep 20% — preserve imports/signatures
    },
    "prose": {
        "tier": "prose",
        "protect_recent": 1,
        "min_tokens_to_compress": 200,
        "target_ratio": 0.15,
    },
    "minimal": {
        "tier": "minimal",
        "protect_recent": 4,
        "min_tokens_to_compress": 500,
        "target_ratio": 0.50,
    },
    "skip": {
        "tier": "skip",
        "min_tokens_to_compress": 999999,
    },
}


# ── Per-tool strategy mapping ─────────────────────────────────────────

TOOL_STRATEGIES: dict[str, str] = {
    # JSON-heavy — aggressive SmartCrusher
    "search_files": "aggressive",
    "web_search": "aggressive",
    "session_search": "aggressive",
    "browser_get_images": "aggressive",
    "skills_list": "aggressive",
    "skill_manage": "aggressive",

    # Mixed content — balanced
    "terminal": "balanced",
    "execute_code": "balanced",
    "read_terminal": "balanced",
    "process": "balanced",
    "cronjob": "balanced",

    # Source code — code strategy
    "read_file": "code",
    "patch": "code",

    # Prose / docs — Kompress
    "web_extract": "prose",
    "delegate_task": "prose",
    "skill_view": "prose",

    # HTML snapshots — skip (binary/image content, headroom can't compress)
    "browser_navigate": "skip",
    "browser_snapshot": "skip",
    "browser_console": "skip",

    # Tiny — minimal or skip
    "todo": "minimal",
    "write_file": "skip",
    "memory": "skip",
    "clarify": "skip",
    "browser_click": "skip",
    "browser_type": "skip",
    "browser_scroll": "skip",
    "browser_press": "skip",
    "browser_back": "skip",
    "browser_vision": "skip",
    "vision_analyze": "skip",
    "image_gen": "skip",
    "tts": "skip",
    "video_gen": "skip",
}

# ── Dev-safe tool override ──────────────────────────────────────────
# When HERMES_COMPRESS_DEV=1, core development tools use minimal
# compression to prevent empty tool outputs during debugging.
_DEV_SAFE_OVERRIDE: dict[str, str] = {
    "read_file": "minimal",
    "terminal": "minimal",
    "execute_code": "minimal",
    "read_terminal": "minimal",
    "patch": "minimal",
    "write_file": "skip",
    "search_files": "balanced",  # downgrade from aggressive
}


def get_strategy(tool_name: str, dev_mode: bool = False) -> dict[str, Any]:
    """Get the compression strategy for a specific tool.

    When dev_mode=True, core development tools are downgraded to
    minimal compression to prevent empty outputs during debugging.
    """
    strategy_name = TOOL_STRATEGIES.get(tool_name, "balanced")
    if dev_mode and tool_name in _DEV_SAFE_OVERRIDE:
        strategy_name = _DEV_SAFE_OVERRIDE[tool_name]
    return STRATEGIES.get(strategy_name, STRATEGIES["balanced"])


def get_global_strategy() -> dict[str, Any]:
    """Get the default global compression strategy."""
    return STRATEGIES["balanced"]


def merge_strategies(
    global_kwargs: dict[str, Any],
    tool_name: str = "",
) -> dict[str, Any]:
    """Merge global kwargs with per-tool strategy.

    Per-tool values override global ones. Protect_recent uses the
    minimum of both (more aggressive wins).
    """
    tool_kwargs = get_strategy(tool_name) if tool_name else {}
    merged = {**global_kwargs}

    for key, value in tool_kwargs.items():
        if key == "protect_recent":
            # More aggressive (lower number) wins
            merged[key] = min(merged.get(key, 4), value)
        elif key == "min_tokens_to_compress":
            # Lower threshold wins (compress more)
            merged[key] = min(merged.get(key, 250), value)
        elif key not in merged or merged[key] is None:
            merged[key] = value

    return merged


# ── Assistant thinking block compression ──────────────────────────────

def compress_thinking_block(content: str, max_chars: int = 500) -> str:
    """Compress an assistant thinking/reasoning block.

    Thinking blocks can be verbose. Keep the first and last sentences,
    collapse the middle with a summary count.
    """
    if not content or len(content) <= max_chars:
        return content

    # Keep first sentence and last sentence
    sentences = [s.strip() for s in content.replace("\n", " ").split(".") if s.strip()]
    if len(sentences) <= 3:
        return content

    first = sentences[0] + "."
    last = sentences[-1] + "." if sentences[-1] else ""

    middle_count = len(sentences) - 2
    return f"{first} [... {middle_count} sentences] {last}"


# ── System prompt caching ─────────────────────────────────────────────

_SYSTEM_PROMPT_CACHE: dict[str, str] = {}


def get_cached_system_prompt(key: str) -> str | None:
    """Get a cached system prompt. Returns None on miss."""
    return _SYSTEM_PROMPT_CACHE.get(key)


def cache_system_prompt(key: str, content: str) -> None:
    """Cache a system prompt for reuse across turns."""
    _SYSTEM_PROMPT_CACHE[key] = content


def clear_system_prompt_cache() -> None:
    """Clear the system prompt cache (on model switch)."""
    _SYSTEM_PROMPT_CACHE.clear()
