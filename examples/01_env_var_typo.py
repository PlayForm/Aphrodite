"""Atomic test 01 — APHRODITEINLINE_THRESHOLD env-var typo.

Bug:  _cfg_int reads "APHRODITEINLINE_THRESHOLD" (missing underscore),
      so the env var is silently ignored and 4096 is always used.
Fix:  key must be "APHRODITE_INLINE_THRESHOLD".

Run:  python examples/01_env_var_typo.py
Pass: prints OK
"""
import os

# ---------- minimal replica of the buggy helper ----------

def _cfg_int_buggy(key: str, default: int) -> int:
    """Original — wrong env-var name."""
    return int(os.environ.get("APHRODITEINLINE_THRESHOLD", default))

def _cfg_int_fixed(key: str, default: int) -> int:
    """Fixed — uses the key argument directly."""
    return int(os.environ.get(key, default))

# ---------- test ----------

os.environ["APHRODITE_INLINE_THRESHOLD"] = "1024"

buggy_value = _cfg_int_buggy("APHRODITE_INLINE_THRESHOLD", 4096)
fixed_value = _cfg_int_fixed("APHRODITE_INLINE_THRESHOLD", 4096)

assert buggy_value == 4096, f"Buggy path should ignore env var, got {buggy_value}"
assert fixed_value == 1024, f"Fixed path should read 1024, got {fixed_value}"

print("01 OK — env var typo detected and fixed")
print(f"  buggy read : {buggy_value}  (ignored env var, used default)")
print(f"  fixed read : {fixed_value}  (correctly read from environment)")

del os.environ["APHRODITE_INLINE_THRESHOLD"]
