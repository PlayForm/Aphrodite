"""Atomic test 07 — tokens_saved counter never incremented.

Bug:  AppState has tokens_saved: AtomicU64 exposed in /stats, but nowhere
      in the compression path is .fetch_add() called — it always returns 0.
Fix:  After each compression, add (original_len - marker_len) to the counter.
      This file simulates the logic in Python to show the fix is correct.

Run:  python examples/07_tokens_saved.py
Pass: prints OK + accumulated savings
"""
from threading import Lock

# ---------- simulated AtomicU64 in Python ----------

class AtomicU64:
    def __init__(self) -> None:
        self._val = 0
        self._lock = Lock()

    def fetch_add(self, n: int) -> int:
        with self._lock:
            old = self._val
            self._val += n
            return old

    def load(self) -> int:
        with self._lock:
            return self._val

# ---------- buggy compress: never calls fetch_add ----------

tokens_saved_buggy = AtomicU64()

def compress_buggy(content: str, marker: str) -> str:
    # ... compression happens ...
    # BUG: tokens_saved.fetch_add(len(content) - len(marker))  <- missing
    return marker

# ---------- fixed compress ----------

tokens_saved_fixed = AtomicU64()

def compress_fixed(content: str, marker: str) -> str:
    result = marker
    saved = max(0, len(content) - len(result))
    tokens_saved_fixed.fetch_add(saved)          # FIX
    return result

# ---------- simulate three compressions ----------

MARKER = "\u2AB7CCR:abc123|tool|4096\u2AB8"  # 25 chars

contents = [
    "A" * 4096,
    "B" * 8192,
    "C" * 2048,
]

for c in contents:
    compress_buggy(c, MARKER)
    compress_fixed(c, MARKER)

assert tokens_saved_buggy.load() == 0, "Buggy counter must remain 0"
expected = sum(max(0, len(c) - len(MARKER)) for c in contents)
assert tokens_saved_fixed.load() == expected, \
    f"Fixed counter should be {expected}, got {tokens_saved_fixed.load()}"

print("07 OK — tokens_saved counter fix verified")
print(f"  buggy counter : {tokens_saved_buggy.load()}  (always 0)")
print(f"  fixed counter : {tokens_saved_fixed.load()}  (total bytes saved)")
