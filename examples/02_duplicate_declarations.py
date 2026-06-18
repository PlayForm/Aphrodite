"""Atomic test 02 - duplicate module-level declarations shadow configured values.

Bug:  _inline_store = {} and INLINE_THRESHOLD = 4096 appear twice at module
      level.  The second bare assignment overwrites whatever _cfg_int returned.
Fix:  Remove the duplicate block; keep only the _cfg_int call.

Run:  python examples/02_duplicate_declarations.py
Pass: prints OK
"""
import os

os.environ["APHRODITE_INLINE_THRESHOLD"] = "512"

# ---------- simulates the buggy module-level sequence ----------

def _cfg_int(key: str, default: int) -> int:
    return int(os.environ.get(key, default))

# first declaration - correct
_inline_store: dict = {}
INLINE_THRESHOLD = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)

# ... many lines later, the duplicate ...
_inline_store = {}          # BUG: resets the dict (harmless here but confusing)
INLINE_THRESHOLD = 4096     # BUG: shadows the configured value

buggy_threshold = INLINE_THRESHOLD  # will be 4096

# ---------- fixed module-level sequence ----------

_inline_store_f: dict = {}
INLINE_THRESHOLD_F = _cfg_int("APHRODITE_INLINE_THRESHOLD", 4096)
# (no second assignment)

fixed_threshold = INLINE_THRESHOLD_F  # will be 512

assert buggy_threshold == 4096, f"Expected 4096, got {buggy_threshold}"
assert fixed_threshold == 512, f"Expected 512, got {fixed_threshold}"

print("02 OK - duplicate declaration shadowing detected")
print(f"  buggy INLINE_THRESHOLD : {buggy_threshold}  (hardcoded, env var lost)")
print(f"  fixed INLINE_THRESHOLD : {fixed_threshold}  (from environment)")

del os.environ["APHRODITE_INLINE_THRESHOLD"]
