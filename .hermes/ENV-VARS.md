# Aphrodite Environment Variable Reference

| Variable                          | Default            | Description                                                                                                                             |
| --------------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| **thresholds**                    |                    |                                                                                                                                         |
| `APHRODITE_TERMINAL_THRESHOLD`    | `2048`             | Minimum terminal output size (bytes) before CCR compression kicks in                                                                    |
| `APHRODITE_INLINE_THRESHOLD`      | `4096`             | Minimum output size (bytes) for inline (zlib) compression fallback                                                                      |
| `APHRODITE_TOOL_THRESHOLD_TOKEN`  | `1024`             | Minimum tool result size (bytes) for token-mode proxy compression                                                                       |
| `APHRODITE_TOOL_THRESHOLD_CACHE`  | `8192`             | Minimum tool result size (bytes) for cache-mode proxy compression                                                                       |
| **engine**                        |                    |                                                                                                                                         |
| `APHRODITE_CONTEXT_ENGINE`        | `""`               | Set to `1` to enable the Context Engine (message-level compression)                                                                     |
| `APHRODITE_ENGINE_THRESHOLD_PCT`  | `50`               | Engine compression threshold. `-1` = always compress, `0` = disabled, `>0` = compress when prompt_tokens >= context_length \* pct / 100 |
| `APHRODITE_ENGINE_PROTECT_FIRST`  | `1`                | Number of initial messages never compressed by engine                                                                                   |
| `APHRODITE_ENGINE_PROTECT_LAST`   | `1`                | Number of trailing messages never compressed by engine                                                                                  |
| `APHRODITE_ENGINE_MIN_MSGS`       | `4`                | Minimum messages before engine starts compressing                                                                                       |
| **coordination**                  |                    |                                                                                                                                         |
| `HEADROOM_SSE_BUFFER_MAX_BYTES`   | `""`               | When set, forces `APHRODITE_INLINE_THRESHOLD` to 1 MB so headroom's SSE buffer isn't overwhelmed                                        |
| `APHRODITE_MAX_REQUEST_BODY_SIZE` | `104857600`        | (100 MB) Skip compression entirely for payloads exceeding this size                                                                     |
| **mode / debug**                  |                    |                                                                                                                                         |
| `APHRODITE_PASSTHROUGH`           | `""`               | `1` = dev mode: plugin disabled, all hooks passthrough, no compression                                                                  |
| `HERMES_DEV`                      | `""`               | Same as `APHRODITE_PASSTHROUGH=1`                                                                                                       |
| `APHRODITE_DEBUG`                 | `""`               | `1` = enable DEBUG-level logging for the aphrodite plugin                                                                               |
| `APHRODITE_CATALOG`               | `"compact"`        | Catalog verbosity: `compact`, `full`, or `tool`                                                                                         |
| `APHRODITE_RECURSIVE_DEPTH`       | `3`                | Max recursive depth for marker expansion in retrieve                                                                                    |
| **binary / proxy**                |                    |                                                                                                                                         |
| `APHRODITE_LOG_COMPACT`           | `""`               | Set (any value) for compact proxy log output (no timestamps, no targets)                                                                |
| `APHRODITE_CONFIG_PATH`           | `"aphrodite.toml"` | Path to multi-proxy config file                                                                                                         |
| `APHRODITE_API_KEY`               | `""`               | API key passed to the proxy binary on startup (read lazily per-start)                                                                   |
| **ports**                         |                    |                                                                                                                                         |
| - cache proxy                     | `9797`             | Static - defined in `_core.PORTS`                                                                                                       |
| - token proxy                     | `9798`             | Static - defined in `_core.PORTS`                                                                                                       |

---

## Headroom-specific vars (consumed by headroom, not aphrodite)

| Variable                        | Description                                                    |
| ------------------------------- | -------------------------------------------------------------- |
| `HEADROOM_SSE_BUFFER_MAX_BYTES` | SSE buffer cap that drives the INLINE_THRESHOLD override above |

## Quick-start

```bash
# Minimal config for compression-aggressive profile
export APHRODITE_DEBUG=1
export APHRODITE_TERMINAL_THRESHOLD=512
export APHRODITE_TOOL_THRESHOLD_TOKEN=512
export APHRODITE_CATALOG=full

# Headroom-aware config
export HEADROOM_SSE_BUFFER_MAX_BYTES=1048576
# INLINE_THRESHOLD automatically bumps to 1 MB
```
