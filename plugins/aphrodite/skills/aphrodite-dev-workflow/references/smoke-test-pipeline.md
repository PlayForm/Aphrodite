# Aphrodite Smoke Test Pipeline

## Quick Reference

Run from Hermes TUI:
```
run aphrodite_test with mode=pipeline
```

Or from any Hermes session:
```
run aphrodite_test with mode=full
run aphrodite_test with mode=matrix
run aphrodite_test with mode=quick
```

## Test Modes

| Mode | Tests | What it does |
|------|-------|-------------|
| quick | 9 | compress, retrieve, stats, files, diff, proxy health |
| full | 13 | quick + large compress, search, threshold checks |
| matrix | 9 + sweep | quick + 15 threshold × protect combinations |
| pipeline | 9 + toggles | quick + 4 feature toggle snapshots + regression |

## Feature Toggles (pipeline mode)

Tests 4 configurations:
- `APHRODITE_DEBUG=1` / `APHRODITE_DEBUG=0`
- `APHRODITE_CONTEXT_ENGINE=1` / `APHRODITE_CONTEXT_ENGINE=0`

Each toggle snapshots: proxy_alive, thresholds (terminal, inline, tool_token, tool_cache), engine_threshold_pct.

## Regression Tracking

Pipeline mode saves results to `~/.hermes/plugins/aphrodite/.test-results.json`.
On next run, compares `previous_passed` vs `current_passed`.
If count drops → `DEGRADED` alert in results.

## Proxy Health Verification

```bash
curl http://127.0.0.1:9798/health   # {"status":"healthy","ccr":true,...}
curl http://127.0.0.1:9798/metrics  # Prometheus format
curl http://127.0.0.1:9798/stats    # Full JSON stats
curl http://127.0.0.1:9797/health   # Cache proxy
```

## Logging Commands

Clean proxy logging (no timestamps, INFO only):
```bash
APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

Full plugin decision logging with timing:
```bash
APHRODITE_DEBUG=1 hermes --provider custom:aphrodite-token
```

Filtered trace (aphrodite only, no hyper/rustls noise):
```bash
RUST_LOG=aphrodite=trace,hyper=off,rustls=off,reqwest=off
```
