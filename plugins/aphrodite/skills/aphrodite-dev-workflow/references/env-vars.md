# Aphrodite Env Var Reference

All thresholds are configurable via environment variables. Defaults shown.

## Engine thresholds

| Env Var | Default | Effect |
|---------|---------|--------|
| `APHRODITE_ENGINE_THRESHOLD_PCT` | 50 | Trigger at N% context. -1=always compress, 0=disabled, >0=fill% |
| `APHRODITE_ENGINE_PROTECT_FIRST` | 2 | Messages kept at head |
| `APHRODITE_ENGINE_PROTECT_LAST` | 5 | Messages kept at tail |
| `APHRODITE_ENGINE_MIN_MSGS` | 0 | Min messages before compress triggers |

## Compression thresholds

| Env Var | Default | Effect |
|---------|---------|--------|
| `APHRODITE_TOOL_THRESHOLD_TOKEN` | 1024 | Token mode min bytes to compress |
| `APHRODITE_TOOL_THRESHOLD_CACHE` | 8192 | Cache mode min bytes to compress |
| `APHRODITE_TERMINAL_THRESHOLD` | 2048 | Terminal output min bytes |
| `APHRODITE_INLINE_THRESHOLD` | 4096 | Inline fallback min bytes |

## Other

| Env Var | Default | Effect |
|---------|---------|--------|
| `APHRODITE_RECURSIVE_DEPTH` | 3 | Max nested CCR resolution depth |
| `APHRODITE_DEBUG` | unset | Set to 1 for verbose debug logging |
| `APHRODITE_PASSTHROUGH` | unset | Set to 1 to disable all proxy routing |
