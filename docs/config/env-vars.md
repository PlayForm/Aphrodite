# Environment Variables

This page lists every `APHRODITE_*` (and the few non-prefixed) env vars that
have a live reader somewhere in this repo, with the exact file:line and what
precedence rule applies. Vars with no reader are listed separately at the
bottom under "Documented but currently unwired" - setting them has no effect.

Precedence, where it says "env > TOML > default": the env var wins if set
and parses; otherwise the matching `aphrodite.toml` key wins if present;
otherwise the compiled-in default applies. A present-but-malformed value
(e.g. `APHRODITE_CCR_TTL=abc`) is never silently treated as absent - it logs
a warning and falls through to the next precedence level, the same rule
`MultiConfig::resolve`'s port overrides have always used
(`crates/aphrodite/src/config.rs::apply_port_override`).

## Rust proxy - multi-proxy mode (`aphrodite.toml` present)

Config resolution lives in `MultiConfig::resolve()`
(`crates/aphrodite/src/config.rs:270-360`).

| Variable                         | Precedence                                                        | Default (no override)                             | Reader                                          |
| -------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------- |
| `APHRODITE_API_KEY`              | env > TOML > required                                             | (must resolve to something)                       | `config.rs:270-278` (fallback chain, see below) |
| `DEEPSEEK_API_KEY`               | fallback #2                                                       | -                                                 | `config.rs:276`                                 |
| `HEADROOM_DEEPSEEK_KEY`          | fallback #3                                                       | -                                                 | `config.rs:277`                                 |
| `APHRODITE_API_URL`              | env > TOML > default                                              | `https://api.openai.com`                          | `config.rs:317-321`                             |
| `APHRODITE_MODEL`                | env > TOML > default                                              | `default-model`                                   | `config.rs:322-326`                             |
| `APHRODITE_CCR_TTL`              | env > TOML > default                                              | `3600`                                            | `config.rs:333-336`                             |
| `APHRODITE_DB`                   | env > TOML                                                        | (none - proxy picks `~/.hermes/aphrodite/ccr.db`) | `config.rs:330-332`                             |
| `APHRODITE_NOTIFY_URL`           | env > TOML                                                        | -                                                 | `config.rs:339`                                 |
| `APHRODITE_NOTIFY_KEY`           | env > TOML                                                        | -                                                 | `config.rs:340`                                 |
| `APHRODITE_CACHE_PORT`           | overrides the `listen` port on the proxy named/moded `cache` only | `9797`                                            | `config.rs:281-286`                             |
| `APHRODITE_TOKEN_PORT`           | overrides the `listen` port on the proxy named/moded `token` only | `9798`                                            | `config.rs:281-286`                             |
| `APHRODITE_TOOL_THRESHOLD_CACHE` | env > `[compression]` > const                                     | `8192` bytes                                      | `proxy.rs::resolve_thresholds`                  |
| `APHRODITE_TOOL_THRESHOLD_TOKEN` | env > `[compression]` > const                                     | `1024` bytes                                      | `proxy.rs::resolve_thresholds`                  |
| `APHRODITE_INLINE_THRESHOLD`     | env > `[compression]` > const                                     | `256` bytes                                       | `proxy.rs::resolve_thresholds`                  |
| `APHRODITE_CODE_MULTIPLIER`      | env > `[compression]` > const                                     | `3.0`                                             | `proxy.rs::resolve_thresholds`                  |

`APHRODITE_MODE`/`APHRODITE_LISTEN` are deliberately **not** honored here -
a single process-wide value would incorrectly apply to every `[[proxies]]`
entry at once and break the cache/token split; use TOML per-proxy
`mode`/`listen` (or the two port vars above) instead.

The four threshold/multiplier vars are also what `POST /reload` and the
config-file watcher re-resolve and apply live to a running proxy - see
"Hot-reload" below.

## Rust proxy - either mode

| Variable                   | Default                                                                      | Reader                                                                                                 |
| -------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `APHRODITE_CONFIG_PATH`    | `./aphrodite.toml`, else `~/.hermes/aphrodite/aphrodite.toml`, else CLI mode | `main.rs::run`                                                                                         |
| `APHRODITE_WORKER_THREADS` | `4× CPU count` (min `32`)                                                    | `main.rs::main`                                                                                        |
| `APHRODITE_LOG_COMPACT`    | off                                                                          | `main.rs::run` via `config::env_bool` (`"1"`/`"true"` case-insensitive; NOT presence-only)             |
| `RUST_LOG`                 | `info`                                                                       | `main.rs::run` (`tracing_subscriber::EnvFilter`) - standard Rust convention, not `APHRODITE_`-prefixed |

## Rust proxy - CLI-fallback mode only (no `aphrodite.toml` present)

Every field on `Cli` (`config.rs:104-176`) is a clap arg with an `env = "..."`
attribute, so all of the multi-proxy-mode vars above also work here via clap
directly, PLUS:

| Variable           | Default          |
| ------------------ | ---------------- |
| `APHRODITE_MODE`   | `token`          |
| `APHRODITE_LISTEN` | `127.0.0.1:9797` |

## Hot-reload

`POST /reload` and the `aphrodite.toml` file watcher both call the same
`resolve_thresholds()` used at startup and write the result into the live
`AppState` - editing `[compression]`'s `tool_threshold_token`,
`tool_threshold_cache`, `inline_threshold`, or `code_multiplier` and either
saving the file or `curl -X POST :PORT/reload` takes effect immediately, no
restart. Every other `[compression]` key (`engine_threshold_pct`,
`catalog_mode`, `auto_expand*`, `terminal_threshold`) is parsed and echoed
back by `/reload` for visibility but has no effect on the Rust proxy - see
the Hermes-plugin section below for where `engine_*`/`terminal_threshold`
actually apply.

## Python plugin / `aphrodite-hermes` dylib

The dylib initializes its session state from `aphrodite.toml` via
`config_loader::Config` (`crates/aphrodite/src/config_loader.rs`), searching
`./aphrodite.toml` then `~/.hermes/aphrodite/aphrodite.toml`. This is a
**separate** resolution path from the Rust proxy above - it feeds
`AphroditeState` (the Hermes hook/tool-dispatch session), not `AppState`
(the HTTP proxy).

| Variable                                        | Precedence                                         | Default                           | Reader                                                                                                                                                                                                                                                                                                                        | Actually gates behavior?                                                                                                                   |
| ----------------------------------------------- | -------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `APHRODITE_TOOL_THRESHOLD_TOKEN`                | env > `compression.tool_threshold_token` > default | `4096` bytes                      | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | yes - `hooks::transform_tool_result`                                                                                                       |
| `APHRODITE_TERMINAL_THRESHOLD`                  | env > `compression.terminal_threshold` > default   | `1024` bytes                      | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | yes - `hooks::transform_terminal_output`                                                                                                   |
| `APHRODITE_ENGINE_THRESHOLD_PCT`                | env > `compression.engine_threshold_pct` > default | `45`                              | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | **no** - populates `AphroditeState.engine_threshold_pct`, exposed via `aphrodite_stats`/`aphrodite_config_get`, but no hook branches on it |
| `APHRODITE_ENGINE_PROTECT_FIRST`                | same pattern                                       | `2`                               | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | **no** - same as above                                                                                                                     |
| `APHRODITE_ENGINE_PROTECT_LAST`                 | same pattern                                       | `5`                               | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | **no** - same as above                                                                                                                     |
| `APHRODITE_ENGINE_MIN_MSGS`                     | same pattern                                       | `8`                               | `config_loader.rs::apply_compression`                                                                                                                                                                                                                                                                                         | **no** - same as above                                                                                                                     |
| `APHRODITE_CONTEXT_ENGINE`                      | `"1"`/`"true"` (case-insensitive) enables          | off                               | Two independent effects, both live: (1) `plugins/aphrodite/__init__.py::register` gates whether a Hermes `ContextEngine` subclass is registered at all (opt-in feature); (2) `config_loader.rs::apply_compression` also sets `AphroditeState.context_engine_enabled` from the same var name/TOML key - don't confuse the two. |
| `APHRODITE_HERMES_DYLIB_PATH`                   | overrides the dylib search path                    | `<plugin dir>/binaries/<name>`    | `plugins/aphrodite/__init__.py::_load_dylib`                                                                                                                                                                                                                                                                                  |
| `APHRODITE_BINARY_PATH`                         | overrides the proxy binary path                    | `<plugin dir>/binaries/aphrodite` | `plugins/aphrodite/__init__.py`                                                                                                                                                                                                                                                                                               |
| `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` | same as the Rust proxy table above                 | `9797`/`9798`                     | health-poll + dylib's own `configured_ports()` (`aphrodite-hermes/src/lib.rs`) - malformed values warn (Python: `_log.warning`; Rust: `eprintln!`, this dylib has no tracing subscriber) and fall back                                                                                                                        |
| `APHRODITE_NO_AUTO_LAUNCH`                      | `"1"`/`"true"` skips proxy auto-launch             | off                               | `plugins/aphrodite/__init__.py::_start_proxy` (both the monorepo plugin and the `cargo install`-embedded copy - they're now the same file, see `setup.rs::HERMES_PLUGIN_SHIM`)                                                                                                                                                |

## Documented but currently unwired

These names appear in older docs/scripts but have **no reader anywhere in
this repo** as of this writing - setting them is a silent no-op. Do not
document them alongside the live vars above without this caveat; if you wire
one up, move its row into the tables above.

`APHRODITE_DEBUG`, `APHRODITE_PASSTHROUGH`, `HERMES_DEV`,
`APHRODITE_AUTO_EXPAND`, `APHRODITE_AUTO_EXPAND_LIMIT`,
`APHRODITE_LIVE_CONTAINER`, `APHRODITE_RECURSIVE_DEPTH`,
`APHRODITE_MAX_REQUEST_BODY_SIZE`, `APHRODITE_RECENT_MARKERS_MAX`,
`APHRODITE_CATALOG` (catalog mode is TOML-only, `compression.catalog_mode`,
with no env override), `APHRODITE_TOOL_THRESHOLD` (no `_TOKEN`/`_CACHE`
suffix - superseded by the two suffixed vars above),
`HEADROOM_SSE_BUFFER_MAX_BYTES` (read only by the vendored `headroom`
Python package this repo's binaries don't run).
