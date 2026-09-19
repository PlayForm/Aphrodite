# Environment Variables

Aphrodite reads a set of `APHRODITE_*` environment variables (plus a few non-prefixed ones) across three surfaces: the Rust proxy, the `aphrodite setup` subcommand, and the Hermes-plugin dylib session. This page lists every variable with a live reader, what precedence applies, and what it gates. Variables with no reader anywhere are listed at the bottom under "Documented but currently unwired" - setting those is a silent no-op.

Precedence rule: where it says "env > TOML > default", the env var wins if set and parses; otherwise the matching `aphrodite.toml` key wins if present; otherwise the compiled-in default applies. A present-but-malformed value (e.g. `APHRODITE_CCR_TTL=abc`) is never silently treated as absent - it logs a warning and falls through to the next precedence level. Boolean env vars share one truthiness rule everywhere: `"1"` / `"true"` (case-insensitive) is true; anything else present, or absent, is false.

## Rust proxy - multi-proxy mode (aphrodite.toml present)

Resolution lives in the multi-proxy config resolver; per-proxy TOML values beat `[defaults]`, and env beats both.

| Variable                         | Precedence                                                        | Default (no override)                            | Effect                                                                   |
| -------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------ |
| `APHRODITE_API_KEY`              | `proxy.api_key` → `defaults.api_key` → env                        | (must resolve to something)                      | Upstream API key; the proxy refuses to start if the chain resolves empty |
| `APHRODITE_API_URL`              | env > TOML > default                                              | `https://api.openai.com`                         | Upstream API base URL                                                    |
| `APHRODITE_MODEL`                | env > TOML > default                                              | `default-model`                                  | Model name to forward                                                    |
| `APHRODITE_CCR_TTL`              | env > TOML > default                                              | `3600`                                           | CCR entry time-to-live, seconds                                          |
| `APHRODITE_DB`                   | env > TOML                                                        | (none - proxy uses `~/.hermes/aphrodite/ccr.db`) | SQLite path for the token proxy                                          |
| `APHRODITE_NOTIFY_URL`           | env > TOML                                                        | -                                                | Hermes callback URL for CCR-create notifications                         |
| `APHRODITE_NOTIFY_KEY`           | env > TOML                                                        | -                                                | Bearer token for the callback                                            |
| `APHRODITE_CACHE_PORT`           | overrides the `listen` port on the proxy named/moded `cache` only | `9797`                                           | Per-instance port override (e.g. multiple Hermes Agents on one machine)  |
| `APHRODITE_TOKEN_PORT`           | overrides the `listen` port on the proxy named/moded `token` only | `9798`                                           | Same, for the token proxy                                                |
| `APHRODITE_TOOL_THRESHOLD_CACHE` | env > `[compression]` > const                                     | `8192` bytes (shipped example: `4096`)           | Cache-proxy compression threshold                                        |
| `APHRODITE_TOOL_THRESHOLD_TOKEN` | env > `[compression]` > const                                     | `1024` bytes (shipped example: `512`)            | Token-proxy compression threshold                                        |
| `APHRODITE_INLINE_THRESHOLD`     | env > `[compression]` > const                                     | `256` bytes (shipped example: `2048`)            | Inline-vs-durable CCR storage cutoff                                     |
| `APHRODITE_CODE_MULTIPLIER`      | env > `[compression]` > const                                     | `3.0`                                            | Multiplies the threshold for `code_*` content types                      |

`APHRODITE_MODE` / `APHRODITE_LISTEN` are deliberately **not** honored in multi-proxy mode - a single process-wide value would incorrectly apply to every `[[proxies]]` entry at once and break the cache/token split. Use per-proxy TOML `mode` / `listen` (or the two port vars above) instead; they work in CLI-fallback mode only.

The four threshold/multiplier vars are also what `POST /reload` and the config-file watcher re-resolve and apply live to a running proxy.

## Rust proxy - either mode

| Variable                   | Default                                                                      | Effect                                                                                                                                                                                                                                                                          |
| -------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `APHRODITE_CONFIG_PATH`    | `./aphrodite.toml`, else `~/.hermes/aphrodite/aphrodite.toml`, else CLI mode | Which config file to load; an explicit path that doesn't exist falls through to CLI mode                                                                                                                                                                                        |
| `APHRODITE_MGMT_TOKEN`     | unset (auth disabled - logs a startup warning)                               | Gates the management routes (`/stats`, `/stats/db`, `/history`, `/retrieve`, `/ccr/*`, `/reload`, `/tool/relay`, `/version`, `/health/upstream`) via `Authorization: Bearer <token>`; `/health` and `/metrics` stay exempt, and the LLM-proxying `/{*path}` route is unaffected |
| `APHRODITE_WORKER_THREADS` | 4x CPU count (min 32)                                                        | Tokio worker thread count                                                                                                                                                                                                                                                       |
| `APHRODITE_LOG_COMPACT`    | off                                                                          | Compact log format (no timestamps, no targets); also `--log-compact` in CLI mode                                                                                                                                                                                                |
| `RUST_LOG`                 | `info`                                                                       | Standard Rust `tracing` filter (not `APHRODITE_`-prefixed)                                                                                                                                                                                                                      |

## Rust proxy - CLI-fallback mode only (no aphrodite.toml present)

The env-attributed CLI fields above (`API_URL`, `MODEL`, `API_KEY`, `CCR_TTL`, `DB`, `NOTIFY_URL`, `NOTIFY_KEY`, `LOG_COMPACT`) all work here via clap directly, PLUS:

| Variable           | Default          | Effect                         |
| ------------------ | ---------------- | ------------------------------ |
| `APHRODITE_MODE`   | `token`          | Proxy mode (`cache` / `token`) |
| `APHRODITE_LISTEN` | `127.0.0.1:9797` | Bind address                   |

`max_context`, `max_output`, `tool_relay`, `dev`, `timeout`, and `no_ccr_marker` are flag-only in CLI mode - they have no env var.

## `aphrodite setup`

The setup subcommand accepts these as flags or env vars (clap `env` attributes):

| Variable               | Default | Effect                                   |
| ---------------------- | ------- | ---------------------------------------- |
| `APHRODITE_API_KEY`    | -       | Upstream API key                         |
| `APHRODITE_API_URL`    | (empty) | Upstream API base URL, written to config |
| `APHRODITE_MODEL`      | (empty) | Model name, written to config            |
| `APHRODITE_CACHE_PORT` | `9797`  | Cache proxy port in the generated config |
| `APHRODITE_TOKEN_PORT` | `9798`  | Token proxy port in the generated config |

## Hermes plugin / aphrodite-hermes dylib

The dylib initializes its session state from `aphrodite.toml` (searching `./aphrodite.toml` then `~/.hermes/aphrodite/aphrodite.toml`), with env vars on top - a **separate** resolution path from the Rust proxy above, feeding the Hermes hook/tool-dispatch session rather than the HTTP proxy. The Python plugin reads a few vars of its own at registration time.

| Variable                                        | Precedence                         | Default                                       | Effect                                                                                                                                                                                                                    |
| ----------------------------------------------- | ---------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `APHRODITE_TOOL_THRESHOLD_TOKEN`                | env > `[compression]` > default    | `4096` bytes                                  | Gates `transform_tool_result` compression (the dylib's single tool-output threshold)                                                                                                                                      |
| `APHRODITE_TERMINAL_THRESHOLD`                  | env > `[compression]` > default    | `1024` bytes                                  | Gates `transform_terminal_output` compression                                                                                                                                                                             |
| `APHRODITE_ENGINE_THRESHOLD_PCT`                | env > `[compression]` > default    | `45`                                          | Populated into session state, exposed via `aphrodite_stats` / `aphrodite_config_get` - **no hook branches on it**                                                                                                         |
| `APHRODITE_ENGINE_PROTECT_FIRST`                | env > `[compression]` > default    | `2`                                           | Same - status only                                                                                                                                                                                                        |
| `APHRODITE_ENGINE_PROTECT_LAST`                 | env > `[compression]` > default    | `5`                                           | Same - status only                                                                                                                                                                                                        |
| `APHRODITE_ENGINE_MIN_MSGS`                     | env > `[compression]` > default    | `8`                                           | Same - status only                                                                                                                                                                                                        |
| `APHRODITE_CONTEXT_ENGINE`                      | `"1"` / `"true"` enables           | off                                           | Two independent live effects: (1) plugin registers a Hermes `ContextEngine` subclass only when set; (2) the dylib session's `context_engine_enabled` status flag (env > TOML `compression.context_engine` > default true) |
| `APHRODITE_PREVIEW_MAX_CHARS`                   | env > `[previews]` > default       | unlimited (0 = no cap; shipped example `120`) | Caps the rendered preview string, enforced on every preview path                                                                                                                                                          |
| `APHRODITE_FLOW_BUDGET_CHARS`                   | env > `[flow]` > default           | `4000` (shipped example `2600`)               | Hard cap on ALL per-turn injected context (directives + nudges + recall catalog + retrieve hint)                                                                                                                          |
| `APHRODITE_POLL_WORKER`                         | env > `[compression]` > default    | `true`                                        | Auto-backgrounds slow `terminal` / `process` calls (pre-tool-call rewrite)                                                                                                                                                |
| `APHRODITE_CHAIN_SPLIT`                         | env > `[compression]` > default    | `false` (opt-in)                              | Fine-grained command splitting: rewrites chained commands with segment markers, splits output per segment                                                                                                                 |
| `APHRODITE_CHAIN_SPLIT_MIN_SEGMENTS`            | env > `[compression]` > default    | `2`                                           | Floor for the adaptive split threshold                                                                                                                                                                                    |
| `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS`            | env > `[compression]` > default    | `6`                                           | Cap for the adaptive split threshold                                                                                                                                                                                      |
| `APHRODITE_SESSION_INJECT`                      | env > `[prompts]` > default        | shipped builtin                               | First-turn orientation text injected on turn 0; empty string disables it                                                                                                                                                  |
| `APHRODITE_DIRECTIVES_DIR`                      | exact directory (first candidate)  | -                                             | Directives directory override; checked before `./directives/`, `~/.hermes/aphrodite/directives/`, binary-relative                                                                                                         |
| `APHRODITE_HOME`                                | overrides the runtime home         | `~/.hermes/aphrodite`                         | User-data home override: binaries, directives, `aphrodite.toml`, `ccr.db`, hot-reload cache                                                                                                                               |
| `APHRODITE_HERMES_DYLIB_PATH`                   | overrides the dylib search path    | `~/.hermes/aphrodite/binaries/<name>`         | Which dylib the plugin loads                                                                                                                                                                                              |
| `APHRODITE_BINARY_PATH`                         | overrides the proxy binary path    | `~/.hermes/aphrodite/binaries/aphrodite`      | Which proxy binary the plugin launches                                                                                                                                                                                    |
| `APHRODITE_NO_AUTO_LAUNCH`                      | `"1"` / `"true"` skips auto-launch | off                                           | Stops the plugin from launching the proxy on registration                                                                                                                                                                 |
| `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` | same as the Rust proxy table above | `9797` / `9798`                               | Proxy health-poll and the dylib's own port resolution; malformed values warn and fall back                                                                                                                                |

The plugin exports `APHRODITE_DIRECTIVES_DIR` (pointing at `~/.hermes/aphrodite/directives`) into the environment it manages, so the dylib picks up the plugin's directive set automatically.

## Documented but currently unwired

These names appear in older docs or scripts but have **no reader anywhere** in this codebase as of this writing - setting them is a silent no-op. Do not document them alongside the live vars above without this caveat; if one gets wired up, move its row into the tables above.

`APHRODITE_DEBUG`, `APHRODITE_PASSTHROUGH`, `HERMES_DEV`, `APHRODITE_AUTO_EXPAND`, `APHRODITE_AUTO_EXPAND_LIMIT`, `APHRODITE_LIVE_CONTAINER`, `APHRODITE_RECURSIVE_DEPTH`, `APHRODITE_MAX_REQUEST_BODY_SIZE`, `APHRODITE_RECENT_MARKERS_MAX`, `APHRODITE_CATALOG` (catalog mode is TOML-only, `compression.catalog_mode`, with no env override), `APHRODITE_TOOL_THRESHOLD` (no `_TOKEN` / `_CACHE` suffix - superseded by the two suffixed vars above), `HEADROOM_SSE_BUFFER_MAX_BYTES` (read only by the vendored `headroom` Python package this repo's binaries don't run).

## Build-time metadata

`APHRODITE_VERSION`, `APHRODITE_GIT_HASH`, `APHRODITE_PROFILE`, `APHRODITE_BUILD_DATE`, and `APHRODITE_TARGET` are embedded at compile time and reported by `--version` and the root `/` endpoint. They are read during the build, not at runtime - exporting them has no effect on a running proxy.
