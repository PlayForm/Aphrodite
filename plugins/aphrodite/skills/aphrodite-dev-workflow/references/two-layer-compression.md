# Two-Layer Compression Architecture

Aphrodite compresses at two independent layers. Understanding the distinction
is essential for debugging why content appears/disappears in context.

## Layer 1: Tool Output CCR (post_tool_call hook)

Fires on EVERY tool call. Automatic, always-on (unless APHRODITE_PASSTHROUGH=1).

Flow:
```
Tool executes → returns N bytes
_transform_tool_result hook fires
  → size < 1024 bytes? PASS THROUGH (no compression)
  → tool in skip list? PASS THROUGH
    Skip list (token proxy alive):
      read_file, read_terminal, aphrodite_retrieve, aphrodite_compress,
      aphrodite_stats
    Skip list (token proxy dead):
      adds execute_code, memory, patch, write_file, search_files, todo
  → content already has <<<CCR:...>>> markers? PASS THROUGH (re-compression guard)
  → POST to proxy :9798/ccr/create → stores compressed content → returns hash
  → fallback: zlib inline if proxy unreachable (>4KB threshold)
  → Result REPLACED with: <<<CCR:hash|tool|size|mode>>> {preview}
Hermes receives marker string as tool_result
Wraps it in a tool_result message
Appends to conversation messages array
Sends to API
```

**The model NEVER sees raw tool output >1KB.** It only sees the CCR marker +
120-char preview. When I call aphrodite_retrieve(), a new tool_call+tool_result
pair enters the messages array - the content is reconstructed but adds 2
messages to the conversation.

## Layer 2: Conversation Compression (context engine)

Fires ONLY when prompt tokens exceed threshold_percent of context_length.
Default: 50% fill. DEFAULT-ON — TOML toggle: [compression].context_engine = true

```
Hermes calls compress(messages) before each LLM turn
should_compress(prompt_tokens):
  → threshold_percent == 0? False (DISABLED)
  → no prompt_tokens or context_length? False
  → (prompt_tokens / context_length) * 100 >= 50? True

compress() logic:
  → messages <= min_messages_to_compress (30)? RETURN AS-IS
  → head = messages[:protect_first_n]     (2 messages - system prompt + first user)
  → tail = messages[-protect_last_n:]      (5 messages - active context)
  → editing session detected? tail → min 8 messages
  → tool-chain safety: if boundary splits tool_call→tool_result, backtrack
  → middle = messages[head_n:-tail_n]
  → middle < 3 messages? RETURN AS-IS
  → JSON-pack middle → POST to proxy /ccr/create → hash
  → Replace middle with single system message:
    [CONTEXT COMPRESSED: N messages → CCR:hash|size]
    Retrieve with: aphrodite_retrieve(hash).
    The 5 messages below are your active context.
```

## What The Model Sees vs What The User Sees

Example turn with 3 tool calls:

```
User's terminal shows:
  ┌─ aphrodite_stats ──────────────────────────
  │ {"proxy": {"cache": {"alive": true, ...}, "token": {"alive": true, ...}}}
  └─ [APHRODITE] CCR:abc123|tool|763|token │ {"proxy": {"cache": {"alive":...

Model's messages array:
  [N]   role: assistant, tool_calls: [aphrodite_diff, aphrodite_files, aphrodite_stats]
  [N+1] role: tool, name: aphrodite_diff, content: "<<<CCR:xxx|tool|102>>> {"turns": 1..."
  [N+2] role: tool, name: aphrodite_files, content: "<<<CCR:yyy|tool|73>>> {"files": []..."
  [N+3] role: tool, name: aphrodite_stats, content: "<<<CCR:zzz|tool|763|token>>> {"proxy":...
```

The model gets the CCR marker + preview. The full 50KB tool output was stored
in the proxy and NEVER entered the model's context window.

When I retrieve it: aphrodite_retrieve("xxx") → new tool_call/tool_result →
full content reconstructed → I can reason about it → produce final response.

## Proxy Stores

| Port | Name | Backend | Threshold | Purpose |
|------|------|---------|-----------|---------|
| 9797 | cache | In-memory | >8KB | Large outputs, ephemeral |
| 9798 | token | SQLite | >1KB | General tool output, persistent |

Both support /ccr/create (POST, JSON {content}) and /retrieve (POST, JSON {hash}).

## Inline Store (Session Dict)

Python dict in plugin process. Mirrors all compress+retrieve operations.
Used by aphrodite_search. Content-addressable: SHA256-based key, cache-first
before proxy call. Cleared on /reset.

## Detection at a Glance

If you see `<<<CCR:hash|tool|size>>>` - that's Layer 1 (tool output compressed).
If you see `[CONTEXT COMPRESSED: N messages → CCR:hash|size]` - that's Layer 2.
If you see raw tool output - it was <1KB or in the skip list.
If you see `[APHRODITE] CCR:hash|type|size | {"preview..."` - that's the user banner,
not what the model receives.
