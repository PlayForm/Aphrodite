"""Atomic test 04 - hardcoded absolute path in _rebuild_handler.

Bug:  repo = "/Users/username/Projects/HermesCompress"
      This breaks on any machine that is not the developer's machine.
Fix:  Derive the repo root from __file__ so it always points to the
      checked-out workspace regardless of where it lives.

Run:  python examples/04_hardcoded_path.py
Pass: prints OK + the computed path
"""
import os

# ---------- buggy version ----------

def _rebuild_handler_buggy() -> str:
    repo = "/Users/username/Projects/HermesCompress"
    return repo

# ---------- fixed version ----------
# __file__ = .../aphrodite/examples/04_hardcoded_path.py
# two dirname() calls  → .../aphrodite  (workspace root)

def _rebuild_handler_fixed() -> str:
    here = os.path.abspath(__file__)                   # this file
    repo = os.path.dirname(os.path.dirname(here))      # workspace root
    return repo

# ---------- assertions ----------

buggy_path = _rebuild_handler_buggy()
fixed_path = _rebuild_handler_fixed()

assert buggy_path.startswith("/Users/"), "Should be a hardcoded user path"
assert os.path.isdir(fixed_path), f"Fixed repo root must exist: {fixed_path}"
assert not fixed_path.startswith("/Users/username"), "Should not contain personal path"

print("04 OK - hardcoded path replaced with __file__-relative resolution")
print(f"  buggy path : {buggy_path}")
print(f"  fixed path : {fixed_path}")
