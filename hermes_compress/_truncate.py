"""
Smart truncation for large tool outputs.

When tool outputs exceed practical limits, truncate intelligently
rather than sending everything to headroom (which slows down).
Strategies:
  - Head+tail: keep first N and last N chars/lines
  - Structured: keep structure, sample content
  - Adaptive: truncate based on content type
"""

from __future__ import annotations

from typing import Any


# ── Truncation strategies ─────────────────────────────────────────────


def truncate_head_tail(
    content: str,
    head_chars: int = 2000,
    tail_chars: int = 2000,
) -> str:
    """Keep the first and last N characters, collapse the middle."""
    if len(content) <= head_chars + tail_chars:
        return content

    head = content[:head_chars]
    tail = content[-tail_chars:]
    skipped = len(content) - head_chars - tail_chars
    return f"{head}\n\n[... {skipped:,} chars truncated ...]\n\n{tail}"


def truncate_lines(
    content: str,
    head_lines: int = 50,
    tail_lines: int = 30,
) -> str:
    """Keep first N and last N lines, collapse middle."""
    lines = content.splitlines()
    if len(lines) <= head_lines + tail_lines:
        return content

    head = "\n".join(lines[:head_lines])
    tail = "\n".join(lines[-tail_lines:])
    skipped = len(lines) - head_lines - tail_lines
    return f"{head}\n[... {skipped} lines truncated ...]\n{tail}"


def truncate_json(
    content: str,
    max_items: int = 100,
) -> str:
    """Truncate JSON arrays/objects - keep first N items."""
    import json

    if len(content) < 5000:
        return content

    try:
        data = json.loads(content)
    except (json.JSONDecodeError, ValueError):
        return truncate_head_tail(content)

    # Array truncation
    if isinstance(data, list) and len(data) > max_items:
        truncated = data[:max_items]
        return json.dumps({
            "_truncated": True,
            "_original_count": len(data),
            "_shown": max_items,
            "items": truncated,
        }, indent=2)

    # Dict with large list values
    if isinstance(data, dict):
        truncated = {}
        for key, value in data.items():
            if isinstance(value, list) and len(value) > max_items:
                truncated[key] = value[:max_items]
                truncated[f"_truncated_{key}"] = len(value)
            else:
                truncated[key] = value
        if len(truncated) > len(data):
            return json.dumps(truncated, indent=2)

    return content


def truncate_for_tool(
    content: str,
    tool_name: str = "",
    max_output_chars: int = 100000,
) -> str:
    """Smart truncation based on tool type and content size.

    Returns truncated content if it exceeds the threshold.
    """
    if not isinstance(content, str) or len(content) <= max_output_chars:
        return content

    # JSON tools: try structured truncation
    if tool_name in {"search_files", "web_search", "session_search", "skills_list"}:
        return truncate_json(content)

    # Code tools: keep head+tail
    if tool_name in {"read_file", "execute_code", "patch"}:
        return truncate_head_tail(content, head_chars=5000, tail_chars=3000)

    # Terminal/logs: keep lines
    if tool_name in {"terminal", "read_terminal", "browser_console"}:
        return truncate_lines(content, head_lines=100, tail_lines=50)

    # Default: head+tail
    return truncate_head_tail(content)


# ── Message deduplication ─────────────────────────────────────────────

_DEDUP_CACHE: dict[str, str] = {}
_DEDUP_MAX_SIZE = 50


def deduplicate_message(
    tool_name: str,
    content: str,
    max_cache: int = _DEDUP_MAX_SIZE,
) -> str | None:
    """Check if this tool result is identical to a previous one.

    Returns None if content is new, or a short reference if duplicate.
    """
    key = f"{tool_name}:{hash(content)}"

    # Check for exact duplicate
    for cached_key, cached_content in list(_DEDUP_CACHE.items()):
        if cached_content == content:
            return f"[Duplicate of previous {tool_name} result - see above]"

    # Store in cache
    if len(_DEDUP_CACHE) >= max_cache:
        # Evict oldest
        oldest = next(iter(_DEDUP_CACHE))
        del _DEDUP_CACHE[oldest]

    _DEDUP_CACHE[key] = content
    return None


def clear_dedup_cache() -> None:
    """Clear the deduplication cache."""
    _DEDUP_CACHE.clear()


# ── Plugin hot-reload detection ───────────────────────────────────────

_HOT_RELOAD_MTIMES: dict[str, float] = {}


def check_hot_reload(plugin_dir: str = "") -> bool:
    """Check if plugin files have changed since last check.

    Returns True if any file was modified - caller should reload.
    """
    import os
    from pathlib import Path

    if not plugin_dir:
        plugin_dir = str(Path(__file__).resolve().parent.parent)

    changed = False
    for root, dirs, files in os.walk(plugin_dir):
        dirs[:] = [d for d in dirs if not d.startswith(".") and d != "__pycache__"]
        for f in files:
            if not f.endswith(".py"):
                continue
            fp = os.path.join(root, f)
            try:
                mtime = os.path.getmtime(fp)
                if fp in _HOT_RELOAD_MTIMES and _HOT_RELOAD_MTIMES[fp] != mtime:
                    changed = True
                _HOT_RELOAD_MTIMES[fp] = mtime
            except OSError:
                pass

    return changed
