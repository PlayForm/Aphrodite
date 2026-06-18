"""Atomic test 13 - engine compress() truncates content to 2000 chars.

Bug:  The engine's compress() packs messages as str(content)[:2000].  A
      50 KB tool output is already CCR-marked (e.g. ⫷CCR:abc123|tool|51200⫸)
      before reaching compress().  Slicing to 2000 chars can cut right through
      the marker, breaking the hash and making retrieval impossible.
Fix:  Skip the [:2000] truncation for strings that contain a CCR marker.

Run:  python examples/13_engine_truncation.py
Pass: prints OK
"""
import re

MARKER_RE = re.compile(r"\u2AB7CCR:[^\u2AB8]+\u2AB8")

# ---------- a realistic already-compressed message ----------

marker  = "\u2AB7CCR:deadbeef01234567|tool|51200\u2AB8"
assert len(marker) < 50, "sanity - markers are short"

# Imagine the message content is large and already replaced by the marker
message_content = marker  # in practice this is what _transform_tool_result leaves

# ---------- buggy pack ----------

def pack_buggy(content: str, limit: int = 2000) -> str:
    return str(content)[:limit]   # BUG: safe only if content has no markers

# ---------- fixed pack ----------

def pack_fixed(content: str, limit: int = 2000) -> str:
    if MARKER_RE.search(content):
        return content             # FIX: never truncate a CCR-marked string
    return str(content)[:limit]

# ---------- demonstrate breakage with an artificially short limit ----------

SHORT_LIMIT = 20  # simulates slicing a long marker

buggy_packed = pack_buggy(message_content, limit=SHORT_LIMIT)
fixed_packed = pack_fixed(message_content, limit=SHORT_LIMIT)

buggy_valid = bool(MARKER_RE.search(buggy_packed))
fixed_valid = bool(MARKER_RE.search(fixed_packed))

assert not buggy_valid, "Buggy: truncated marker cannot be parsed by retrieval"
assert fixed_valid,     "Fixed: full marker preserved"

print("13 OK - CCR marker truncation bug caught")
print(f"  original marker   : {message_content!r}")
print(f"  buggy (limit={SHORT_LIMIT:4d}) : {buggy_packed!r}  valid={buggy_valid}")
print(f"  fixed (limit={SHORT_LIMIT:4d}) : {fixed_packed!r}  valid={fixed_valid}")
