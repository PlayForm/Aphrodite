---
name: aphrodite-auto-expand-testing
description: "Use when testing if CCR markers auto-expand or stay raw. Controls: TOML auto_expand_limit, APHRODITE_AUTO_EXPAND, retrieve behavior."
version: 2.1.0
platforms: [macos]
tags: [aphrodite, ccr, auto-expand, testing]
---

# Aphrodite Auto-Expand Testing

Protocol for testing auto-expand behavior: whether `<<<CCR:...>>>` markers
appear raw in the LLM's view or get auto-resolved to full content before the
turn. Use when changing auto-expand config, when markers appear unexpectedly,
or when verifying the context engine.

## Mechanism

The plugin is a thin ctypes loader; all compression/engine/auto-expand logic
lives in the Rust dylib. Auto-expand is controlled by `AUTO_EXPAND_LIMIT`
(int): when `> 0`, the pre_llm hook scans the conversation for CCR markers and
resolves those where `marker_size < AUTO_EXPAND_LIMIT`.

**Defaults** (from `~/.hermes/aphrodite.toml`):

- `compression.auto_expand_limit` = 5 → `AUTO_EXPAND_LIMIT` = 5
- With limit=5, only markers < 5 bytes get resolved → **effectively OFF**
- `APHRODITE_AUTO_EXPAND=1` enables aggressive mode (limit becomes 51200 = 50KB)

`APHRODITE_NO_AUTO_EXPAND` does NOT exist in the source - never use it. The
default (limit=5) already produces raw markers.

## The Three Layers

1. **Proxy response compression** (Rust, :9798) - compresses provider RESPONSE
   messages. Produces `<<<CCR:...>>>` in model responses. Always active.
2. **Context engine** - compresses MIDDLE messages in conversation_history into
   a CCR marker. Fires on turn 2+ when the threshold is reached. Tool call
   outputs get compressed into CCR memory containers; the LLM sees markers and
   polls via `aphrodite_retrieve()`.
3. **Auto-expand** (pre_llm hook) - resolves CCR markers to full content before
   the LLM sees them. Controlled by `AUTO_EXPAND_LIMIT`.

## Protocol

### Step 1: Check current config

```bash
grep -n "auto_expand" ~/.hermes/aphrodite.toml
```

### Step 2: Test context engine with raw markers (default)

Start a multi-turn session (default config already has auto-expand effectively
OFF):

```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1 \
	APHRODITE_ENGINE_MIN_MSGS=4 \
	APHRODITE_ENGINE_PROTECT_FIRST=1 \
	APHRODITE_ENGINE_PROTECT_LAST=1 \
	hermes
```

Run several tool calls. After turn 2+, the context engine compresses tool
results. The LLM sees `<<<CCR:hash|context|N>>>` - use
`aphrodite_retrieve(hash)` to poll the CCR memory container.

### Step 3: Test with auto-expand ON

```bash
APHRODITE_AUTO_EXPAND=1 hermes
```

Markers under 50KB get auto-resolved - the LLM sees full content instead of
markers.

### Step 4: Verify retrieval always works

```python
aphrodite_retrieve(hash="<hash>")
```

The retrieve tool **always** returns full content regardless of the auto-expand
setting.

## Expected Behavior Matrix

| Config                    | What LLM sees                      | Retrieve returns |
| ------------------------- | ---------------------------------- | ---------------- |
| Default (limit=5)         | `<<<CCR:hash\|context\|N>>>` (raw) | Full content     |
| `APHRODITE_AUTO_EXPAND=1` | Expanded content (<50KB)           | Full content     |
| Engine threshold not hit  | Full content                       | Full content     |

## Pitfalls

- never use `APHRODITE_NO_AUTO_EXPAND` - the variable does not exist; the
  default (limit=5) already yields raw markers
- never test with a single-turn session (`hermes -z`) - the context engine
  needs turn 2+
- don't confuse the layers - the proxy compresses provider responses (:9798);
  the engine compresses conversation middle messages
- protected first/last messages (`protect_first_n` / `protect_last_n`) stay raw
  regardless of limit