"""Atomic test 11 - should_compress() ignores threshold_percent.

Bug:  AphroditeContextEngine.should_compress() always returns True regardless
      of prompt_tokens or ENGINE_THRESHOLD_PCT.  The env var is defined but
      never actually consulted.
Fix:  Compare (prompt_tokens / context_length) * 100 against threshold_pct.

Run:  python examples/11_should_compress.py
Pass: prints OK
"""

# ---------- buggy implementation ----------


class ContextEngineBuggy:
    def __init__(self, threshold_pct: float = 75.0, context_length: int = 128_000):
        self.threshold_percent = threshold_pct
        self.context_length = context_length

    def should_compress(self, prompt_tokens: int | None = None) -> bool:
        return True  # BUG: always True


# ---------- fixed implementation ----------


class ContextEngineFixed:
    def __init__(self, threshold_pct: float = 75.0, context_length: int = 128_000):
        self.threshold_percent = threshold_pct
        self.context_length = context_length

    def should_compress(self, prompt_tokens: int | None = None) -> bool:
        if self.threshold_percent == 0:
            return True
        if not prompt_tokens or not self.context_length:
            return True
        pct = (prompt_tokens / self.context_length) * 100
        return pct >= self.threshold_percent


# ---------- test cases ----------

engine_b = ContextEngineBuggy(threshold_pct=75.0, context_length=128_000)
engine_f = ContextEngineFixed(threshold_pct=75.0, context_length=128_000)

# 10% fill - should NOT compress
assert engine_b.should_compress(12_800) is True, "Buggy always True"
assert engine_f.should_compress(12_800) is False, "Fixed: 10% < 75%, skip compress"

# 80% fill - should compress
assert engine_b.should_compress(102_400) is True, "Buggy always True"
assert engine_f.should_compress(102_400) is True, "Fixed: 80% >= 75%, do compress"

# None tokens - safe default
assert engine_f.should_compress(None) is True, "Fixed: unknown fill → compress"

print("11 OK - threshold_percent now gates compression")
print("  at 10% fill  → buggy=True  fixed=False")
print("  at 80% fill  → buggy=True  fixed=True")
print("  at None fill → buggy=True  fixed=True (safe default)")
