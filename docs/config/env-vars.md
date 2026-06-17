# Environment Variables

Origin: Both the Rust proxy and Python plugin are configured via environment
variables for deployment flexibility. Critically, the API key chain allows
multiple fallback env vars for different deployment contexts.

Source of truth: `crates/aphrodite/src/config.rs` (CLI args with env fallbacks),
`crates/aphrodite/src/main.rs`, `plugins/aphrodite/_core/config.py` (config
loaders + defaults), `plugins/aphrodite/aphrodite.toml` (TOML-driven defaults)

## Rust Proxy (Binary)

### API & Connection

| Variable                | Default                  | Used In   | Description                      |
| ----------------------- | ------------------------ | --------- | -------------------------------- |
| `APHRODITE_API_KEY`     | (required)               | config.rs | Primary API key for upstream LLM |
| `DEEPSEEK_API_KEY`      | -                        | config.rs | Fallback #1                      |
| `HEADROOM_DEEPSEEK_KEY` | -                        | config.rs | Fallback #2                      |
| `APHRODITE_API_URL`     | `https://api.openai.com` | config.rs | Upstream API base URL            |
| `APHRODITE_MODEL`       | `default-model`          | config.rs | Model name to forward            |

### Proxy Operation

| Variable                   | Default                      | Used In   | Description                                |
| -------------------------- | ---------------------------- | --------- | ------------------------------------------ | ------------------------- |
| `APHRODITE_MODE`           | `token`                      | config.rs | `cache` or `token`                         |
| `APHRODITE_LISTEN`         | `127.0.0.1:9797`             | config.rs | Listen address                             |
| `APHRODITE_DB`             | `~/.hermes/aphrodite/ccr.db` | config.rs | SQLite database path                       |
| `APHRODITE_CCR_TTL`        | `3600`                       | config.rs | CCR entry TTL in seconds                   |
| `APHRODITE_CONFIG_PATH`    | `aphrodite.toml`             | main.rs   | Multi-proxy config path                    |
| `APHRODITE_WORKER_THREADS` | `4×CPU                       | 32`       | main.rs                                    | Tokio worker thread count |
| `APHRODITE_LOG_COMPACT`    | false (flag)                 | main.rs   | Compact log format (no timestamps/targets) |

### Notification Callbacks

| Variable               | Default | Used In   | Description                        |
| ---------------------- | ------- | --------- | ---------------------------------- |
| `APHRODITE_NOTIFY_URL` | -       | config.rs | Hermes callback URL for CCR events |
| `APHRODITE_NOTIFY_KEY` | -       | config.rs | Bearer token for callback auth     |

## Python Plugin (Hermes)

### Core

| Variable                      | Default   | Used In   | Description                                 |
| ----------------------------- | --------- | --------- | ------------------------------------------- |
| `APHRODITE_DEBUG`             | `0`       | config.py | Enable debug logging                        |
| `APHRODITE_CONTEXT_ENGINE`    | (unset)   | config.py | Enable context engine (`=1` overrides TOML) |
| `APHRODITE_CATALOG`           | `compact` | config.py | Catalog mode: `full`, `compact`, `tool`     |
| `APHRODITE_PASSTHROUGH`       | `0`       | config.py | Disable all compression (dev mode)          |
| `HERMES_DEV`                  | `0`       | config.py | Alternative passthrough trigger             |
| `APHRODITE_AUTO_EXPAND`       | `0`       | config.py | Enable aggressive auto-expand (`=1`)        |
| `APHRODITE_AUTO_EXPAND_LIMIT` | `0`       | config.py | Byte limit for auto-expand (0=off)          |
| `APHRODITE_LIVE_CONTAINER`    | `0`       | live.py   | Wrap `read_file` results in CCR markers     |

### Thresholds

| Variable                          | Default       | Used In   | Description                                                             |
| --------------------------------- | ------------- | --------- | ----------------------------------------------------------------------- |
| `APHRODITE_ENGINE_THRESHOLD_PCT`  | `45`          | config.py | Context fill % to trigger engine compression. `-1`=always, `0`=disabled |
| `APHRODITE_ENGINE_PROTECT_FIRST`  | `2`           | config.py | Messages to protect at head                                             |
| `APHRODITE_ENGINE_PROTECT_LAST`   | `5`           | config.py | Messages to protect at tail                                             |
| `APHRODITE_ENGINE_MIN_MSGS`       | `8`           | config.py | Minimum messages before engine compresses                               |
| `APHRODITE_TOOL_THRESHOLD_TOKEN`  | `512`         | config.py | Tool output compression (token proxy)                                   |
| `APHRODITE_TOOL_THRESHOLD_CACHE`  | `4096`        | config.py | Tool output compression (cache proxy)                                   |
| `APHRODITE_TERMINAL_THRESHOLD`    | `1024`        | config.py | Terminal output threshold (bytes)                                       |
| `APHRODITE_INLINE_THRESHOLD`      | `2048`        | config.py | Inline zlib fallback threshold (bytes)                                  |
| `APHRODITE_CODE_MULTIPLIER`       | `3.0`         | config.py | Threshold multiplier for code types                                     |
| `APHRODITE_RECURSIVE_DEPTH`       | `3`           | config.py | Max nesting depth for resolve                                           |
| `APHRODITE_MAX_REQUEST_BODY_SIZE` | `104_857_600` | config.py | Max request body (bytes)                                                |
| `APHRODITE_RECENT_MARKERS_MAX`    | `500`         | config.py | Max recent markers deque size                                           |

All defaults can also be set in `aphrodite.toml` under `[compression]`.
Priority: env var > aphrodite.toml > hardcoded default.
