# aphrodite.toml Configuration

Multi-proxy deployments (cache + token, or multiple token instances) are
configured through a single TOML file. Settings resolve in this order: TOML →
CLI flags → environment variables → built-in defaults.

This file is Aphrodite's own proxy/engine config - a **different file** from
Hermes Agent's `config.yaml`. See
[Troubleshooting: two separate config files](../install/troubleshooting.md#two-separate-config-files)
if you came here looking for Hermes-side keys like `plugins.enabled` or
`context.engine` instead.

## File location

| | |
| --- | --- |
| Default | `aphrodite.toml` in the current working directory |
| Override | `APHRODITE_CONFIG_PATH` environment variable |
| If missing | Falls back to CLI-flag mode (single proxy, see [CLI Equivalents](#cli-equivalents)) |

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

| Field | Meaning | Default |
| --- | --- | --- |
| `name` | Proxy label, also used for port-override matching | listen address |
| `listen` | Bind address | `127.0.0.1:9797` |
| `mode` | `"cache"` or `"token"` | `"token"` |
| `api_key` | Upstream API key, overrides `[defaults]` | - |
| `api_url` | Upstream API base URL, overrides `[defaults]` | - |
| `model` | Model name, overrides `[defaults]` | - |
| `tool_relay` | Enable the `/tool/relay` endpoint | `false` |
| `dev` | Verbose request/response logging | `false` |
| `ccr_ttl_seconds` | CCR entry time-to-live | `3600` |
| `ccr_db_path` | SQLite path for the token proxy | `~/.hermes/aphrodite/ccr.db` |
| `notify_url` / `notify_key` | Hermes callback URL + bearer token for CCR-create notifications | - |
| `timeout` | Upstream request timeout, seconds (clamped to 600) | `300` |
| `max_context` | Max context tokens | `1,000,000` |
| `max_output` | Max output tokens (must be less than `max_context`) | `384,000` |

`[defaults]` accepts `api_url`, `model`, `ccr_ttl_seconds`, and `api_key`, and
applies to every `[[proxies]]` entry that doesn't override them.

## `[compression]`

This section actually drives proxy behavior:

| Field | Meaning |
| --- | --- |
| `engine_threshold_pct` | Context engine activates once session fill reaches this % |
| `engine_protect_first` | Messages kept untouched at the start of the conversation |
| `engine_protect_last` | Messages kept untouched at the end of the conversation |
| `engine_min_msgs` | Minimum message count before the engine may activate |
| `tool_threshold_token` | Token-proxy (`:9798`) compression threshold, bytes |
| `tool_threshold_cache` | Cache-proxy (`:9797`) compression threshold, bytes |
| `terminal_threshold` | Terminal output compression threshold, bytes |
| `inline_threshold` | Inline zlib-fallback threshold, bytes |
| `auto_expand` | If `true`, tool output stays raw (no CCR markers) |
| `auto_expand_limit` | Byte cap for auto-expand; `0` disables the cap |
| `catalog_mode` | `"compact"` \| `"full"` \| `"tool"` |
| `classifier_poll` | Skip CCR entirely for clean/short outputs |
| `code_multiplier` | Multiplies the threshold for `code_*` content types (keeps code inline longer) |
| `context_engine` | Turns the context engine on/off (default on) |

The shipped root `aphrodite.toml` also sets `prefetch` in this section, but
it doesn't currently appear to change proxy behavior - treat it as reserved
rather than load-bearing.

## `[previews]` and `[prompts]`

| Section | Field | Meaning |
| --- | --- | --- |
| `[previews]` | `model_family` | `"compact"` \| `"code_first"` \| `"balance"` - which preview template family to render |
| `[previews]` | `code_structure_map` | Include function/struct/class signatures in code previews |
| `[previews]` | `preview_max_chars` | Max characters per rendered preview line |
| `[previews]` | `rust_preview_lines` | Lines of Rust source to include in `code_first`-family previews |
| `[prompts]` | `retrieve_guidance` | `"minimal"` \| `"standard"` \| `"verbose"` - how much the system prompt explains CCR retrieval |
| `[prompts]` | `ccr_marker_hint` | Append a retrieval hint after markers |
| `[prompts]` | `catalog_intent_hints` | Show intent hints alongside hashes in catalog output |

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

| Field | Resolution |
| --- | --- |
| `listen` | `proxy.listen` → `127.0.0.1:9797` |
| `mode` | `proxy.mode` → `"token"` (warns on unknown values) |
| `api_url` | `proxy.api_url` → `defaults.api_url` → `https://api.openai.com` |
| `model` | `proxy.model` → `defaults.model` → `"default-model"` |
| `ccr_ttl_seconds` | `proxy.ccr_ttl_seconds` → `defaults.ccr_ttl_seconds` → `3600` |
| `ccr_db_path` | `proxy.ccr_db_path` (non-empty) → `~/.hermes/aphrodite/ccr.db` (or `/tmp` fallback) |
| `tool_relay` | `proxy.tool_relay` → `false` |
| `dev` | `proxy.dev` → `false` |
| `timeout` | `proxy.timeout` → `300` (clamped to a max of `600`) |
| `max_context` | `proxy.max_context` → `1,000,000` |
| `max_output` | `proxy.max_output` → `384,000` |

## Validation

| Check | Behavior |
| --- | --- |
| `max_output` vs `max_context` | Refuses to start if `max_output >= max_context` |
| `listen` address | Must parse as a valid socket address, or the proxy refuses to start |
| API key | Must be non-empty after the full resolution chain, or the proxy refuses to start |
| `timeout` | Clamped to a 600-second maximum, with a warning if the configured value exceeds it |
| `mode` | Unknown values fall back to `"token"` with a warning; an unset `mode` also defaults to `"token"`, silently |

## Modes

| Mode | Backend | Threshold | Tool relay |
| --- | --- | --- | --- |
| `"token"` | SQLite | >1 KB | Yes, aggressive compression |
| `"cache"` | In-memory | >8 KB | No |

## Database path resolution

A relative `ccr_db_path` is resolved against the binary's own directory, not
the current working directory - so a relative path behaves consistently
regardless of where the proxy is launched from. Parent directories are
created automatically if missing.

## CLI equivalents

When running without `aphrodite.toml`, CLI flags mirror these fields:

| TOML field | CLI flag | Env var |
| --- | --- | --- |
| `mode` | `--mode` | `APHRODITE_MODE` |
| `listen` | `--listen` | `APHRODITE_LISTEN` |
| `api_url` | `--api-url` | `APHRODITE_API_URL` |
| `api_key` | `--api-key` | `APHRODITE_API_KEY` |
| `model` | `--model` | `APHRODITE_MODEL` |
| `ccr_db_path` | `--ccr-db-path` | `APHRODITE_DB` |
| `ccr_ttl_seconds` | `--ccr-ttl` | `APHRODITE_CCR_TTL` |
| `tool_relay` | `--tool-relay` | - |
| `notify_url` | `--notify-url` | `APHRODITE_NOTIFY_URL` |
| `notify_key` | `--notify-key` | `APHRODITE_NOTIFY_KEY` |
| `dev` | `--dev` | - |
| `log_compact` | `--log-compact` | `APHRODITE_LOG_COMPACT` |
| `timeout` | `--timeout` | - |

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
