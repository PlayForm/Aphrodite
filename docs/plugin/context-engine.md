# Context Engine

Origin: When the agent's conversation context fills beyond a threshold, the engine compresses middle messages into CCR markers, keeping only the head and tail raw. This avoids losing context entirely (like summarization-based compressors) while saving significant token budget.

Source of truth: `plugins/aphrodite/_engine.py` (lines 92-289), `plugins/aphrodite/plugin.yaml` (line 23)

## Activation

```yaml
# plugin.yaml
provides_context_engine: true
```

```bash
APHRODITE_CONTEXT_ENGINE=1
```

Set `context.engine: aphrodite` in Hermes config.yaml.

## Class

```python
class AphroditeContextEngine(ContextEngine):
    threshold_percent = ENGINE_THRESHOLD_PCT     # default 50
    protect_first_n = ENGINE_PROTECT_FIRST        # default 1
    protect_last_n = ENGINE_PROTECT_LAST          # default 1
    min_messages_to_compress = ENGINE_MIN_MSGS    # default 4
```

From `_engine.py:112`.

## Threshold Semantics

| Value | Behavior | Source |
|-------|----------|--------|
| -1 | Always compress (any context fill triggers) | _engine.py:141 |
| 0 | Disabled (never compress) | _engine.py:141 |
| >0 (e.g., 50) | Compress when prompt_tokens ≥ context_length × pct/100 | _engine.py:150 |

## Compress Algorithm (line 152)

```
compress(messages, current_tokens, focus_topic):
    1. If len(messages) ≤ min_messages_to_compress → return unchanged
    2. Determine head_n = max(protect_first_n, 1)
    3. Editing detection:
       a. Scan last 10 messages for tool role + editing keywords
          (wrote|patched|modified|created|deleted|successfully|written)
       b. If editing: tail_n = max(tail_n, 8)  -  protect active edits
    4. Clamp tail_n ≤ len(messages) - head_n
    5. Sweep orphan tool messages into tail:
       a. Forward scan from boundary: include tool messages
       b. Backward scan for owning assistant (has tool_calls)
    6. Split: head = [:head_n], middle = [head_n:-tail_n], tail = [-tail_n:]
    7. If len(middle) < 3 → return unchanged
    8. Pack middle messages → JSON (conditionally include tool_call_id, tool_calls)
    9. If packed < 200 bytes → return unchanged
   10. Try proxy compression (token preferred, cache fallback)
   11. If no proxy: inline compression fallback
   12. Store in inline store, append to recent_markers
   13. Build marker:
       <<<CCR:hash|context|size|engine>>>
       These messages were offloaded to reduce context.
       Retrieve with: aphrodite_retrieve(hash).
       The {protect_last_n} messages below are your active context.
   14. Return: head + [marker system message] + tail
   15. Fire hook: aphrodite_engine_compressed
```

## Message Packing

From `_pack_msg()` at line 74:
```python
def _pack_msg(messages):
    for m in messages:
        entry = {"role": role, "content": content}
        if tool_call_id and role == "tool": entry["tool_call_id"] = tool_call_id
        if tool_calls: entry["tool_calls"] = tool_calls
    return json.dumps(out, separators=(",", ":"))
```
Compact JSON  -  no whitespace.

## Editing Detection

`_EDITING_RE` at line 45:
```python
re.compile(r"\b(?:wrote|patched|modified|created|deleted|successfully|written)\b", re.IGNORECASE)
```

When editing is detected in the last 10 messages: `tail_n = max(tail_n, 8)`  -  protects more context to avoid losing the agent's editing momentum.

## Orphan Tool Message Sweep

Lines 170-185 handle the case where compressing middle messages would break tool_call → tool_result pairing:

1. Forward sweep: include trailing tool messages (orphan without their owning assistant)
2. Backward sweep: include the assistant that owns those tool messages (has `tool_calls`)
3. Re-clamp to prevent exceeding message count

## Mutual Exclusion

The context engine and `compression.enabled` SHOULD NOT both be active  -  the engine provides a different strategy (compress middle, keep head/tail) vs. per-tool compression (compress individual tool outputs). From plugin.yaml description.

## Hooks

The engine fires `aphrodite_engine_compressed` hook (if hermes_cli.plugins.invoke_hook available):

```python
_fire_hook("aphrodite_engine_compressed", engine=self, stats={
    "messages_compressed": middle_len,
    "packed_size": packed_len,
    "hash": hash_val,
    "count": self.compression_count,
})
```

Other plugins can listen and react (e.g., tracking compression frequency).

## Status

`get_status()` returns:
```python
{
    "last_prompt_tokens": int,
    "threshold_tokens": int,      # context_length * threshold_pct / 100
    "context_length": int,
    "usage_percent": float,       # min(100, tokens/context_length * 100)
    "compression_count": int,
}
```

## Session Lifecycle

### on_session_start
```python
def on_session_start(self, session_id="", **kw):
    self.session_id = session_id
```

### on_session_reset
Resets all state: tokens, compression count, inline store, conv_index, turn counter, file refs, markers, git cache.

## Integration Points

| Component | Integration |
|-----------|-------------|
| Proxy | Same /ccr/create endpoint for compression |
| Inline store | `_inline_store_put(hash, packed)` for fallback |
| Recent markers | Appends `{"hash", "type": "context", "size", "preview"}` |
| pre_llm_hook | Shows engine stats in catalog |
| aphrodite_stats | Returns engine status and stats |

## Default Configuration

| Setting | Default | Env Var |
|---------|---------|---------|
| Threshold % | 50 | APHRODITE_ENGINE_THRESHOLD_PCT |
| Protect first N | 1 | APHRODITE_ENGINE_PROTECT_FIRST |
| Protect last N | 1 | APHRODITE_ENGINE_PROTECT_LAST |
| Min messages | 4 | APHRODITE_ENGINE_MIN_MSGS |

With context_length=1,000,000 and threshold=50%: engine compresses when prompt_tokens ≥ 500,000.
