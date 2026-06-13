"""
Deep merge utility - recursively merges two dictionaries.

Mirrors the TypeScript Function/Merge.ts pattern.
The TypeScript version uses @playform/pipe's merge; this is a pure-Python port.
"""

from __future__ import annotations

from typing import Any


def Merge(base: dict[str, Any], overlay: dict[str, Any]) -> dict[str, Any]:
    """Deep merge overlay into base. Overlay values win on conflict.

    Recursively merges nested dicts; lists and scalars are replaced,
    not merged.
    """
    result = {**base}
    for key, value in overlay.items():
        if (
            key in result
            and isinstance(result[key], dict)
            and isinstance(value, dict)
        ):
            result[key] = Merge(result[key], value)
        else:
            result[key] = value
    return result
