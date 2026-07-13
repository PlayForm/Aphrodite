# aphrodite.toml Configuration

Multi-proxy deployments (cache + token, or multiple token instances) are
configured through a single TOML file. For `api_url`, `model`,
`ccr_ttl_seconds`, `ccr_db_path`, `notify_url`, `notify_key`, and the four
`[compression]` threshold fields, the real resolution order is **env var →
TOML → built-in default** - an env var wins if set (and parses), matching
every shipped TOML's own header comment. `mode` and `listen` are the
exception: they're TOML-only per `[[proxies]]` entry (plus the two
port-specific env overrides below), since a single process-wide
`APHRODITE_MODE`/`APHRODITE_LISTEN` would incorrectly apply to every proxy
at once. CLI flags only apply in CLI-fallback mode (no `aphrodite.toml`
present at all) - see [CLI Equivalents](#cli-equivalents). Full var-by-var
detail: [`docs/config/env-vars.md`](env-vars.md).

This file is Aphrodite's own proxy/engine config - a **different file** from
Hermes Agent's `config.yaml`. See
[Troubleshooting: two separate config files](../install/troubleshooting.md#two-separate-config-files)
if you came here looking for Hermes-side keys like `plugins.enabled` or
`context.engine` instead.

## File location

|                              |                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------ |
| Default                      | `aphrodite.toml` in the current working directory                                          |
| Fallback (default path only) | `~/.hermes/aphrodite/aphrodite.toml` - where `aphrodite setup` writes its generated config |
| Override                     | `APHRODITE_CONFIG_PATH` environment variable                                               |
| If missing                   | Falls back to CLI-flag mode (single proxy, see [CLI Equivalents](#cli-equivalents))        |

The `~/.hermes/aphrodite/aphrodite.toml` fallback only applies when
`APHRODITE_CONFIG_PATH` was **not** explicitly set - an explicit override
that points at a nonexistent file still falls through to CLI-flag mode
rather than silently redirecting elsewhere.

## Full schema

```toml
[defaults]
# Shared defaults across all [[proxies]]
api_url = "https://api.openai.com"
model = "default-model"
ccr_ttl_seconds = 3600
api_key = "sk-..."

[[proxies]]
name = "cache-proxy"
listen = "127.0.0.1:9797"
mode = "cache"
api_key = "sk-..."           # overrides defaults.api_key
api_url = "https://..."      # overrides defaults.api_url
model = "..."                # overrides defaults.model
tool_relay = false
dev = false
ccr_ttl_seconds = 3600
ccr_db_path = "~/data/ccr.db"
notify_url = "https://..."
notify_key = "bearer-token"
timeout = 300
max_context = 1000000
max_output = 384000

[[proxies]]
name = "token-proxy"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
```

Also accepted at the top level: `[compression]`, `[previews]`, and
`[prompts]` tables (see below) - omitted from this example because their
defaults are almost always fine. The repo's own root `aphrodite.toml` sets
them explicitly as a worked example.

## `[[proxies]]` fields

| Field                       | Meaning                                                         | Default                      |
| --------------------------- | --------------------------------------------------------------- | ---------------------------- |
| `name`                      | Proxy label, also used for port-override matching               | listen address               |
| `listen`                    | Bind address                                                    | `127.0.0.1:9797`             |
| `mode`                      | `"cache"` or `"token"`                                          | `"token"`                    |
| `api_key`                   | Upstream API key, overrides `[defaults]`                        | -                            |
| `api_url`                   | Upstream API base URL, overrides `[defaults]`                   | -                            |
| `model`                     | Model name, overrides `[defaults]`                              | -                            |
| `tool_relay`                | Enable the `/tool/relay` endpoint                               | `false`                      |
| `dev`                       | Verbose request/response logging                                | `false`                      |
| `ccr_ttl_seconds`           | CCR entry time-to-live                                          | `3600`                       |
| `ccr_db_path`               | SQLite path for the token proxy                                 | `~/.hermes/aphrodite/ccr.db` |
| `notify_url` / `notify_key` | Hermes callback URL + bearer token for CCR-create notifications | -                            |
| `timeout`                   | Upstream request timeout, seconds (clamped to 600)              | `300`                        |
| `max_context`               | Max context tokens                                              | `1,000,000`                  |
| `max_output`                | Max output tokens (must be less than `max_context`)             | `384,000`                    |

`[defaults]` accepts `api_url`, `model`, `ccr_ttl_seconds`, and `api_key`, and
applies to every `[[proxies]]` entry that doesn't override them.

## `[compression]`

This one section feeds two independent consumers with different levels of
"live":

**Drives the Rust proxy (`AppState`), including hot-reload** (env var wins
over these if set - see the precedence note above; editing these and
saving, or `POST /reload`, applies immediately with no restart):

| Field                  | Meaning                                                                        | Env override                     |
| ---------------------- | ------------------------------------------------------------------------------ | -------------------------------- |
| `tool_threshold_token` | Token-proxy (`:9798`) compression threshold, bytes                             | `APHRODITE_TOOL_THRESHOLD_TOKEN` |
| `tool_threshold_cache` | Cache-proxy (`:9797`) compression threshold, bytes                             | `APHRODITE_TOOL_THRESHOLD_CACHE` |
| `inline_threshold`     | Inline-vs-durable CCR storage cutoff, bytes                                    | `APHRODITE_INLINE_THRESHOLD`     |
| `code_multiplier`      | Multiplies the threshold for `code_*` content types (keeps code inline longer) | `APHRODITE_CODE_MULTIPLIER`      |

**Drives the Hermes-plugin dylib session** (`aphrodite-hermes`'s
`AphroditeState` - a separate process/codepath from the Rust proxy above,
read once at dylib load via `config_loader::Config`, not hot-reloaded):

| Field                | Meaning                                                                                 |
| -------------------- | --------------------------------------------------------------------------------------- |
| `terminal_threshold` | Terminal-output compression threshold, bytes - gates `hooks::transform_terminal_output` |

Also parsed into the Hermes-plugin session state and exposed via
`aphrodite_stats`/`aphrodite_config_get`, but **not consulted by any
compression decision** (populated, not load-bearing):
`engine_threshold_pct`, `engine_protect_first`, `engine_protect_last`,
`engine_min_msgs`.

**Not read by anything** (parsed by the TOML schema, echoed by `/reload`
for visibility, no consumer): `auto_expand`, `auto_expand_limit`,
`catalog_mode` (catalog mode has no env or TOML wiring at all - it's
whatever the caller passes per-request), `classifier_poll`, `context_engine`
(a _different_ `APHRODITE_CONTEXT_ENGINE` env var gates a real, unrelated
feature - see [`env-vars.md`](env-vars.md) for the disambiguation), and
`prefetch` (the shipped root `aphrodite.toml` sets it, but nothing reads it
back).

## `[previews]` and `[prompts]`

| Section      | Field                  | Meaning                                                                                        |
| ------------ | ---------------------- | ---------------------------------------------------------------------------------------------- |
| `[previews]` | `model_family`         | `"compact"` \| `"code_first"` \| `"balance"` - which preview template family to render         |
| `[previews]` | `code_structure_map`   | Include function/struct/class signatures in code previews                                      |
| `[previews]` | `preview_max_chars`    | Max characters per rendered preview line                                                       |
| `[previews]` | `rust_preview_lines`   | Lines of Rust source to include in `code_first`-family previews                                |
| `[prompts]`  | `retrieve_guidance`    | `"minimal"` \| `"standard"` \| `"verbose"` - how much the system prompt explains CCR retrieval |
| `[prompts]`  | `ccr_marker_hint`      | Append a retrieval hint after markers                                                          |
| `[prompts]`  | `catalog_intent_hints` | Show intent hints alongside hashes in catalog output                                           |

**Caveat**: unlike `[compression]`, these two sections don't currently
appear to have any effect on proxy behavior - the values parse cleanly but
nothing in the current codebase reads them back out. Don't assume changing
these values changes preview or prompt output; treat them as reserved until
that's confirmed.

## `[templates.*]`

The shipped root `aphrodite.toml` also ships a large
`[templates.preview.*]` / `[templates.marker]` / `[templates.prompts]` /
`[templates.reverse]` block of per-content-type format strings, with a
variable reference (`{type}`, `{ln}`, `{size}`, `{hash}`, `{fns}`, `{sigs}`,
etc.) in its header comment.

**Caveat**: like `[previews]`/`[prompts]`, this section doesn't currently
appear to be wired up to anything - preview strings are generated
internally rather than rendered from these templates. If you're editing
`[templates.*]` expecting it to change preview output, verify against your
running version first; treat it as reserved/aspirational rather than
functioning configuration.

## API key resolution

```
proxy.api_key
  → defaults.api_key
    → APHRODITE_API_KEY env var
      → DEEPSEEK_API_KEY env var
        → HEADROOM_DEEPSEEK_KEY env var
          → error: no API key configured
```

Stops at the first non-empty value.

## Default value chain

| Field                       | Resolution                                                                                                                        |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `listen`                    | `proxy.listen` → `127.0.0.1:9797`, then `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT` overrides just the port (name/mode-matched) |
| `mode`                      | `proxy.mode` → `"token"` (warns on unknown values) - no env override in multi-proxy mode                                          |
| `api_url`                   | `APHRODITE_API_URL` → `proxy.api_url` → `defaults.api_url` → `https://api.openai.com`                                             |
| `model`                     | `APHRODITE_MODEL` → `proxy.model` → `defaults.model` → `"default-model"`                                                          |
| `ccr_ttl_seconds`           | `APHRODITE_CCR_TTL` → `proxy.ccr_ttl_seconds` → `defaults.ccr_ttl_seconds` → `3600`                                               |
| `ccr_db_path`               | `APHRODITE_DB` → `proxy.ccr_db_path` (non-empty) → `~/.hermes/aphrodite/ccr.db` (or `/tmp` fallback)                              |
| `notify_url` / `notify_key` | `APHRODITE_NOTIFY_URL`/`APHRODITE_NOTIFY_KEY` → `proxy.notify_url`/`notify_key` → unset                                           |
| `tool_relay`                | `proxy.tool_relay` → `false`                                                                                                      |
| `dev`                       | `proxy.dev` → `false`                                                                                                             |
| `timeout`                   | `proxy.timeout` → `300` (clamped to a max of `600`)                                                                               |
| `max_context`               | `proxy.max_context` → `1,000,000`                                                                                                 |
| `max_output`                | `proxy.max_output` → `384,000`                                                                                                    |

## Hot-reload

`POST /reload` (per-listener) and the `aphrodite.toml` file watcher (all
listeners at once) both re-resolve the four live `[compression]` threshold
fields from the section above and write them into the running proxy's
state immediately - no restart needed. `/reload`'s JSON response has
`"applied": true` and echoes the values that took effect, plus a
`"parsed_only"` object for the fields that don't (see the `[compression]`
breakdown above). Everything else about a proxy's config (`listen`,
`api_url`, `model`, `mode`, ...) is fixed at startup; changing those
requires a restart.

## Validation

| Check                         | Behavior                                                                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `max_output` vs `max_context` | Refuses to start if `max_output >= max_context`                                                            |
| `listen` address              | Must parse as a valid socket address, or the proxy refuses to start                                        |
| API key                       | Must be non-empty after the full resolution chain, or the proxy refuses to start                           |
| `timeout`                     | Clamped to a 600-second maximum, with a warning if the configured value exceeds it                         |
| `mode`                        | Unknown values fall back to `"token"` with a warning; an unset `mode` also defaults to `"token"`, silently |

## Modes

| Mode      | Backend   | Threshold | Tool relay                  |
| --------- | --------- | --------- | --------------------------- |
| `"token"` | SQLite    | >1 KB     | Yes, aggressive compression |
| `"cache"` | In-memory | >8 KB     | No                          |

## Database path resolution

A relative `ccr_db_path` is resolved against the binary's own directory, not
the current working directory - so a relative path behaves consistently
regardless of where the proxy is launched from. Parent directories are
created automatically if missing.

## CLI equivalents

When running without `aphrodite.toml`, CLI flags mirror these fields:

| TOML field        | CLI flag        | Env var                 |
| ----------------- | --------------- | ----------------------- |
| `mode`            | `--mode`        | `APHRODITE_MODE`        |
| `listen`          | `--listen`      | `APHRODITE_LISTEN`      |
| `api_url`         | `--api-url`     | `APHRODITE_API_URL`     |
| `api_key`         | `--api-key`     | `APHRODITE_API_KEY`     |
| `model`           | `--model`       | `APHRODITE_MODEL`       |
| `ccr_db_path`     | `--ccr-db-path` | `APHRODITE_DB`          |
| `ccr_ttl_seconds` | `--ccr-ttl`     | `APHRODITE_CCR_TTL`     |
| `tool_relay`      | `--tool-relay`  | -                       |
| `notify_url`      | `--notify-url`  | `APHRODITE_NOTIFY_URL`  |
| `notify_key`      | `--notify-key`  | `APHRODITE_NOTIFY_KEY`  |
| `dev`             | `--dev`         | -                       |
| `log_compact`     | `--log-compact` | `APHRODITE_LOG_COMPACT` |
| `timeout`         | `--timeout`     | -                       |

## Example: full multi-proxy

```toml
[defaults]
api_url = "https://api.deepseek.com/v1"
model = "deepseek-v4-pro"
ccr_ttl_seconds = 7200

[[proxies]]
name = "cache"
listen = "127.0.0.1:9797"
mode = "cache"
ccr_ttl_seconds = 600

[[proxies]]
name = "token"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
notify_url = "https://hermes.internal/callback"
notify_key = "hermes-api-key-123"
timeout = 120
```
