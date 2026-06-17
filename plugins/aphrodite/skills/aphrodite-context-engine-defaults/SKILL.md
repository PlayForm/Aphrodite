---
name: aphrodite-context-engine-defaults
description: "Context engine defaults, async poll pattern, and correct launch config. Auto-expand is effectively OFF by default (limit=5). No special env var needed for raw markers."
version: 2.0.0
platforms: [macos]
related_skills: [aphrodite-auto-expand-testing, aphrodite-boundary-behaviors]
---

# Aphrodite Context Engine Defaults

The context engine is **default-on** — three-layer chain:

| Layer | Source | Value |
|-------|--------|-------|
| TOML config | `aphrodite.toml` → `[compression] context_engine` | `true` (line 46) |
| Env var fallback | `APHRODITE_CONTEXT_ENGINE` | defaults `True` |
| Plugin init | `__init__.py:150` → `engine_configured = CONTEXT_ENGINE` | registers on start |

From `_core/config.py:169`:
```python
CONTEXT_ENGINE = _cfg_bool("APHRODITE_CONTEXT_ENGINE", True, ("compression", "context_engine"))
```

Disable with: `APHRODITE_CONTEXT_ENGINE=0` or TOML `context_engine = false`.

## Auto-Expand Is Effectively OFF By Default

`aphrodite.toml:42`: `auto_expand_limit = 5`. This sets `AUTO_EXPAND_LIMIT = 5` — only markers with original content < 5 bytes get auto-resolved. Nothing real meets that threshold.

**The context engine compresses but auto-expand doesn't resolve** — this is the default. The LLM sees raw `<<<CCR:hash|context|N>>>` markers.

**To ENABLE auto-expand** (resolve markers inline): `APHRODITE_AUTO_EXPAND=1` (sets limit to 51200 = 50KB).

**`APHRODITE_NO_AUTO_EXPAND` does NOT exist in source** — it was fictional in old skills. Default is already raw markers.

Source: `_core/config.py:161-163`, `_hooks/session.py:155-189`.

## Async Poll Pattern (Live Containers)

The "async poll" pattern: context engine compresses tool output → `<<<CCR:hash|context|N>>>` marker → LLM sees raw marker (tiny placeholder) → LLM fetches content on demand via `aphrodite_retrieve(hash)`.

This is the DEFAULT behavior. The engine is already running, and auto-expand is effectively OFF.

## Correct Launch Config

Default session (already has raw markers):
```bash
hermes
```

Aggressive engine testing (compression triggers early):
```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1 \
APHRODITE_ENGINE_MIN_MSGS=4 \
APHRODITE_ENGINE_PROTECT_FIRST=1 \
APHRODITE_ENGINE_PROTECT_LAST=1 \
hermes
```

To enable auto-expand for debugging (LLM sees expanded content):
```bash
APHRODITE_AUTO_EXPAND=1 hermes
```

## Config Reference

From `aphrodite.toml`:
```toml
[compression]
engine_threshold_pct = 45      # compress at 45% context fill
engine_protect_first = 2       # messages to keep at start
engine_protect_last = 5        # messages to keep at end
engine_min_msgs = 8            # minimum messages before activating
context_engine = true          # default-on
auto_expand_limit = 5          # 0-5 = effectively OFF; 51200 = aggressive
```

## Pitfalls

- **Single-turn (`hermes -z`)**: context engine won't fire (needs turn 2+). Use multi-turn session.
- **First turn no compress**: `last_prompt_tokens` is 0 on first turn. `should_compress()` returns True (v0.7.4+), but actual compression needs enough messages after head/tail protection.
- **Message count gate**: need `≥ protect_first + protect_last + 3` messages (default: 2 + 5 + 3 = 10) for a compressible middle slice.
