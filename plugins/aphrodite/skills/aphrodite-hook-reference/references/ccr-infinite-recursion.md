# CCR Compression: Skip List and Infinite Recursion

Discovered 2026-06-15. Critical pitfall for any Hermes plugin that provides CCR retrieval tools.

## The Problem

If a plugin provides CCR retrieval tools (like `headroom_retrieve`, `headroom_stats`) and those tools produce output >1KB (token mode threshold), `_transform_tool_result` will compress them AGAIN into another CCR marker. This creates INFINITE RECURSION:

```
1. Tool output (15KB) → _transform_tool_result → [CCR:hash|tool|15922]
2. Agent calls headroom_retrieve(hash)
3. headroom_retrieve returns 15KB raw content
4. _transform_tool_result compresses IT → [CCR:hash|tool|15922] again
5. Agent sees ANOTHER CCR marker, can never read actual content
```

## The Fix

Add retrieval tools to the skip list in `_transform_tool_result`:

```python
# BEFORE (broken):
skip = {"read_file", "read_terminal"}

# AFTER (fixed):
skip = {"read_file", "read_terminal", "headroom_retrieve", "headroom_stats"}
```

Both token and cache mode skip lists must include retrieval tools.

## Detection

If you see `headroom_retrieve` returning a CCR marker instead of actual content, the output is being re-compressed. Check:
1. Is the tool name in the skip list?
2. Is the output size actually below threshold? (shouldn't matter with skip)

## Related

- `_transform_tool_result` in plugin: compresses tool outputs >1KB (token) or >8KB (cache)
- `headroom_retrieve` tool: resolves CCR markers to original content
- Skip list applies BEFORE threshold check — skip items are never compressed regardless of size
