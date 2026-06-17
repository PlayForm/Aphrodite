# Aphrodite Debug Startup Banner

The debug banner is emitted by `plugins/aphrodite/__init__.py::register()` (lines 1306-1323)
when `APHRODITE_DEBUG=1`. It uses `_log.info()` which goes to `~/.hermes/logs/agent.log`,
NOT the Hermes TUI terminal.

## What It Looks Like

```
APHRODITE v1.49.0 - DEBUG MODE
  Mode: proxy+hooks | Engine: on/off (TOML: [compression].context_engine) | Dev: off
  Thresholds: terminal=2048 inline=4096 tool_token=1024 tool_cache=8192
  Engine: threshold=50% protect=2/5 min_msgs=30
  CCR: regex=<<<CCR:([^>]+)>>> depth=3
  Tools: retrieve, compress, stats, rebuild, files, diff, search, test
  Proxies: cache=:9797 token=:9798 | waiting for session_start...
============================================================
```

## Fields

| Field | Source | Description |
|-------|--------|-------------|
| Version | `PLUGIN_VERSION` constant | Plugin version (e.g. 1.49.0) |
| Mode | `engine_configured` env check | `proxy+hooks` (default) or `proxy+hooks+engine` |
| Engine | `[compression].context_engine` TOML | Whether context compression engine is enabled (default-on) |
| Dev | `APHRODITE_PASSTHROUGH` env | Passthrough mode - disables all proxy routing |
| Thresholds | `_cfg_int` env vars | terminal, inline, tool_token, tool_cache |
| Engine params | `_cfg_int` env vars | threshold%, protect_first, protect_last, min_msgs |
| CCR | `_CCR_RE` pattern + `RECURSIVE_DEPTH` | Regex for marker parsing, recursive depth |
| Tools | Hardcoded list | 8 registered tools |
| Proxies | Hardcoded ports | cache (:9797) and token (:9798) |

## How to Read

```bash
# Find the latest debug banner
grep "APHRODITE.*DEBUG MODE" ~/.hermes/logs/agent.log | tail -1

# Show full banner with surrounding context
grep -A10 "APHRODITE.*DEBUG MODE" ~/.hermes/logs/agent.log | tail -12
```

## Also Logged (not part of banner)

- Per-tool compression: `transform_tool_result: BELOW/CCR/SKIP <tool_name> size=<N> ratio=<R>`
- Terminal hook: `terminal_hook: BELOW/CCR size=<N> ratio=<R> (cmd: <truncated>)`
- Conv cache: `conv-cache: stored T<N> → <hash> (<total> total)`
- Proxy status on session_start: `aphrodite: token=UP/DOWN`
- Plugin registration: `aphrodite v<N> registered - <N> tools + hooks`

## Pitfall

The debug banner does NOT appear in the Hermes interactive TUI. It only goes to `agent.log`.
If you start Hermes with `APHRODITE_DEBUG=1` and look for output in the terminal, you
won't see it. Always check the log file.
