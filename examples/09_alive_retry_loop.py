"""Atomic test 09 — fixed 0.5 s sleep with no retry loop after proxy launch.

Bug:  on_start() does time.sleep(0.5) then calls _alive() once.  On a slow
      machine the proxy might not be up yet and the plugin logs "DOWN" even
      though the proxy starts a moment later.
Fix:  Replace the single sleep with a short retry loop.

Run:  python examples/09_alive_retry_loop.py
Pass: prints OK
"""
import time

# ---------- simulated proxy that becomes ready after 2 polls ----------

_poll_count = 0
READY_AFTER_POLLS = 3

def _alive_sim() -> bool:
    global _poll_count
    _poll_count += 1
    return _poll_count >= READY_AFTER_POLLS

# ---------- buggy on_start ----------

def on_start_buggy() -> bool:
    time.sleep(0)          # simulating 0.5 s — zero here for speed
    return _alive_sim()    # single attempt

# ---------- fixed on_start with retry ----------

def _wait_alive(retries: int = 10, delay: float = 0.0) -> bool:
    """delay=0.0 in test for speed; 0.3 in production."""
    for _ in range(retries):
        if _alive_sim():
            return True
        time.sleep(delay)
    return False

def on_start_fixed() -> bool:
    return _wait_alive(retries=10, delay=0.0)

# ---------- test ----------

_poll_count = 0
buggy_result = on_start_buggy()

_poll_count = 0
fixed_result = on_start_fixed()

assert buggy_result is False, "Buggy: single poll before proxy is ready should fail"
assert fixed_result is True,  "Fixed: retry loop should eventually succeed"

print("09 OK — retry loop replaces fixed single-sleep")
print(f"  buggy result (1 attempt)  : {buggy_result}")
print(f"  fixed result (retry loop) : {fixed_result}")
