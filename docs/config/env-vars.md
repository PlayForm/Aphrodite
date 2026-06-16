# Environment Variables

Origin: Both the Rust proxy and Python plugin are configured via environment variables for deployment flexibility. Critically, the API key chain allows multiple fallback env vars for different deployment contexts.

Source of truth: `crates/aphrodite/src/config.rs` (CLI args with env fallbacks), `crates/aphrodite/src/main.rs` (lines 28-34, 45, 57), `plugins/aphrodite/_core.py` (lines 20-48), `plugins/aphrodite/plugin.yaml` (line 29)

## Rust Proxy (Binary)

### API & Connection

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_API_KEY` | (required) | config.rs:143 | Primary API key for upstream LLM |
| `DEEPSEEK_API_KEY` | — | config.rs:144 | Fallback #1 |
| `HEADROOM_DEEPSEEK_KEY` | — | config.rs:145 | Fallback #2 |
| `APHRODITE_API_URL` | `https://api.openai.com` | config.rs:35 | Upstream API base URL |
| `APHRODITE_MODEL` | `default-model` | config.rs:43 | Model name to forward |

### Proxy Operation

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_MODE` | `token` | config.rs:27 | `cache` or `token` |
| `APHRODITE_LISTEN` | `127.0.0.1:9797` | config.rs:31 | Listen address |
| `APHRODITE_DB` | `~/.hermes/aphrodite/ccr.db` | config.rs:55 | SQLite database path |
| `APHRODITE_CCR_TTL` | `3600` | config.rs:59 | CCR entry TTL in seconds |
| `APHRODITE_CONFIG_PATH` | `aphrodite.toml` | main.rs:45 | Multi-proxy config path |
| `APHRODITE_WORKER_THREADS` | `4×CPU\|32` | main.rs:28 | Tokio worker thread count |
| `APHRODITE_LOG_COMPACT` | false (flag) | main.rs:57,69 | Compact log format (no timestamps/targets) |

### Notification Callbacks

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_NOTIFY_URL` | — | config.rs:71 | Hermes callback URL for CCR events |
| `APHRODITE_NOTIFY_KEY` | — | config.rs:75 | Bearer token for callback auth |

## Python Plugin (Hermes)

### Core

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_DEBUG` | `0` | _core.py:42 | Enable debug logging |
| `APHRODITE_CONTEXT_ENGINE` | (unset) | plugin.yaml:29 | Enable context engine (`=1`) |
| `APHRODITE_CATALOG` | `compact` | _core.py:43 | Catalog mode: `full`, `compact`, `tool` |
| `APHRODITE_PASSTHROUGH` | `0` | _core.py:48 | Disable all compression (dev mode) |
| `HERMES_DEV` | `0` | _core.py:48 | Alternative passthrough trigger |
| `QUIET` | `0` | _hooks.py:534 | Suppress catalog injection |

### Thresholds

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_ENGINE_THRESHOLD_PCT` | `50` | _core.py:27 | Context fill % to trigger engine compression. `-1`=always, `0`=disabled |
| `APHRODITE_ENGINE_PROTECT_FIRST` | `1` | _core.py:29 | Messages to protect at head |
| `APHRODITE_ENGINE_PROTECT_LAST` | `1` | _core.py:30 | Messages to protect at tail |
| `APHRODITE_ENGINE_MIN_MSGS` | `4` | _core.py:31 | Minimum messages before engine compresses |
| `APHRODITE_TOOL_THRESHOLD_TOKEN` | `1024` | _core.py:32 | Tool output compression when token proxy alive |
| `APHRODITE_TOOL_THRESHOLD_CACHE` | `8192` | _core.py:33 | Tool output compression when only cache proxy alive |
| `APHRODITE_TERMINAL_THRESHOLD` | `2048` | _core.py:34 | Terminal output compression threshold |
| `APHRODITE_INLINE_THRESHOLD` | `4096` | _core.py:35 | Inline fallback threshold (bumped to 1MB if `HEADROOM_SSE_BUFFER_MAX_BYTES` set) |

### Limits

| Variable | Default | Used In | Description |
|----------|---------|---------|-------------|
| `APHRODITE_RECURSIVE_DEPTH` | `3` | _core.py:40 | Max nested CCR resolution depth |
| `APHRODITE_AUTO_EXPAND_LIMIT` | `51200` | _core.py:41 | Max size for auto-expanding tool CCR markers |
| `APHRODITE_MAX_REQUEST_BODY_SIZE` | `104857600` (100MB) | _core.py:46 | Skip compression above this |
| `APHRODITE_RECENT_MARKERS_MAX` | `500` | _core.py:102 | Max markers in deque |
| `HEADROOM_SSE_BUFFER_MAX_BYTES` | — | _core.py:38 | If set, bumps `INLINE_THRESHOLD` to 1MB |

## Resolution Order

### API Key
```
APHRODITE_API_KEY → DEEPSEEK_API_KEY → HEADROOM_DEEPSEEK_KEY
```

### Passthrough
```
APHRODITE_PASSTHROUGH=1 OR HERMES_DEV=1 → plugin disabled
```

### Debug Logging
```
APHRODITE_DEBUG=1 → logging.DEBUG
```

## Recommended Production Settings

```bash
export APHRODITE_API_KEY="sk-..."
export APHRODITE_API_URL="https://api.deepseek.com/v1"
export APHRODITE_MODEL="deepseek-v4-pro"
export APHRODITE_MODE="token"
export APHRODITE_LISTEN="0.0.0.0:9798"      # Docker/Prometheus access
export APHRODITE_CCR_TTL="7200"
export APHRODITE_WORKER_THREADS="64"
export APHRODITE_NOTIFY_URL="https://..."
export APHRODITE_NOTIFY_KEY="..."            # if using callbacks
```

## Recommended Development Settings

```bash
export APHRODITE_DEBUG="1"
export APHRODITE_CATALOG="full"
export APHRODITE_ENGINE_THRESHOLD_PCT="50"
export APHRODITE_ENGINE_PROTECT_FIRST="4"
export APHRODITE_ENGINE_PROTECT_LAST="4"
```
