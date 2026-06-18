"""Atomic test 06 - Unicode marker glyph mismatch in injected tool description.

Bug:  The headroom_retrieve tool description uses ⭷ (U+2B77) / ⭸ (U+2B78)
      but smart_marker() and all compression code use ⫷ (U+2AB7) / ⫸ (U+2AB8).
      The LLM reads the description and looks for the wrong brackets, making
      intent recognition unreliable.
Fix:  Use the same glyph pair everywhere.

Run:  python examples/06_marker_glyph.py
Pass: prints OK
"""
import re

# ---------- the two glyph pairs ----------

GLYPH_WRONG_OPEN  = "\u2B77"   # ⭷  - used in description (BUG)
GLYPH_WRONG_CLOSE = "\u2B78"   # ⭸
GLYPH_RIGHT_OPEN  = "\u2AB7"   # ⫷  - used everywhere else
GLYPH_RIGHT_CLOSE = "\u2AB8"   # ⫸

# ---------- replica of smart_marker ----------

def smart_marker(hash_val: str, kind: str = "tool", size: int = 0) -> str:
    return f"{GLYPH_RIGHT_OPEN}CCR:{hash_val}|{kind}|{size}{GLYPH_RIGHT_CLOSE}"

# ---------- buggy description ----------

DESCRIPTION_BUGGY = (
    f"Call this tool when you see a {GLYPH_WRONG_OPEN}CCR:hash{GLYPH_WRONG_CLOSE} "
    "marker in the conversation."
)

# ---------- fixed description ----------

DESCRIPTION_FIXED = (
    f"Call this tool when you see a "
    f"{GLYPH_RIGHT_OPEN}CCR:hash|type|size{GLYPH_RIGHT_CLOSE} "
    "marker in the conversation."
)

# ---------- check: does a real marker match each description's glyph? ----------

sample_marker = smart_marker("abc123", "tool", 1024)

def glyph_in_description(desc: str, marker: str) -> bool:
    """Extract the open glyph from the description and check it's in the marker."""
    # find the first non-ASCII character in the description
    for ch in desc:
        if ord(ch) > 127:
            return ch in marker
    return False

assert not glyph_in_description(DESCRIPTION_BUGGY, sample_marker), \
    "Wrong glyph should NOT appear in a real marker"
assert glyph_in_description(DESCRIPTION_FIXED, sample_marker), \
    "Correct glyph MUST appear in a real marker"

print("06 OK - Unicode glyph mismatch caught")
print(f"  wrong glyphs : {GLYPH_WRONG_OPEN!r} {GLYPH_WRONG_CLOSE!r}  (U+2B77/78)")
print(f"  right glyphs : {GLYPH_RIGHT_OPEN!r} {GLYPH_RIGHT_CLOSE!r}  (U+2AB7/8)")
print(f"  sample marker: {sample_marker}")
