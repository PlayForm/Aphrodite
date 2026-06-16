# Aphrodite Verification Checklist

Run through after any significant plugin or proxy change.

## Memory
- [ ] Memory shows T1, T2, T3 (not UUIDs)
- [ ] Sequential turn counter works across sessions

## Engine
- [ ] Engine catalog shows "engine: N compressions | last: X msgs → CCR:hash"
- [ ] Context engine compresses when >10 messages
- [ ] proxy=token in APHRODITE block

## Tools
- [ ] aphrodite_retrieve returns raw content (not re-compressed)
- [ ] aphrodite_stats returns proxy health + engine status
- [ ] No "Unknown toolsets" warning

## Session Reset
- [ ] On /reset, counters + inline store clear
- [ ] On session end, engine state cleans up

## Test Pane Launch (aggressive thresholds for quick testing)
```bash
APHRODITE_DEBUG=1 \
APHRODITE_ENGINE_THRESHOLD_PCT=0 \
APHRODITE_ENGINE_PROTECT_FIRST=2 \
APHRODITE_ENGINE_PROTECT_LAST=5 \
APHRODITE_ENGINE_MIN_MSGS=0 \
APHRODITE_TOOL_THRESHOLD_TOKEN=1024 \
APHRODITE_TOOL_THRESHOLD_CACHE=8192 \
APHRODITE_TERMINAL_THRESHOLD=2048 \
APHRODITE_INLINE_THRESHOLD=4096 \
APHRODITE_RECURSIVE_DEPTH=3 \
hermes --provider custom:aphrodite-token
```

## Proxy Pane Launch
```bash
cargo watch -x 'run -p aphrodite'
# Requires api_key in aphrodite.toml [defaults]
```
