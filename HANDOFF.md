# Aphrodite Plugin - Comprehensive Handoff

## Current State: v1.8.0 (16 commits)

### What's Working
- Terminal output: fixed (`stdout`→`output` parameter mismatch was root cause of 2-day bug)
- Tool output compression: >1KB token mode, >8KB cache mode, CCR markers inline
- Inline fallback: zlib+base64 when proxy down, session-scoped store (500 entries)
- Recursive CCR resolution: headroom_retrieve unwraps nested markers up to 3 levels
- Knowledge map: pre_llm_hook builds grouped catalog with previews per type (tool/terminal)
- Turn compression: old turns (>6 back) compressed to CCR with structured summaries
- Context Engine: AphroditeContextEngine subclasses ContextEngine, plugs into Hermes compress()
  - should_compress() returns True always (configurable via APHRODITE_ENGINE_THRESHOLD_PCT)
  - compress() offloads middle messages to CCR, keeps first 2 + last 5 raw
  - Tool-chain safe: extends tail boundary to not split tool_call→tool_result
- Sequential memory: _turn_counter replaces Hermes UUIDs → shows T1, T2, T3
- headroom_stats tool: returns proxy health + engine status + inline store size
- Extensible hooks: aphrodite_engine_compressed fires on each compression (other plugins can listen)
- get_engine() global accessor for other plugins
- Session reset: clears _conv_index, _turn_counter, inline store, engine counters
- All thresholds configurable via env vars

### Known Limitations
- pre_llm_call hook gets a COPY of conversation_history (can't remove old messages from context)
- Context engine only compresses when >2+3+5=10 messages exist for middle content
- headroom_retrieve/stats excluded from compression (fixed infinite recursion bug)
- Old sessions show UUIDs in memory (fixed code, needs fresh session)
- aphrodite.toml needs api_key for cargo watch (not committed, .gitignored)

### Key Files
- plugins/aphrodite/__init__.py (970 lines) - all logic
- plugins/aphrodite/plugin.yaml - manifest
- aphrodite.toml - proxy config (gitignored, contains api_key)
- crates/aphrodite/src/ - Rust proxy binary

### Config
```yaml
context.engine: aphrodite    # activates context engine
compression.enabled: true    # Hermes compression on
```
Revert: `hermes config set context.engine compressor`

### Debug Env Vars
```
APHRODITE_DEBUG=1                    verbose logging
APHRODITE_ENGINE_THRESHOLD_PCT=75    trigger at 75% context (0=always)
APHRODITE_ENGINE_PROTECT_FIRST=3     head messages kept raw
APHRODITE_ENGINE_PROTECT_LAST=10     tail messages kept raw
APHRODITE_ENGINE_MIN_MSGS=50         min messages before compress
APHRODITE_TOOL_THRESHOLD_TOKEN=512   token mode threshold
APHRODITE_TERMINAL_THRESHOLD=1024    terminal compression threshold
APHRODITE_INLINE_THRESHOLD=8192      inline fallback threshold
APHRODITE_RECURSIVE_DEPTH=2          CCR unwrap depth
APHRODITE_DEV=1                      disable all hooks (dev mode)
```

### Verification Checklist
- [ ] Memory shows T1, T2, T3 (not UUIDs)
- [ ] Engine catalog shows "engine: N compressions | last: X msgs → CCR:hash"
- [ ] proxy=token in APHRODITE block
- [ ] headroom_retrieve returns raw content (not re-compressed)
- [ ] headroom_stats returns proxy health + engine status
- [ ] No "Unknown toolsets" warning
- [ ] On /reset, counters + inline store clear
- [ ] Context engine compresses when >10 messages

### Test Pane Launch
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

### Proxy Pane Launch
```bash
# In separate pane:
cargo watch -x 'run -p aphrodite'
# Requires api_key in aphrodite.toml [defaults]
```
