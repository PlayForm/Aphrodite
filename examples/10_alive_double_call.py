"""Atomic test 10 - redundant _alive() calls per Hermes turn.

Bug:  _transform_tool_result calls _alive(port) for both ports, then
      _pre_llm_hook calls _alive() again before each LLM request.  With a
      3 s timeout × 2 ports × 2 call sites = up to 12 s overhead per turn
      when proxies are down.
Fix:  Cache the alive status with a short TTL (5 s).

Run:  python examples/10_alive_double_call.py
Pass: prints OK
"""
import time

# ---------- simulated network probe ----------

probe_count = 0

def _probe_port(port: int) -> bool:
    global probe_count
    probe_count += 1
    return True

# ---------- buggy: no cache, every call probes the network ----------

def _alive_buggy(port: int) -> bool:
    return _probe_port(port)

def simulate_turn_buggy():
    # _transform_tool_result
    _alive_buggy(9797)
    _alive_buggy(9798)
    # _pre_llm_hook
    _alive_buggy(9797)
    _alive_buggy(9798)

# ---------- fixed: TTL cache ----------

_cache: dict[int, tuple[bool, float]] = {}
TTL = 5.0

def _alive_fixed(port: int) -> bool:
    now = time.monotonic()
    if port in _cache:
        result, ts = _cache[port]
        if now - ts < TTL:
            return result
    result = _probe_port(port)
    _cache[port] = (result, now)
    return result

def simulate_turn_fixed():
    _alive_fixed(9797)
    _alive_fixed(9798)
    _alive_fixed(9797)
    _alive_fixed(9798)

# ---------- test ----------

probe_count = 0
simulate_turn_buggy()
buggy_probes = probe_count

probe_count = 0
_cache.clear()
simulate_turn_fixed()
fixed_probes = probe_count

assert buggy_probes == 4, f"Buggy: expected 4 probes per turn, got {buggy_probes}"
assert fixed_probes == 2, f"Fixed: expected 2 probes (one per port), got {fixed_probes}"

print("10 OK - TTL cache halves network probes per turn")
print(f"  buggy probes / turn : {buggy_probes}")
print(f"  fixed probes / turn : {fixed_probes}")
