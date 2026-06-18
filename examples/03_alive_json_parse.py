"""Atomic test 03 - health-check body comparison: missing space in JSON key.

Bug:  _alive() checks  '"status":"healthy"'  (no space after colon).
      serde_json serialises  '"status": "healthy"'  (space after colon).
      The substring check therefore ALWAYS fails for the JSON branch.
Fix:  Parse the body as JSON and check data.get("status").

Run:  python examples/03_alive_json_parse.py
Pass: prints OK
"""
import json

# ---------- proxy responses we need to handle ----------

RESPONSE_PLAIN   = "ok"                                      # legacy plain-text
RESPONSE_JSON    = '{"status": "healthy", "version": "0.2.0"}'  # serde_json (space)
RESPONSE_COMPACT = '{"status":"healthy"}'                    # compact (no space)
RESPONSE_BAD     = '{"status": "starting"}'                  # not-ready yet

# ---------- buggy implementation ----------

def _alive_buggy(body: str) -> bool:
    return body == "ok" or '"status":"healthy"' in body  # misses the space

# ---------- fixed implementation ----------

def _alive_fixed(body: str) -> bool:
    if body.strip() == "ok":
        return True
    try:
        data = json.loads(body)
        return data.get("status") in ("healthy", "ok", "degraded")
    except Exception:
        return False

# ---------- assertions ----------

assert _alive_buggy(RESPONSE_PLAIN)   is True,  "plain 'ok' should be truthy"
assert _alive_buggy(RESPONSE_JSON)    is False, "BUG: serde_json space causes miss"
assert _alive_buggy(RESPONSE_COMPACT) is True,  "compact (no space) happens to match"

assert _alive_fixed(RESPONSE_PLAIN)   is True,  "fixed: plain 'ok'"
assert _alive_fixed(RESPONSE_JSON)    is True,  "fixed: serde_json with space"
assert _alive_fixed(RESPONSE_COMPACT) is True,  "fixed: compact JSON"
assert _alive_fixed(RESPONSE_BAD)     is False, "fixed: starting is not healthy"

print("03 OK - JSON health-check space bug demonstrated and fixed")
print(f"  buggy(serde_json body)  : {_alive_buggy(RESPONSE_JSON)}   <- FALSE NEGATIVE")
print(f"  fixed(serde_json body)  : {_alive_fixed(RESPONSE_JSON)}   <- correct")
