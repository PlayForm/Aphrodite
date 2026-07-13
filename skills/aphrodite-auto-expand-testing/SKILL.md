---
name: aphrodite-auto-expand-testing
description: "Protocol for testing auto-expand behavior - controlled by AUTO_EXPAND_LIMIT
    from TOML auto_expand_limit (default 5 → effectively OFF).
    APHRODITE_AUTO_EXPAND=1 enables aggressive. Context engine is the real async
    engine."
version: 2.0.0
platforms: [macos]
related_skills: [aphrodite-boundary-behaviors, aphrodite-dev-workflow]
---

# Aphrodite Auto-Expand Testing

Protocol for testing auto-expand behavior: whether `<<<CCR:...>>>` markers
appear raw or get auto-resolved.

**Historical note:** the file references below (`plugins/aphrodite/_hooks/session.py`,
`_core/config.py`, `_engine.py`) describe the pre-Rust-port Python plugin
layout. The current plugin (`plugins/aphrodite/`) is a thin ctypes loader -
all compression/engine/auto-expand logic now lives in the Rust dylib
(`crates/aphrodite/src/`, `crates/aphrodite-hermes/src/`). The *mechanism*
described here (TOML `auto_expand_limit`, `APHRODITE_AUTO_EXPAND` env var,
the pre_llm hook resolving markers below the limit) still applies
conceptually, but the specific Python file/line citations no longer exist
in this tree and Step 1's `from _core import AUTO_EXPAND_LIMIT` will fail.

## Mechanism

Auto-expand is controlled by `AUTO_EXPAND_LIMIT` (int). When `> 0`, the pre_llm
hook scans conversation for CCR markers and resolves those where
`marker_size < AUTO_EXPAND_LIMIT`.

**Defaults** (from `aphrodite.toml`):

- `compression.auto_expand_limit` = 5 → `AUTO_EXPAND_LIMIT` = 5
- With limit=5, only markers < 5 bytes get resolved → **effectively OFF**
- Set `APHRODITE_AUTO_EXPAND=1` to enable aggressive (limit becomes 51200 =
  50KB)

**`APHRODITE_NO_AUTO_EXPAND` does NOT exist in the source code.** It was
fictional in old skills. Default (limit=5) already produces raw markers.

## The Three Layers

1. **Proxy response compression** (Rust, :9798) - compresses provider RESPONSE
   messages. Produces `<<<CCR:...>>>` in model responses. Always active.

2. **Context engine** (Python, `_engine.py`) - compresses MIDDLE messages in
   conversation_history into a CCR marker. Fires on turn 2+ when threshold
   reached. This is the "async engine" - tool call outputs get compressed into
   CCR memory containers, LLM sees markers and polls via `aphrodite_retrieve()`.

3. **Auto-expand** (pre_llm hook) - resolves CCR markers to full content before
   LLM sees them. Controlled by `AUTO_EXPAND_LIMIT`.

## Protocol

### Step 1: Check current config

```bash
python3 -c "
import sys; sys.path.insert(0, 'plugins/aphrodite')
from _core import AUTO_EXPAND_LIMIT
print(f'AUTO_EXPAND_LIMIT={AUTO_EXPAND_LIMIT}')
print('Auto-expand is', 'ON' if AUTO_EXPAND_LIMIT > 50 else 'effectively OFF')
"
```

### Step 2: Test context engine with raw markers (default)

Start a multi-turn session. Default config already has auto-expand effectively
OFF:

```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1 \
	APHRODITE_ENGINE_MIN_MSGS=4 \
	APHRODITE_ENGINE_PROTECT_FIRST=1 \
	APHRODITE_ENGINE_PROTECT_LAST=1 \
	hermes
```

Run several tool calls. After turn 2+, the context engine compresses tool
results. LLM sees `<<<CCR:hash|context|N>>>` - must use
`aphrodite_retrieve(hash)` to poll from the CCR memory container.

### Step 3: Test with auto-expand ON

```bash
APHRODITE_AUTO_EXPAND=1 hermes
```

Markers under 50KB get auto-resolved - LLM sees full content instead of markers.

### Step 4: Verify retrieval always works

```python
aphrodite_retrieve(hash="<hash>")
```

The retrieve tool **always** returns full content regardless of auto-expand
setting.

## Expected Behavior Matrix

| Config                    | What LLM sees                      | Retrieve returns |
| ------------------------- | ---------------------------------- | ---------------- |
| Default (limit=5)         | `<<<CCR:hash\|context\|N>>>` (raw) | Full content     |
| `APHRODITE_AUTO_EXPAND=1` | Expanded content (<50KB)           | Full content     |
| Engine threshold not hit  | Full content                       | Full content     |

## Pitfalls

- **`APHRODITE_NO_AUTO_EXPAND` is fictional** - no such env var in source.
  Default (limit=5) already means raw markers.
- **Single-turn (`hermes -z`)**: context engine won't fire (needs turn 2+). Use
  multi-turn.
- **Proxy compresses responses, engine compresses messages**: proxy handles
  provider responses; engine handles conversation middle messages.
- **protect_first_n / protect_last_n**: protected messages stay raw.
