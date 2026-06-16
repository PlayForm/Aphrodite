# Tool-Chain Safe Compression

When compressing conversation messages, the boundary between "kept" (tail) and "compressed" (middle) must not split a tool call chain. Splitting `assistant[tool_call]` from its `tool[result]` breaks the LLM's ability to understand the chain.

## The Problem

With `protect_last_n=5`, the last 5 messages are kept raw. If message 95 is an assistant tool_call and message 96 is the tool_result, compressing at that boundary produces:

```
Head (2 msgs) + Middle (msgs 3-95, compressed) + Tail (msgs 96-100)
```

The tail starts with a tool_result that has no corresponding tool_call — the LLM can't understand it.

## The Fix

After determining the boundary, scan forward for tool messages and extend the tail:

```python
tail_n = self.protect_last_n
if len(messages) > tail_n:
    boundary = len(messages) - tail_n
    # If boundary splits a tool chain, extend tail
    while boundary < len(messages) and messages[boundary].get("role") == "tool":
        boundary += 1
        tail_n += 1
    tail_n = min(tail_n, len(messages) - head_n)

head = messages[:head_n]
middle = messages[head_n:-tail_n]
tail = messages[-tail_n:]
```

This ensures tool results stay with their tool calls in the kept tail.
