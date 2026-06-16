---
name: aphrodite-dev-workflow
description: "End-to-end aphrodite plugin development: cargo watch, WezTerm MCP, Hermes testing, binary build, context engine setup, git workflow."
version: 2.0.0
platforms: [macos]
---

# Aphrodite Development Pitfalls (Updated 2026-06-16)

## Critical Gotchas

### Aggressive Profile Uses Cache Proxy (NOT Token)
The aggressive profile defaults to `provider: aphrodite-cache` (:9797) which caches LLM responses and **skips compression**. For actual compression, must switch to token: `hermes config set model.provider aphrodite-token --profile aphrodite-compress-aggressive`. Symptom: 99% cache hits, near-zero compression activity.

### Skills Must Be Shipped With Plugin
The plugin registers 9 tools but ships 0 skills by default. Skills `aphrodite-dev-workflow`, `aphrodite-hook-reference`, and `aphrodite-iterate-release` must be symlinked into `plugins/aphrodite/skills/` and registered via `ctx.register_skill()` in `__init__.py`. Otherwise agents run 55+ minutes without critical context.

### Profile Symlinks (Not Files)
Profile `plugins/aphrodite` and `skills` must be **symlinks**, not text files containing paths. Text files cause FileExistsError on startup. Check: `ls -la ~/.hermes/profiles/<name>/plugins/aphrodite` - should show `lrwxr-xr-x` with `->` target. Fix: `rm` the file, `ln -s` to source.

### Plugin Must Be Enabled Per Profile
After profile sync or copy, the plugin is "not enabled" by default. Check: `hermes plugins list --profile <name>`. Enable: `hermes plugins enable aphrodite --profile <name>`.

### Session State Is Per-Profile
`state.db` is per-profile. Sessions from one profile cannot be resumed in another via `--resume`. CCR content IS shared via the proxy database - use `aphrodite_retrieve` and `aphrodite_search` to access compressed content across profiles.

### Never Kill Proxy During Release
Killing the proxy clears in-memory CCR cache and stats (tokens_saved, requests). SQLite CCR survives. Sync only clears `__pycache__`, never proxy process. The release pipeline step 8 (SYNC) must NOT include proxy restart.

### aphrodite.toml api_key Overrides Env
The `api_key` field in `aphrodite.toml` takes priority over `APHRODITE_API_KEY` env var. If auth fails with 401 but the env var is correct, check for a hardcoded key in the toml file. Fix: comment out the `api_key` line.

### Source Doesn't Propagate in Background
`terminal(background=true)` does not inherit `source`'d env vars. Use explicit env files or pass values inline. `.env.sh` files with `export KEY=value` format are the standard approach.

### Title Generation UTF-16 Error
The aphrodite proxy's compressed responses corrupt the auxiliary title generation client. Route title generation through deepseek directly: `hermes config set auxiliary.title_generation.provider deepseek --profile <name>`.

### Flash Agents Return Summaries Only
When dispatching via `hermes -z --model deepseek-v4-flash`, the main session only sees the agent's final summary response. Full file contents and tool outputs stay in the flash agent's session. This is by design - flash is an execution layer, not a context extension.

### Model Catalog for Proxy Providers
Aphrodite proxy doesn't serve `/v1/models`. Add static model entries in profile config:
```yaml
model_catalog:
  providers:
    aphrodite-token:
      - deepseek-v4-pro
      - deepseek-v4-flash
```

### Cargo Watch Port Conflicts
When cargo watch rebuilds after file changes, the new proxy process conflicts with any manually-started proxy on the same ports. Either kill cargo watch or configure it to use different ports via `aphrodite.toml`.

### Benchmark Results Are Timestamped
`scripts/benchmark.py` writes to `.hermes/benchmark-results.json` with each run. Results are naturally keyed by timestamp - no need for JSONL accumulation. Just don't clear between runs.

## When to Load

Any aphrodite development: plugin code, proxy Rust, hook debugging, testing via Hermes, binary rebuilds, git, context engine setup.

## Architecture

Plugin registers 5 hooks + 3 tools + 1 context engine. Proxy: cache (:9797) + token (:9798).

## Dev Environment

Pane 0 (proxy): `RUST_LOG=debug cargo watch -x 'run -p aphrodite'` with API key inline.
Pane 3 (test): `hermes --provider custom:aphrodite-token`

**CRITICAL**: Send control chars SEPARATELY from command text via MCP. Merging causes garbled input like "x03x03APHRODITE_API_KEY=...".

## Context Engine Setup

Activate: `hermes config set context.engine aphrodite` (revert: `compressor`)

Requirements:
- compression.enabled: true
- Engine MUST inherit from agent.context_engine.ContextEngine
- name MUST be @property, not class attr
- update_model needs all 7 params from abstract base
- Takes effect on next Hermes session start

## Git

Stage specific files only, never -A. Branch: Current -> aphrodite/Current.

## Testing

Terminal: echo with marker - must show output (empty = stdout vs output mismatch).
Proxy: check /health and /stats on proxy ports.
Context engine: look for "Using context engine: aphrodite" in logs.

## Common Pitfalls

1. Context engine silently rejected if plain class (isinstance check fails)
2. name as class attr instead of @property => rejected
3. update_model param mismatch => rejected
4. Control chars merged with text via MCP => garbled input
5. pre_api_request hook never fires in Hermes v0.16.0
6. turn_id from Hermes is UUID string, not counter
7. toolset param on register_tool drops tools in v0.16.0+
