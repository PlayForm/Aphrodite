# Engine Should Compress But Doesn't — Root Cause

## Symptom
- `context_engine: session started` appears in agent.log
- Engine shows as "active" in aphrodite_stats
- But `context_engine: compressed N msgs → CCR:hash` NEVER appears
- `should_compress` returns True when traced, but compress() never fires

## Root Cause Chain

1. **`should_compress(self, prompt_tokens=None)`** checks the PARAMETER `prompt_tokens` which defaults to None
2. Hermes may not pass `prompt_tokens` when calling `should_compress`
3. Hermes may not call `update_from_response(usage)` on the engine at all
4. `self.last_prompt_tokens` stays at 0 (initialized in __init__)
5. `tokens = prompt_tokens or self.last_prompt_tokens` = `None or 0` = 0
6. `not 0` is True → `should_compress` returns False

## Fix (v0.5.49 — TRIPLE FALLBACK)

```python
def should_compress(self, prompt_tokens=None):
    """0 = never compress (disabled). 50 = compress at 50% fill.
    NOTE: Falls back to context_length when Hermes doesn't call update_from_response."""
    if self.threshold_percent == 0:
        return False
    tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1000000)
    if not self.context_length:
        return False
    pct = (tokens / self.context_length) * 100
    return pct >= self.threshold_percent
```

**The critical line**: `tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1000000)`

Without the context_length fallback, the engine is silently dead when Hermes doesn't provide token counts.

## False Fix That Broke It

The fallback was removed in one commit as "dangerous" — but removing it made the engine silently stop compressing. This happened because the committer didn't understand that Hermes doesn't call `update_from_response`.

**Lesson**: When a fallback looks "dangerous" but is the only thing making a feature work, research WHY it was added before removing it. Check the commit history and surrounding code.
