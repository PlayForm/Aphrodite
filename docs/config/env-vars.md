# Environment Variables

Both the Rust proxy and Python plugin are configured via environment variables
for deployment flexibility. The API key chain allows multiple fallback env
vars for different deployment contexts, and all defaults can also be set in
`aphrodite.toml` under `[compression]` (priority: env var > `aphrodite.toml` >
hardcoded default).

## Rust Proxy (Binary)

### API & Connection

| Variable                | Default                  | Description                       |
| ------------------------ | ------------------------- | ----------------------------------- |
| `APHRODITE_API_KEY`     | (required)               | Primary API key for upstream LLM  |
| `DEEPSEEK_API_KEY`      | -                        | Fallback #1                       |
| `HEADROOM_DEEPSEEK_KEY` | -                        | Fallback #2                       |
| `APHRODITE_API_URL`     | `https://api.openai.com` | Upstream API base URL             |
| `APHRODITE_MODEL`       | `default-model`          | Model name to forward             |

### Proxy Operation

| Variable                   | Default                      | Description                                  |
| ---------------------------- | ------------------------------ | ----------------------------------------------- |
| `APHRODITE_MODE`           | `token`                      | `cache` or `token`                           |
| `APHRODITE_LISTEN`         | `127.0.0.1:9797`             | Listen address                               |
| `APHRODITE_DB`             | `~/.hermes/aphrodite/ccr.db` | SQLite database path                         |
| `APHRODITE_CCR_TTL`        | `3600`                       | CCR entry TTL in seconds                     |
| `APHRODITE_CONFIG_PATH`    | `aphrodite.toml`             | Multi-proxy config path                      |
| `APHRODITE_WORKER_THREADS` | `4x CPU count` (max `32`)    | Tokio worker thread count                    |
| `APHRODITE_LOG_COMPACT`    | false (flag)                 | Compact log format (no timestamps/targets)   |

### Notification Callbacks

| Variable                | Default | Description                        |
| ------------------------- | --------- | ------------------------------------- |
| `APHRODITE_NOTIFY_URL` | -       | Hermes callback URL for CCR events  |
| `APHRODITE_NOTIFY_KEY` | -       | Bearer token for callback auth     |

## Python Plugin (Hermes)

### Core

| Variable                      | Default   | Description                                  |
| -------------------------------- | ----------- | ----------------------------------------------- |
| `APHRODITE_DEBUG`             | `0`       | Enable debug logging                         |
| `APHRODITE_CONTEXT_ENGINE`    | (unset)   | Enable context engine (`=1` overrides TOML)  |
| `APHRODITE_CATALOG`           | `compact` | Catalog mode: `full`, `compact`, `tool`      |
| `APHRODITE_PASSTHROUGH`       | `0`       | Disable all compression (dev mode)           |
| `HERMES_DEV`                  | `0`       | Alternative passthrough trigger              |
| `APHRODITE_AUTO_EXPAND`       | `0`       | Enable aggressive auto-expand (`=1`)         |
| `APHRODITE_AUTO_EXPAND_LIMIT` | `0`       | Byte limit for auto-expand (0=off)           |
| `APHRODITE_LIVE_CONTAINER`    | `0`       | Wrap `read_file` results in CCR markers      |

### Thresholds

| Variable                          | Default       | Description                                                              |
| ------------------------------------ | --------------- | ---------------------------------------------------------------------------- |
| `APHRODITE_ENGINE_THRESHOLD_PCT`  | `45`          | Context fill % to trigger engine compression. `-1`=always, `0`=disabled  |
| `APHRODITE_ENGINE_PROTECT_FIRST`  | `2`           | Messages to protect at head                                              |
| `APHRODITE_ENGINE_PROTECT_LAST`   | `5`           | Messages to protect at tail                                              |
| `APHRODITE_ENGINE_MIN_MSGS`       | `8`           | Minimum messages before engine compresses                                |
| `APHRODITE_TOOL_THRESHOLD_TOKEN`  | `512`         | Tool output compression (token proxy)                                    |
| `APHRODITE_TOOL_THRESHOLD_CACHE`  | `4096`        | Tool output compression (cache proxy)                                    |
| `APHRODITE_TERMINAL_THRESHOLD`    | `1024`        | Terminal output threshold (bytes)                                        |
| `APHRODITE_INLINE_THRESHOLD`      | `2048`        | Inline zlib fallback threshold (bytes)                                   |
| `APHRODITE_CODE_MULTIPLIER`       | `3.0`         | Threshold multiplier for code types                                      |
| `APHRODITE_RECURSIVE_DEPTH`       | `3`           | Max nesting depth for resolve                                            |
| `APHRODITE_MAX_REQUEST_BODY_SIZE` | `104_857_600` | Max request body (bytes)                                                 |
| `APHRODITE_RECENT_MARKERS_MAX`    | `500`         | Max recent markers deque size                                            |
