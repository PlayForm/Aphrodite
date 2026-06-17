---
name: aphrodite-proxy-lifecycle
description: Proxy version detection, auto-restart on session start, cache safety across restarts, and background process workflow patterns.
version: 1.0.0
---

# Aphrodite Proxy Lifecycle

## Version-Aware Auto-Restart

`_proxy/lifecycle.py:on_start()` queries `/health` for the running proxy version before skipping `_start()`. If the running version doesn't match `BIN_VERSION`, it kills the stale proxy and launches the new binary.

### Cache Safety
- **SQLite CCR store** survives restarts (disk-backed at `~/.hermes/aphrodite/ccr.db`)
- **In-memory cache** is rebuilt on next session start via `_restore_markers()`
- No compressed content is lost

### Version Sources
- `BIN_VERSION` in `_core/config.py` — plugin's expected binary version
- Running version from proxy `/health` JSON `"version"` field
- `PLUGIN_VERSION` — Python plugin version

## Background Process Workflow

Never block the session on long-running commands. Pattern:

```
# Dispatch build in background
terminal(background=true, notify_on_complete=true, command="cargo build --release -p aphrodite")

# Continue working...

# When notified, verify
terminal(command="curl -s http://localhost:9798/health")
```

- **Builds/tests >10s**: background + notify_on_complete
- **Servers/watchers**: background (no notify — never exit)
- **Progress**: `process(action='poll')` for non-blocking output checks
- **Verification**: foreground for fast checks after background completion

## References

- `references/background-process-workflow.md` — detailed pattern
- `references/proxy-version-auto-restart.md` — implementation details
