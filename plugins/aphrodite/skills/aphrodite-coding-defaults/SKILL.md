---
name: aphrodite-coding-defaults
description:
    "Coding-optimized compression defaults, centers, auto-expand, submodule
    sync, and auto-release pipeline for aphrodite v0.5.104+."
version: 1.0.0
platforms: [macos]
---

# Aphrodite Coding Defaults

Reference for all tunable compression settings, their defaults, and env
overrides.

## Engine Thresholds

| Setting              | Default      | Env Var                          |
| -------------------- | ------------ | -------------------------------- |
| ENGINE_THRESHOLD_PCT | 55           | `APHRODITE_ENGINE_THRESHOLD_PCT` |
| ENGINE_PROTECT_FIRST | 3            | `APHRODITE_ENGINE_PROTECT_FIRST` |
| ENGINE_PROTECT_LAST  | 8            | `APHRODITE_ENGINE_PROTECT_LAST`  |
| ENGINE_MIN_MSGS      | 12 (dynamic) | `APHRODITE_ENGINE_MIN_MSGS`      |

MIN_MSGS is dynamic: max(12, min(session_msgs/10, 50)). System messages never
compress.

## Compression

| Setting         | Default         | Env Var                       |
| --------------- | --------------- | ----------------------------- |
| Code multiplier | ×2              | `APHRODITE_CODE_MULTIPLIER=4` |
| Auto-expand     | OFF             | `APHRODITE_AUTO_EXPAND=1`     |
| Budget curve    | linear 0.50–1.0 | N/A                           |

## Centers

LLM passes `_ccr_center` in tool params — travels with marker through
retrievals:

- `code_rust` / `code_python` — code-aware extraction
- `debug` — full errors, deeper extraction
- `compact` — minimal previews
- Centers embed as `;center=X` in marker structure line

## Submodule Sync

```bash
git submodule update --remote --recursive --merge
git add --force vendor/headroom
```

## Auto-Release Pipeline

`scripts/auto-release.sh` handles:

1. Submodule sync
2. Stage + commit
3. Version bump (Cargo.toml + BIN_VERSION auto-sync)
4. cargo build --release
5. cargo test
6. Tag + push

Always pass `GIT_EDITOR=true` to suppress tag prompts.

## Pitfalls

- API rejects emoji in tool names: must match `^[a-zA-Z0-9_-]+$`
- Prometheus counters must end in `_total`
- Submodule commits must commit inside submodule first, then git add --force in
  parent
- rand::random() in rand 0.10 IS thread-local — no need to replace with
  thread_rng()
- format! requires string literal, not const — use a function for templates
- rhai::Engine is !Send — store source strings, create Engine per call
