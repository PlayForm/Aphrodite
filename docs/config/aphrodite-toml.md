# aphrodite.toml

Aphrodite's proxy listeners and compression engine are configured through a single TOML file. This page documents every section, field, precedence rule, and validation check; the companion page [Environment Variables](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/env-vars.md) lists every env var that overrides a value here.

This file is Aphrodite's own proxy/engine config - a **different file** from Hermes Agent's `config.yaml`. See [Troubleshooting: two separate config files](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/troubleshooting.md#two-separate-config-files) if you came here looking for Hermes-side keys like `plugins.enabled` or `context.engine`.

## File location

|                              |                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------ |
| Default                      | `aphrodite.toml` in the current working directory                                          |
| Fallback (default path only) | `~/.hermes/aphrodite/aphrodite.toml` - where `aphrodite setup` writes its generated config |
| Override                     | `APHRODITE_CONFIG_PATH` environment variable                                               |
| If missing                   | Falls back to CLI-flag mode (single proxy, see [CLI equivalents](#cli-equivalents))        |

The `~/.hermes/aphrodite/aphrodite.toml` fallback only applies when `APHRODITE_CONFIG_PATH` was **not** explicitly set - an explicit override that points at a nonexistent file still falls through to CLI-flag mode rather than silently redirecting elsewhere. `aphrodite setup` writes its generated config (with `{cache_port}` / `{token_port}` placeholders substituted) from the same embedded template that ships as `aphrodite.toml.example` in the repo root.

## Precedence

For every field except the two noted below, the resolution order is **env var → TOML (proxy entry → `[defaults]`) → compiled-in default**: an env var wins if set and parses; otherwise the proxy entry's value wins; otherwise `[defaults]`; otherwise the default compiled into the binary. A present-but-malformed env value (e.g. `APHRODITE_CCR_TTL=abc`) is never treated as absent - it logs a warning and falls through to the next level.

Two exceptions:

- `api_key`: TOML comes **first** (`proxy.api_key` → `defaults.api_key` → `APHRODITE_API_KEY`), because the explicit config is the primary way to set per-proxy keys. See [API key resolution](#api-key-resolution).
- `mode` and `listen`: TOML-only per `[[proxies]]` entry - a single process-wide `APHRODITE_MODE` / `APHRODITE_LISTEN` would incorrectly apply to every proxy at once. The two port-specific overrides below are the only env influence on `listen`.

CLI flags apply only in CLI-fallback mode (no `aphrodite.toml` present at all) - see [CLI equivalents](#cli-equivalents).

## Full schema

```toml
[defaults]
# Shared defaults applied to every [[proxies]] entry that doesn't override them.
ccr_ttl_seconds = 3600

[[proxies]]
name = "cache"
listen = "127.0.0.1:9797"
mode = "cache"
tool_relay = true
timeout = 120

[[proxies]]
name = "token"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
timeout = 300

[compression]
engine_threshold_pct = 45        # engine status flag (dylib session only)
tool_threshold_token = 512       # token-proxy compression threshold, bytes
tool_threshold_cache = 4096      # cache-proxy compression threshold, bytes
terminal_threshold = 1024        # terminal-output threshold, bytes (dylib)
inline_threshold = 2048          # inline-vs-durable CCR cutoff, bytes
code_multiplier = 3.0            # multiplies threshold for code_* types
chain_split = false              # opt-in fine-grained command splitting

[previews]
preview_max_chars = 120          # cap on rendered preview strings; absent = unlimited

[prompts]
session_inject = "..."           # first-turn orientation text; "" disables

[directives]
active = ["focus", "foresight"]

[flow]
budget_chars = 2600              # hard cap on per-turn injected context (dylib)
```

The root `aphrodite.toml.example` ships the full annotated version, including the `[templates.*]` blocks. The schema is not strict: unknown keys parse and are ignored, and every section above is optional.

## `[[proxies]]` fields

| Field                       | Meaning                                                                        | Default                      |
| --------------------------- | ------------------------------------------------------------------------------ | ---------------------------- |
| `name`                      | Proxy label, also used for port-override matching                              | listen address               |
| `listen`                    | Bind address (loopback-only unless changed - see below)                        | `127.0.0.1:9797`             |
| `mode`                      | `"cache"` or `"token"` - backend + compression threshold (see [Modes](#modes)) | `"token"`                    |
| `api_key`                   | Upstream API key, overrides `[defaults]`                                       | -                            |
| `api_url`                   | Upstream API base URL, overrides `[defaults]`                                  | -                            |
| `model`                     | Model name to forward, overrides `[defaults]`                                  | -                            |
| `tool_relay`                | Enable the `/tool/relay` endpoint                                              | `false`                      |
| `dev`                       | Verbose request/response logging                                               | `false`                      |
| `ccr_ttl_seconds`           | CCR entry time-to-live, seconds                                                | `3600`                       |
| `ccr_db_path`               | SQLite path for the token proxy                                                | `~/.hermes/aphrodite/ccr.db` |
| `notify_url` / `notify_key` | Hermes callback URL + bearer token for CCR-create notifications                | -                            |
| `timeout`                   | Upstream request timeout, seconds (clamped to 600)                             | `300`                        |
| `max_context`               | Max context tokens                                                             | `1,000,000`                  |
| `max_output`                | Max output tokens (must be less than `max_context`)                            | `384,000`                    |

`[defaults]` accepts `api_url`, `model`, `ccr_ttl_seconds`, and `api_key`; each applies to every proxy that doesn't set its own value. No provider-specific defaults are baked in - `api_url` / `model` resolve from env or config, and fall back to `https://api.openai.com` / `default-model` only when neither is set.

All listeners bind loopback by default, and the proxy rejects non-loopback peers (and non-loopback `Host` headers) on every route except `/health`. Binding `0.0.0.0` is possible but exposes unauthenticated management routes (`/retrieve`, `/history`, `/ccr/*`) to your network - see the shipped example's comments on the `0.0.0.0` lines.

## `[compression]`

One shared table feeds two independent consumers:

**Drives the Rust proxy** (env var wins over these if set; editing and saving, or `POST /reload`, applies immediately with no restart):

| Field                  | Meaning                                                                 | Env override                     | Compiled default | Shipped example |
| ---------------------- | ----------------------------------------------------------------------- | -------------------------------- | ---------------- | --------------- |
| `tool_threshold_token` | Token-proxy compression threshold, bytes                                | `APHRODITE_TOOL_THRESHOLD_TOKEN` | `1024`           | `512`           |
| `tool_threshold_cache` | Cache-proxy compression threshold, bytes                                | `APHRODITE_TOOL_THRESHOLD_CACHE` | `8192`           | `4096`          |
| `inline_threshold`     | Inline-vs-durable CCR storage cutoff, bytes                             | `APHRODITE_INLINE_THRESHOLD`     | `256`            | `2048`          |
| `code_multiplier`      | Multiplies the threshold for `code_*` content types (keeps code inline) | `APHRODITE_CODE_MULTIPLIER`      | `3.0`            | `3.0`           |

The compiled defaults apply when neither env nor TOML sets a value; the shipped example config sets the values in the last column, so installations using it see those instead.

**Drives the Hermes-plugin dylib session** (a separate process/codepath from the Rust proxy, read once at dylib load):

| Field                      | Meaning                                                                                                      | Env override                         | Default |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------ | ------- |
| `terminal_threshold`       | Terminal-output compression threshold, bytes - gates `transform_terminal_output`                             | `APHRODITE_TERMINAL_THRESHOLD`       | `1024`  |
| `poll_worker`              | Auto-backgrounding of slow `terminal` / `process` calls (pre-tool-call rewrite)                              | `APHRODITE_POLL_WORKER`              | `true`  |
| `chain_split`              | Opt-in fine-grained command splitting (`SEG_MARKER` segments); off by default                                | `APHRODITE_CHAIN_SPLIT`              | `false` |
| `chain_split_min_segments` | Floor for the adaptive split threshold                                                                       | `APHRODITE_CHAIN_SPLIT_MIN_SEGMENTS` | `2`     |
| `chain_split_max_segments` | Cap for the adaptive split threshold                                                                         | `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS` | `6`     |
| `context_engine`           | Status flag exposed via `aphrodite_stats` / `aphrodite_config_get`; the standalone HTTP proxy never reads it | `APHRODITE_CONTEXT_ENGINE`           | `true`  |

The `engine_*` family (`engine_threshold_pct`, `engine_protect_first`, `engine_protect_last`, `engine_min_msgs`) is parsed into the dylib session state and exposed the same way, but is **not consulted by any compression decision** - it is populated for visibility, not load-bearing. (`engine_threshold_pct` has no effect on the engine; the shipped example sets it to 45 as the documented value, with 100+ as the escape hatch to disable engine compression entirely.)

**Not read by anything** (parsed for schema compatibility, echoed by `/reload` for visibility, no consumer): `auto_expand`, `auto_expand_limit`, `catalog_mode` (catalog mode is whatever the caller passes per-request), `classifier_poll`, `prefetch` (the `aphrodite_prefetch` tool exists regardless of this key).

## `[previews]` and `[prompts]`

| Section      | Field                  | Meaning                                                                                        | Status                                                                              |
| ------------ | ---------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `[previews]` | `preview_max_chars`    | Caps the rendered preview string, in chars; absent/0 = unlimited                               | Wired (env `APHRODITE_PREVIEW_MAX_CHARS` > TOML > unlimited; shipped example `120`) |
| `[previews]` | `model_family`         | `"compact"` \| `"code_first"` \| `"balance"` - preview template family                         | Reserved - no reader                                                                |
| `[previews]` | `code_structure_map`   | Include function/struct/class signatures in code previews                                      | Reserved - no reader                                                                |
| `[previews]` | `rust_preview_lines`   | Lines of Rust source to include in previews                                                    | Reserved - no reader                                                                |
| `[prompts]`  | `session_inject`       | First-turn orientation text injected by `pre_llm_call` on turn 0; `""` disables                | Wired (env `APHRODITE_SESSION_INJECT` > TOML > shipped builtin)                     |
| `[prompts]`  | `retrieve_guidance`    | `"minimal"` \| `"standard"` \| `"verbose"` - how much the system prompt explains CCR retrieval | Reserved - no reader                                                                |
| `[prompts]`  | `ccr_marker_hint`      | Append a retrieval hint after markers                                                          | Reserved - no reader                                                                |
| `[prompts]`  | `catalog_intent_hints` | Show intent hints alongside hashes in catalog output                                           | Reserved - no reader                                                                |

"Reserved - no reader" means the values parse cleanly but nothing in the current codebase reads them back out; changing them does not change behavior, so treat them as reserved until that's confirmed. `[previews] preview_max_chars` and `[prompts] session_inject` are the two keys in these sections that do have live effects.

## `[templates.*]`

The shipped example config carries a large `[templates.preview.*]` / `[templates.marker]` / `[templates.prompts]` / `[templates.reverse]` block of per-content-type format strings, with a variable reference (`{type}`, `{ln}`, `{size}`, `{hash}`, `{fns}`, `{sigs}`, etc.) in its header comment.

Nothing in the codebase renders previews from these templates - preview strings are generated internally. If you're editing `[templates.*]` expecting it to change preview output, verify against your running version first; treat the section as reserved/aspirational rather than functioning configuration.

## `[directives]`

```toml
[directives]
active = ["focus", "foresight"]  # e.g. ["focus", "foresight"]
```

| Field    | Meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `active` | Which loaded directives start active. Directive `.md` files are discovered from `APHRODITE_DIRECTIVES_DIR` (if set) → `./directives/` → `~/.hermes/aphrodite/directives/` → binary-relative - the **first directory that exists** wins, and an existing-but-empty directives dir is intentional (no custom directives). If no directory exists, built-in directives are used. Names in `active` that aren't in the loaded set are silently filtered out; if `active` resolves empty while directives ARE loaded, the session seeds `focus` / `foresight` / `lazy` from the loaded set instead. Loading is never gated on this list being non-empty. |

Read by the Hermes-plugin dylib session, not the Rust proxy. The active set is fully runtime-mutable via the `aphrodite_directive` tool - see [Directives](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/directives.md) for the complete feature reference.

## `[flow]`

| Field          | Meaning                                                                                           | Env override                  | Default | Shipped example |
| -------------- | ------------------------------------------------------------------------------------------------- | ----------------------------- | ------- | --------------- |
| `budget_chars` | Hard cap for ALL per-turn injected context (directives + nudges + recall catalog + retrieve hint) | `APHRODITE_FLOW_BUDGET_CHARS` | `4000`  | `2600`          |

Sections drop bottom-up when over budget; directives and nudges never drop. Dylib-session only - the Rust proxy never reads `[flow]`.

## API key resolution

```
proxy.api_key
  → defaults.api_key
    → APHRODITE_API_KEY env var
      → error: no API key configured
```

Stops at the first non-empty value. This chain is TOML-first by design; the two legacy provider-specific fallbacks (`DEEPSEEK_API_KEY`, `HEADROOM_DEEPSEEK_KEY`) no longer exist in the codebase. The proxy refuses to start if the chain resolves empty.

## Default value chain

| Field                       | Resolution                                                                                                                         |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `listen`                    | `proxy.listen` → `127.0.0.1:9797`, then `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` override just the port (name/mode-matched) |
| `mode`                      | `proxy.mode` → `"token"` (unknown values fall back with a warning) - no env override in multi-proxy mode                           |
| `api_url`                   | `APHRODITE_API_URL` → `proxy.api_url` → `defaults.api_url` → `https://api.openai.com`                                              |
| `model`                     | `APHRODITE_MODEL` → `proxy.model` → `defaults.model` → `"default-model"`                                                           |
| `ccr_ttl_seconds`           | `APHRODITE_CCR_TTL` → `proxy.ccr_ttl_seconds` → `defaults.ccr_ttl_seconds` → `3600`                                                |
| `ccr_db_path`               | `APHRODITE_DB` → `proxy.ccr_db_path` (non-empty) → `~/.hermes/aphrodite/ccr.db` (or `/tmp` fallback)                               |
| `notify_url` / `notify_key` | `APHRODITE_NOTIFY_URL` / `APHRODITE_NOTIFY_KEY` → `proxy.notify_url` / `notify_key` → unset                                        |
| `tool_relay`                | `proxy.tool_relay` → `false`                                                                                                       |
| `dev`                       | `proxy.dev` → `false`                                                                                                              |
| `timeout`                   | `proxy.timeout` → `300` (clamped to a max of `600` with a warning)                                                                 |
| `max_context`               | `proxy.max_context` → `1,000,000`                                                                                                  |
| `max_output`                | `proxy.max_output` → `384,000`                                                                                                     |

## Hot-reload

`POST /reload` (per-listener) and the `aphrodite.toml` file watcher (all listeners at once) re-resolve the four live `[compression]` fields and write them into the running proxy's state immediately - no restart needed. `/reload` also re-applies `[previews] preview_max_chars`. `/reload`'s JSON response has `"applied": true`, echoes the values that took effect, and carries a `"parsed_only"` object for the fields that don't (see the `[compression]` breakdown above). The file watcher applies only the four thresholds. Everything else about a proxy (`listen`, `api_url`, `model`, `mode`, ...) is fixed at startup; changing those requires a restart.

## Validation

| Check                         | Behavior                                                                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `max_output` vs `max_context` | Refuses to start if `max_output >= max_context`                                                            |
| `listen` address              | Must parse as a valid socket address, or the proxy refuses to start                                        |
| API key                       | Must be non-empty after the full resolution chain, or the proxy refuses to start                           |
| `timeout`                     | Clamped to a 600-second maximum, with a warning if the configured value exceeds it                         |
| `mode`                        | Unknown values fall back to `"token"` with a warning; an unset `mode` also defaults to `"token"`, silently |

## Modes

| Mode      | Backend   | Compression threshold | Notes                                                                        |
| --------- | --------- | --------------------- | ---------------------------------------------------------------------------- |
| `"token"` | SQLite    | >1 KB (`1024`)        | Durable CCR storage; supports tool relay (per-proxy flag, default `false`)   |
| `"cache"` | In-memory | >8 KB (`8192`)        | Lightweight caching; `tool_relay` is independent of mode, not disabled by it |

Thresholds are the compiled defaults; the shipped example config lowers them (see `[compression]` above). The token proxy's SQLite path comes from `ccr_db_path` (or the default below) and its TTL from `ccr_ttl_seconds`; the cache proxy's in-memory store uses the same TTL.

## Database path resolution

A relative `ccr_db_path` is resolved against the binary's own directory, not the current working directory - so a relative path behaves consistently regardless of where the proxy is launched from. Parent directories are created automatically if missing.

## CLI equivalents

When running without `aphrodite.toml`, CLI flags mirror these fields:

| TOML field        | CLI flag          | Env var                 |
| ----------------- | ----------------- | ----------------------- |
| `mode`            | `--mode`          | `APHRODITE_MODE`        |
| `listen`          | `--listen`        | `APHRODITE_LISTEN`      |
| `api_url`         | `--api-url`       | `APHRODITE_API_URL`     |
| `api_key`         | `--api-key`       | `APHRODITE_API_KEY`     |
| `model`           | `--model`         | `APHRODITE_MODEL`       |
| `ccr_db_path`     | `--ccr-db-path`   | `APHRODITE_DB`          |
| `ccr_ttl_seconds` | `--ccr-ttl`       | `APHRODITE_CCR_TTL`     |
| `tool_relay`      | `--tool-relay`    | -                       |
| `notify_url`      | `--notify-url`    | `APHRODITE_NOTIFY_URL`  |
| `notify_key`      | `--notify-key`    | `APHRODITE_NOTIFY_KEY`  |
| `dev`             | `--dev`           | -                       |
| `log_compact`     | `--log-compact`   | `APHRODITE_LOG_COMPACT` |
| `timeout`         | `--timeout`       | -                       |
| `max_context`     | `--max-context`   | -                       |
| `max_output`      | `--max-output`    | -                       |
| (CCR off)         | `--no-ccr-marker` | -                       |

In CLI mode, `max_context` / `max_output` / `tool_relay` / `dev` / `timeout` / `no_ccr_marker` are flag-only; the env-attributed rows apply via clap directly.

## Example: full multi-proxy

```toml
[defaults]
ccr_ttl_seconds = 7200

[compression]
tool_threshold_token = 512
tool_threshold_cache = 4096
inline_threshold = 2048
code_multiplier = 3.0

[previews]
preview_max_chars = 120

[prompts]
session_inject = ""

[directives]
active = ["focus", "foresight"]

[[proxies]]
name = "cache"
listen = "127.0.0.1:9797"
mode = "cache"
tool_relay = true
timeout = 120

[[proxies]]
name = "token"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
notify_url = "https://hermes.internal/callback"
notify_key = "hermes-api-key-123"
timeout = 300
```
