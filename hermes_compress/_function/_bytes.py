"""
Bytes formatter - human-readable byte sizes.

Port of @playform/pipe's Bytes function for Python.
"""

from __future__ import annotations


def Bytes(size: int) -> str:
    """Format a byte count as human-readable string.

    Examples:
        500 → "500 B"
        1500 → "1.46 KB"
        1500000 → "1.43 MB"
    """
    if size < 0:
        return "0 B"

    units = ["B", "KB", "MB", "GB", "TB"]
    unit_index = 0
    value = float(size)

    while value >= 1024 and unit_index < len(units) - 1:
        value /= 1024
        unit_index += 1

    if unit_index == 0:
        return f"{int(value)} {units[unit_index]}"
    return f"{value:.2f} {units[unit_index]}"
