"""Atomic test 08 — upstream API call on every /health request.

Bug:  health_check() in proxy.rs pings the upstream /models endpoint on
      every GET /health.  Since _alive() is called before every tool output,
      this burns rate-limit quota and adds latency on each Hermes turn.
Fix:  /health returns local proxy status only; upstream is checked at most
      once per minute via a cached result.

Run:  python examples/08_health_upstream.py
Pass: prints OK
"""
import time

# ---------- simulated upstream ping ----------

upstream_calls = 0

def _ping_upstream() -> bool:
    global upstream_calls
    upstream_calls += 1
    return True

# ---------- buggy handler: always pings upstream ----------

def health_handler_buggy() -> dict:
    up = _ping_upstream()   # BUG: called every time
    return {"status": "healthy", "upstream": up}

# ---------- fixed handler: TTL-cached upstream check ----------

_upstream_cache: dict = {"ok": False, "ts": 0.0}
UPSTREAM_TTL = 60.0  # seconds

def _upstream_ok_cached() -> bool:
    now = time.monotonic()
    if now - _upstream_cache["ts"] > UPSTREAM_TTL:
        _upstream_cache["ok"] = _ping_upstream()
        _upstream_cache["ts"] = now
    return _upstream_cache["ok"]

def health_handler_fixed() -> dict:
    return {"status": "healthy", "upstream": _upstream_ok_cached()}

# ---------- simulate 10 back-to-back health checks ----------

upstream_calls = 0
for _ in range(10):
    health_handler_buggy()
buggy_calls = upstream_calls

upstream_calls = 0
for _ in range(10):
    health_handler_fixed()
fixed_calls = upstream_calls

assert buggy_calls == 10, f"Buggy: expected 10 upstream pings, got {buggy_calls}"
assert fixed_calls == 1,  f"Fixed: expected 1 upstream ping (cache), got {fixed_calls}"

print("08 OK — upstream health-check TTL cache verified")
print(f"  buggy upstream calls for 10 health checks : {buggy_calls}")
print(f"  fixed upstream calls for 10 health checks : {fixed_calls}")
