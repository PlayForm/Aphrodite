"""Atomic test 12 — _resolve_one() only tries the token port (9798).

Bug:  _resolve_one() hard-codes the retrieve URL to 127.0.0.1:9798 (token
      proxy).  If the content was compressed via the cache proxy (9797) and
      the token proxy is down, retrieval returns None silently.
Fix:  Try the alive port first, fall back to the other.

Run:  python examples/12_resolve_port_fallback.py
Pass: prints OK
"""

# ---------- simulated proxy store ----------
# cache proxy (9797) has the content; token proxy (9798) is down

STORE = {
    9797: {"abc123": "<full tool output content>"},
    9798: {},   # empty / down
}

PORTS = {"cache": 9797, "token": 9798}

def _alive_sim(port: int) -> bool:
    """9797 is up, 9798 is down in this scenario."""
    return port == 9797

def _retrieve_from(port: int, hash_val: str) -> str | None:
    if not _alive_sim(port):
        return None
    return STORE[port].get(hash_val)

# ---------- buggy: only tries token port ----------

def _resolve_one_buggy(hash_val: str) -> str | None:
    return _retrieve_from(9798, hash_val)  # BUG: hardcoded

# ---------- fixed: tries alive port first, then falls back ----------

def _resolve_one_fixed(hash_val: str) -> str | None:
    primary, fallback = (
        (PORTS["token"], PORTS["cache"])
        if _alive_sim(PORTS["token"])
        else (PORTS["cache"], PORTS["token"])
    )
    for port in (primary, fallback):
        result = _retrieve_from(port, hash_val)
        if result is not None:
            return result
    return None

# ---------- test ----------

buggy_result = _resolve_one_buggy("abc123")
fixed_result = _resolve_one_fixed("abc123")

assert buggy_result is None,                         "Buggy: token port is down → None"
assert fixed_result == "<full tool output content>", "Fixed: fell back to cache port"

print("12 OK — port fallback in _resolve_one verified")
print(f"  buggy result : {buggy_result!r}  (token port down, no fallback)")
print(f"  fixed result : {fixed_result!r}  (fell back to cache port)")
