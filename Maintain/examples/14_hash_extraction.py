"""Atomic test 14 - pipe-suffix not stripped before hash lookup.

Bug:  The headroom_retrieve tool receives args["hash"] from the LLM.  The LLM
      sometimes copies the full marker token including the pipe-delimited
      suffix, e.g. "abc123|tool|1024".  The store key is only the bare hash
      "abc123", so the lookup misses.
Fix:  Strip everything from the first '|' before the store lookup.

Run:  python examples/14_hash_extraction.py
Pass: prints OK
"""

# ---------- simulated store ----------

STORE: dict[str, str] = {
    "abc123": "<the real content>",
}

# ---------- buggy retrieve ----------


def retrieve_buggy(hash_arg: str) -> str | None:
    return STORE.get(hash_arg)  # BUG: passes raw arg, may include '|type|size'


# ---------- fixed retrieve ----------


def retrieve_fixed(hash_arg: str) -> str | None:
    clean = hash_arg.split("|")[0].strip()  # FIX: strip suffix before lookup
    return STORE.get(clean)


# ---------- test cases ----------

BARE_HASH = "abc123"
SUFFIXED = "abc123|tool|1024"
WHITESPACED = "  abc123  "

assert retrieve_buggy(BARE_HASH) == "<the real content>", "bare hash OK in buggy"
assert retrieve_buggy(SUFFIXED) is None, "BUG: suffix causes miss"

assert retrieve_fixed(BARE_HASH) == "<the real content>", "bare hash OK in fixed"
assert retrieve_fixed(SUFFIXED) == "<the real content>", "suffix stripped"
assert retrieve_fixed(WHITESPACED) == "<the real content>", "whitespace stripped"

print("14 OK - pipe-suffix stripping verified")
print(f"  buggy('{SUFFIXED}') -> {retrieve_buggy(SUFFIXED)!r}  (miss)")
print(f"  fixed('{SUFFIXED}') -> {retrieve_fixed(SUFFIXED)!r}  (hit)")
