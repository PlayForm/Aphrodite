"""
Directory utility - resolves and normalizes paths.

Mirrors the TypeScript Function/Directory.ts pattern.
"""

from __future__ import annotations

from pathlib import Path


async def Directory(path: str) -> str:
    """Normalize a file path to its directory with trailing slash.

    Port of the TypeScript Directory function.
    """
    p = Path(path).resolve()
    d = str(p.parent).replace("\\", "/")
    if not d.endswith("/"):
        d += "/"
    return d


def normalize_path(path: str) -> str:
    """Normalize a path for cross-platform consistency."""
    return str(Path(path)).replace("\\", "/")
