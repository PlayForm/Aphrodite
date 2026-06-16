"""Atomic test 15 — silent failure when binary is missing.

Bug:  _ensure_binary() / register() logs an error but continues silently
      when neither the downloaded binary nor the cargo-built binary is found.
      The plugin appears loaded but all proxy-dependent tools are unavailable
      with no user-visible indication in the Hermes UI.
Fix:  Raise RuntimeError (or return a clear error string from register()) so
      Hermes can surface the problem to the user immediately.

Run:  python examples/15_binary_launch_warn.py
Pass: prints OK
"""
import os

# ---------- buggy register ----------

def register_buggy(binary_path: str) -> dict:
    if not os.path.isfile(binary_path):
        # BUG: logs silently, returns partial registration
        print(f"  [buggy] WARNING: binary not found at {binary_path} — continuing")
        return {"tools": []}     # empty tools, no error surfaced
    return {"tools": ["headroom_retrieve", "headroom_store"]}

# ---------- fixed register ----------

def register_fixed(binary_path: str) -> dict:
    if not os.path.isfile(binary_path):
        raise RuntimeError(
            f"Aphrodite binary not found at {binary_path!r}.\n"
            "Run: cargo build -p aphrodite --release\n"
            "  or set APHRODITE_BIN_PATH to the correct location."
        )
    return {"tools": ["headroom_retrieve", "headroom_store"]}

# ---------- test ----------

FAKE_PATH = "/nonexistent/aphrodite"

buggy_result = register_buggy(FAKE_PATH)
assert buggy_result["tools"] == [], "Buggy: returns empty tools silently"

error_raised = False
try:
    register_fixed(FAKE_PATH)
except RuntimeError as exc:
    error_raised = True
    assert "cargo build" in str(exc), "Error should include build instructions"

assert error_raised, "Fixed: must raise RuntimeError when binary is missing"

print("15 OK — missing binary surfaces RuntimeError instead of silent skip")
print(f"  buggy: tools={buggy_result['tools']!r}  (empty, no error)")
print("  fixed: RuntimeError raised with actionable message")
