---
name: aphrodite-auto-expand-testing
description: "Use when debugging why CCR markers appear raw in the LLM view. Auto-expand is vestigial, engine threshold knobs, retrieve-first rule."
version: 2.2.0
platforms: [macos]
tags: [aphrodite, ccr, auto-expand, testing]
---

# Aphrodite Auto-Expand Testing

Protocol for understanding when `<<<CCR:...>>>` markers appear raw in the
LLM's view. Auto-expand machinery is **vestigial in the current codebase** - do
not debug marker appearance by chasing `auto_expand` config; the operative rule
is the retrieve-first doctrine (see `aphrodite-tool-testing`).

## Current reality (verified 2026-09-18)

- The Rust proxy serializes `auto_expand` / `auto_expand_limit` in its config
  endpoint, but **no consumer reads them** - the source comment at
  `crates/aphrodite/src/proxy.rs` marks `engine_threshold_pct`,
  `catalog_mode`, and `auto_expand*` as having no consumer.
- The plugin (`plugins/aphrodite/__init__.py`) has **no** `AUTO_EXPAND`
  handling; the `pre_llm_call` hook served by the dylib
  (`crates/aphrodite-hermes/src/lib.rs`) injects a catalog/guidance context -
  it does not expand markers.
- `APHRODITE_NO_AUTO_EXPAND` and `APHRODITE_AUTO_EXPAND` both have **zero
  consumers** in source - never use either; they silently change nothing.
- Live `~/.hermes/aphrodite/aphrodite.toml` still carries `auto_expand = true`
  and `auto_expand_limit = 102400` as leftover keys - they are inert.

Consequence: markers appear raw unless the context engine already resolved
them. Whenever you SEE a marker, `aphrodite_retrieve(hash)` it immediately -
retrieval is the only path that guarantees content.

## The Three Layers

1. **Proxy response compression** (Rust, :9798) - compresses provider RESPONSE
   messages. Produces `<<<CCR:...>>>` in model responses. Always active.
2. **Context engine** - compresses MIDDLE messages in conversation_history
   into a CCR marker when the threshold is reached. Fires on turn 2+; the LLM
   sees markers and polls via `aphrodite_retrieve()`.
3. **pre_llm catalog injection** - the dylib's `pre_llm_call` arm injects a
   catalog/guidance context string. No marker expansion happens here.

## Engine knobs (verified in config_loader.rs: env > TOML > default)

| Knob                       | Env var                          | TOML key                           | Default |
| -------------------------- | -------------------------------- | ---------------------------------- | ------- |
| Compression threshold %    | `APHRODITE_ENGINE_THRESHOLD_PCT` | `compression.engine_threshold_pct` | 45      |
| Min messages before engine | `APHRODITE_ENGINE_MIN_MSGS`      | `compression.engine_min_msgs`      | 8       |
| Protect first N            | `APHRODITE_ENGINE_PROTECT_FIRST` | `compression.engine_protect_first` | 2       |
| Protect last N             | `APHRODITE_ENGINE_PROTECT_LAST`  | `compression.engine_protect_last`  | 5       |

Live `~/.hermes/aphrodite/aphrodite.toml` sets `engine_threshold_pct = 100`
(100% = effectively disables CCR compression of tool output, per standing
feedback) - do not expect tool-output markers by default; markers come from
terminal output, the token proxy, and cross-session content instead. The plugin
registers the context engine only when `APHRODITE_CONTEXT_ENGINE=1` is set;
the dylib side additionally honors `compression.context_engine` (default
true).

## Protocol

### Step 1: Check current config

```bash
grep -n "auto_expand\|engine_threshold\|context_engine" ~/.hermes/aphrodite/aphrodite.toml
```

### Step 2: Force engine compression to observe markers

```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1 \
	APHRODITE_ENGINE_MIN_MSGS=4 \
	APHRODITE_ENGINE_PROTECT_FIRST=1 \
	APHRODITE_ENGINE_PROTECT_LAST=1 \
	hermes
```

Run several tool calls. After turn 2+, the context engine compresses middle
tool results - the LLM sees `<<<CCR:hash|context|N>>>` and must poll with
`aphrodite_retrieve(hash)`.

### Step 3: Verify retrieval always works

`aphrodite_retrieve(hash="<hash>")` returns the full original content
regardless of config.

## Expected Behavior Matrix

| Config                      | What LLM sees             | Retrieve returns |
| --------------------------- | ------------------------- | ---------------- |
| Live TOML (threshold 100%)  | Full content (no markers) | Full content     |
| Engine threshold forced low | `<<<CCR:hash              | context          | N>>>` | Full content |
| Terminal output > 512 bytes | `<<<CCR:hash              | terminal         | N>>>` | Full content |

## Pitfalls

- never chase `auto_expand` config to explain raw markers - the keys have no
  consumer in the current source; the retrieve-first rule is the answer
- never use `APHRODITE_NO_AUTO_EXPAND` or `APHRODITE_AUTO_EXPAND` - neither
  has a consumer; they silently change nothing
- never test with a single-turn session (`hermes -z`) - the context engine
  needs turn 2+
- don't confuse the layers - the proxy compresses provider responses (:9798);
  the engine compresses conversation middle messages
- protected first/last messages (`engine_protect_first` / `engine_protect_last`)
  stay raw regardless of threshold
