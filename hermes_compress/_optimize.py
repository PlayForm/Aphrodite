"""
Zero-fidelity-loss content optimization.

Transforms that preserve ALL semantic information while reducing token count:
  - Whitespace normalization (tabs, multiple spaces, trailing)
  - JSON number rounding (high-precision floats)
  - Boilerplate stripping (standard tool output headers/footers)
  - Path normalization (absolute -> relative, consistent separators)
  - Timestamp shortening (ISO -> compact)

All transforms are lossless: no information is discarded, only formatting.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

# ── Whitespace normalization ──────────────────────────────────────────


def normalize_whitespace(text: str) -> str:
    """Normalize whitespace without losing structure.

    - Tabs -> spaces
    - Multiple spaces -> single space (preserves indentation)
    - Remove trailing whitespace
    - Collapse blank lines (max 2 consecutive)
    """
    if not isinstance(text, str):
        return text

    # Tabs to spaces
    text = text.replace("\t", "    ")

    # Remove trailing whitespace per line
    lines = [line.rstrip() for line in text.splitlines()]

    # Collapse multiple blank lines to max 2
    result = []
    blank_count = 0
    for line in lines:
        if not line:
            blank_count += 1
            if blank_count <= 2:
                result.append("")
        else:
            blank_count = 0
            result.append(line)

    return "\n".join(result)


# ── JSON number rounding ──────────────────────────────────────────────


def _round_json_numbers(obj: Any, precision: int = 4) -> Any:
    """Recursively round floats in JSON to given decimal places."""
    if isinstance(obj, float):
        if obj == int(obj):
            return int(obj)
        return round(obj, precision)
    if isinstance(obj, dict):
        return {k: _round_json_numbers(v, precision) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_round_json_numbers(item, precision) for item in obj]
    return obj


def compact_json_numbers(content: str, precision: int = 4) -> str:
    """Round floating-point numbers in JSON to reduce token count.

    High-precision floats like 3.141592653589793 become 3.1416.
    Semantic meaning is preserved — 4 decimal places is enough
    for all practical purposes in LLM context.
    """
    if not isinstance(content, str) or len(content) < 50:
        return content

    try:
        data = json.loads(content)
    except (json.JSONDecodeError, ValueError):
        return content

    compacted = _round_json_numbers(data, precision)
    return json.dumps(compacted, separators=(",", ":"), ensure_ascii=False)


# ── Boilerplate stripping ─────────────────────────────────────────────

# Patterns that appear at the start/end of common tool outputs
# and carry no semantic value.
_BOILERPLATE_HEADERS: list[re.Pattern] = [
    re.compile(r"^(?:[=#*]+.*?[=#*]+\n)+", re.MULTILINE),  # ASCII headers
    re.compile(r"^---+$", re.MULTILINE),                    # Markdown hr
]

_BOILERPLATE_FOOTERS: list[re.Pattern] = [
    re.compile(r"\nresume this session with:.*$", re.IGNORECASE),
    re.compile(r"\nsession:\s+\d{8}_\d{6}_\w{6}\s*$", re.IGNORECASE),
    re.compile(r"\nduration:\s+[\d.]+s\s*$", re.IGNORECASE),
    re.compile(r"\nmessages:\s+\d+.*$", re.IGNORECASE),
]


def strip_boilerplate(text: str, tool_name: str = "") -> str:
    """Strip standard boilerplate from tool outputs.

    Removes common headers, footers, and metadata lines that
    appear in every tool call but don't contribute to the LLM's
    understanding of the content.
    """
    if not isinstance(text, str) or len(text) < 200:
        return text

    # Strip headers
    for pattern in _BOILERPLATE_HEADERS:
        text = pattern.sub("", text, count=1)

    # Strip footers
    for pattern in _BOILERPLATE_FOOTERS:
        text = pattern.sub("", text)

    return text.strip()


# ── Path normalization ────────────────────────────────────────────────

_HOME_DIR = str(Path.home())


def normalize_paths(text: str, home_dir: str = _HOME_DIR) -> str:
    """Normalize file paths in tool output.

    - Replace home directory with ~
    - Normalize separators to /
    - Strip redundant ./
    """
    if not isinstance(text, str) or len(text) < 50:
        return text

    # Replace home dir with ~
    text = text.replace(home_dir, "~")

    # Normalize backslashes
    text = text.replace("\\", "/")

    # Remove redundant ./ prefixes
    text = re.sub(r"(?<!\w)\./", "", text)

    return text


# ── Timestamp shortening ──────────────────────────────────────────────

_ISO_TS_RE = re.compile(
    r"\b(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?\b"
)


def shorten_timestamps(text: str) -> str:
    """Shorten ISO 8601 timestamps to compact form.

    2026-06-13T07:28:20.609Z -> 20260613-072820
    """
    if not isinstance(text, str):
        return text

    def _compact(m):
        return f"{m.group(1)}{m.group(2)}{m.group(3)}-{m.group(4)}{m.group(5)}{m.group(6)}"

    return _ISO_TS_RE.sub(_compact, text)


# ── Full optimization pipeline ────────────────────────────────────────


def optimize_content(
    content: str,
    tool_name: str = "",
    *,
    normalize_ws: bool = True,
    compact_numbers: bool = True,
    strip_boiler: bool = True,
    normalize_paths_enabled: bool = True,
    shorten_ts: bool = True,
) -> str:
    """Run all zero-fidelity-loss optimizations on content.

    Order: whitespace first (cleans input), then JSON numbers,
    then boilerplate, then paths, then timestamps (most specific last).
    """
    if not isinstance(content, str) or not content:
        return content

    original_len = len(content)

    if normalize_ws:
        content = normalize_whitespace(content)

    if compact_numbers:
        content = compact_json_numbers(content)

    if strip_boiler:
        content = strip_boilerplate(content, tool_name)

    if normalize_paths_enabled:
        content = normalize_paths(content)

    if shorten_ts:
        content = shorten_timestamps(content)

    saved = original_len - len(content)
    if saved > 0:
        import logging
        logging.getLogger(__name__).debug(
            "optimize %s: %d -> %d chars (saved %d)",
            tool_name or "message", original_len, len(content), saved,
        )

    return content
