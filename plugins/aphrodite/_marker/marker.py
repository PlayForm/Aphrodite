"""CCR output block builder."""

import re

from .._core import _render_marker_tmpl

# Compiled regex for valid CCR hash validation
_VALID_HASH_RE = re.compile(r"^(?:[0-9a-f]{24,}|i:[0-9a-f]{6,})$")


def _is_valid_ccr_hash(h):
    """Check if h is a valid CCR hash.

    Fast-reject short strings before regex; the regex enforces >=24 hex chars
    for proxy hashes (>=6 hex for i: inline hashes).
    """
    if not h or len(h) < 8:
        return False
    return bool(_VALID_HASH_RE.match(h.lower()))


def _ccr_marker(hash_val, ccr_type, size, mode="", preview="", headroom_budget=None, meta=None, center=None):
    """Build a CCR output block using TOML-driven template.

    Delegates to _render_marker_tmpl() which reads [templates.marker] from
    aphrodite.toml. Falls back to hardcoded three-line format if TOML missing.
    Headroom budget truncates preview under tight budgets.
    """
    # Headroom budget: truncate preview for tight budgets
    safe = preview.replace("|", "-").replace("\n", " ").replace("\r", " ").strip()
    safe = "".join(c if c >= " " else " " for c in safe)
    if headroom_budget is not None:
        try:
            budget = int(headroom_budget)
            if budget < 25:
                safe = safe[:30]
            elif budget < 50:
                safe = safe[:60]
            elif budget < 75:
                safe = safe[:100]
        except (ValueError, TypeError):
            pass

    # Build metadata string
    meta_parts = []
    if meta:
        for k, v in meta.items():
            safe_v = str(v).replace("|", "/").replace("\n", " ").strip()
            if safe_v:
                meta_parts.append(f"{k}={safe_v}")
    meta_str = ";".join(meta_parts)
    if len(meta_str) > 300:
        meta_str = meta_str[:297] + "..."

    return _render_marker_tmpl(safe, ccr_type, meta_str, center, hash_val, size)
