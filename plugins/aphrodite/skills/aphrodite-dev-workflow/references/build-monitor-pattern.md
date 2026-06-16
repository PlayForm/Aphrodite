# Build Monitor Pattern

## Problem

Every fix agent dispatched via `hermes -z` runs `cargo check -p aphrodite` to verify compilation — ~2s each. With 20+ agents, that's 40s of redundant compilation. Worse during parallel execution when 4 agents all run cargo check simultaneously, thrashing the build cache.

## Solution

A dedicated build monitor agent polls the cargo watch pane every 5s and writes a status file. Fix agents read this file instead of running cargo check.

## Architecture

```
Build monitor (hermes -z) → wezterm cli get-text --pane-id 17 → parse output → .hermes/build-status.json
                                    ↑
Fix agents → read .hermes/build-status.json → skip cargo check if status=ok
```

## Launch

```bash
# Monitor (one-time, long-running background process)
hermes -z 'Monitor cargo watch build status via wezterm cli. Every 5 seconds:
1. wezterm cli get-text --pane-id 17 --start-line -20 2>&1
2. Parse for: "Compiling aphrodite", "Finished", "error[", "could not compile"
3. Write .hermes/build-status.json: {"status":"ok"|"building"|"error","last_build":"ISO",...}
4. Sleep 5, repeat. Never run cargo or edit code.' --model deepseek-v4-flash
```

## Status File Format

```json
{
  "status": "ok",
  "last_build": "2026-06-16T09:56:32Z",
  "version": "v0.5.61",
  "errors": []
}
```

## Fix Agent Check

```bash
# First step in every fix agent:
cat .hermes/build-status.json 2>/dev/null || echo '{"status":"unknown"}'
# If status=ok, skip cargo check. If status=error, read errors and report.
```

## Caveats

- MCP tools are NOT available to subagents — use `wezterm cli` instead
- The monitor is a single-purpose agent — it does nothing else
- Pane 17 must have cargo watch running
- Build failures detected within 5-10s of compilation
