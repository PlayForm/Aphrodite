"""
Parser mapping - which headroom compressor handles each content type.

Mirrors the TypeScript Variable/Parser.ts pattern.
"""

from __future__ import annotations

from typing import Dict, List, Union


# Content type → headroom compressor mapping.
# Headroom's ContentRouter auto-detects the content type and routes
# to the appropriate compressor. This mapping is for documentation
# and tool-type routing hints.
Parser: Dict[str, Union[str, List[str]]] = {
    "JSON": "SmartCrusher",       # JSON arrays, structured data
    "Code": "CodeCompressor",     # Source code (6 languages AST-aware)
    "Prose": "Kompress",          # ML-based text compression
    "Log": "Kompress",            # Log output → routed as prose
    "HTML": "SmartCrusher",       # HTML snapshots → structured
    "Image": "skip",              # Not compressible as text
    "Cache": "CacheAligner",      # Prefix stabilization for KV cache
    "Mixed": [                    # Auto-detect → routes to best compressor
        "SmartCrusher",
        "CodeCompressor",
        "Kompress",
        "CacheAligner",
    ],
}


# Content type routing priority.
# When ContentRouter detects multiple possible types, use this order.
RoutePriority: List[str] = [
    "CacheAligner",    # Always first - stabilizes prefixes
    "ContentRouter",   # Auto-detect content type
    "SmartCrusher",    # JSON / structured data
    "CodeCompressor",  # Source code
    "Kompress",        # Prose / text fallback
]
