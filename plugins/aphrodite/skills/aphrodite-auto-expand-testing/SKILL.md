---
name: aphrodite-auto-expand-testing
description: "Protocol for testing APHRODITE_NO_AUTO_EXPAND behavior — verifying raw CCR markers appear instead of auto-resolved content. Covers proxy-vs-engine distinction, terminal output routing, and retrieval verification."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-boundary-behaviors, aphrodite-dev-workflow]
---

# Aphrodite Auto-Expand Testing

Protocol for verifying `APHRODITE_NO_AUTO_EXPAND=1` is working: that `<<<CCR:...>>>` markers appear **raw** in the LLM's context instead of being automatically resolved.

**Note**: This skill lives under the `default` Hermes profile. The canonical umbrella `aphrodite-boundary-behaviors` (under `aphrodite-compress-aggressive`) covers the same territory and already references a `references/testing-auto-expand.md` file that has not yet been created there. If you have access to the `aphrodite-compress-aggressive` profile, update that skill's references instead.

## When to Use

- Debugging why content appears expanded when `APHRODITE_NO_AUTO_EXPAND=1` is set
- Verifying a new proxy build correctly compresses terminal output
- Distinguishing proxy compression from engine auto-expand in test results
- Auditing whether the env var is being respected across profile switches

## Protocol

### Step 1: Check env and stats

```bash
echo "APHRODITE_NO_AUTO_EXPAND=${APHRODITE_NO_AUTO_EXPAND:-not set}"
```

```python
aphrodite_stats()
```

Verify `engine.compressions == 0` and `last_prompt_tokens` is well below `threshold_tokens` (720K default). If the engine has already fired, the conversation may have pre-existing markers.

### Step 2: Produce large terminal output

Run a command that generates >5KB of stdout. The token proxy has a base threshold of 1KB, so content well above 1KB triggers compression. A safe target is 7–10KB.

```bash
cat crates/aphrodite/src/proxy.rs crates/aphrodite/src/main.rs crates/aphrodite/src/retrieve.rs 2>&1 | head -200
```

Or generate synthetic content:

```bash
python3 -c "for i in range(200): print(f'// Section {i}: ' + 'x' * 80)"
```

### Step 3: Observe the output in context

Look for `<<<CCR:hash|type|size>>>` at the start of the terminal output.

| Result | Meaning |
|--------|---------|
| `<<<CCR:hash|terminal|N>>> //! aphrodite...` | Auto-expand is OFF — raw marker visible |
| Full expanded content (all lines shown) | Either auto-expand is ON, or context is below threshold |

### Step 4: Verify retrieval

```python
aphrodite_retrieve(hash="<hash from marker>")
```

Should return the full content. The retrieve tool **always** returns expanded content regardless of `APHRODITE_NO_AUTO_EXPAND` — this is deliberate (contrast with engine auto-expand).

### Step 5: Check proxy stats

```python
aphrodite_stats()
```

Key fields to check:
- `proxy.token.requests_compressed` — increased by at least 1 (proxy compressed the terminal output)
- `proxy.token.tokens_saved` — reflects the size difference
- `engine.compressions` — should still be 0 (the proxy did the compression, not the engine)

## Why This Works

The compression pipeline has two independent stages:

1. **Proxy response compression** (Rust binary) — intercepts `/v1/chat/completions` responses, compresses large `message.content` and `tool_calls[].function.arguments`. Produces `<<<CCR:...>>>` markers. Always active when CCR is enabled.

2. **Engine auto-expand** (Python plugin `_hooks.py` → `compute_compressed_conversation`) — runs in the `pre_llm` hook before each LLM turn. Finds CCR markers in the conversation and resolves them by fetching content from the proxy. Controlled by `APHRODITE_NO_AUTO_EXPAND`.

Terminal output from a `terminal()` tool call lands in the tool result message. On the next LLM turn, the context engine sees it — if the content exceeds the type-specific threshold, the proxy compresses it into a CCR marker during the API request/response cycle. With auto-expand OFF, that marker stays raw in the LLM's visible context.

## Pitfalls

- **Terminal stdout cap**: the terminal tool caps output at 50KB. Output beyond this is truncated and the proxy never sees the full content. Stay under 40KB for reliable testing.
- **Proxy vs engine mixing**: "proxy did 11 compressions" does not mean "engine fired". Check `engine.compressions` specifically, not `proxy.token.requests_compressed`.
- **protect_first_n / protect_last_n**: The first 5 and last 7 messages in the conversation are protected from compression. If your test command output is in a protected slot, it won't be compressed regardless of size.
- **Cross-profile sessions**: `APHRODITE_NO_AUTO_EXPAND` is an env variable. If you switch profiles without re-sourcing, it may not be set. Verify with `echo $APHRODITE_NO_AUTO_EXPAND` before testing.
- **Nested compression**: Retrieving a CCR marker via curl through the proxy can itself be compressed — you may see two CCR entries in the catalog for one test cycle (the original output + the curl response). This is expected.
- **Cross-profile skill editing blocked**: `skill_manage` with `action=write_file` or `action=patch` cannot modify skills that live under a different Hermes profile. The canonical `aphrodite-boundary-behaviors` skill (under `aphrodite-compress-aggressive`) already references `references/testing-auto-expand.md` but the file has not been created. If you have access to that profile, create the reference file there instead.

## Quick Reference: Expected Behavior Matrix

| Scenario | What the LLM sees | What retrieve returns |
|----------|------------------|---------------------|
| `APHRODITE_NO_AUTO_EXPAND=1` | `<<<CCR:...\|terminal\|7104>>>` (raw) | Full expanded content |
| `APHRODITE_NO_AUTO_EXPAND` unset/0 | Full expanded content | Full expanded content |
| Context engine threshold not reached | Full expanded content | Full expanded content |
