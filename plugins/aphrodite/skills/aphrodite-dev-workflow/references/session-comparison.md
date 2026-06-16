# Session Comparison — Compressed vs Normal

## Purpose

Capture side-by-side comparison of what the LLM sees with and without aphrodite compression active. Used for debugging, onboarding, and performance analysis.

## Quick Capture Commands

```bash
# From aggressive profile session
grep -A2 "pre_llm_hook:\|transform_tool_result: CCR\|terminal_hook: CCR" \
  ~/.hermes/profiles/aphrodite-compress-aggressive/logs/agent.log | tail -20

# From token profile session (minimal compression)
grep -A2 "pre_llm_hook:\|transform_tool_result:" \
  ~/.hermes/profiles/aphrodite-proxy-token/logs/agent.log | tail -20

# Proxy CCR stats (persistent)
curl -s http://127.0.0.1:9798/stats/db

# Plugin stats (per-session)
grep "aphrodite v\|tokens_saved\|CCR created" \
  ~/.hermes/profiles/*/logs/agent.log | tail -5
```

## Key Differences

| Aspect | WITH Compression | WITHOUT |
|--------|-----------------|---------|
| Tool output >1KB | `<<<CCR:hash\|type\|size>>>` | Full raw output |
| Terminal output >2KB | `<<<CCR:hash\|terminal\|size>>>` | Full raw output |
| Pre-LLM catalog | Injected before every call | Not present |
| Context growth | Slow (markers are ~50 bytes) | Linear (grows with every tool call) |
| Cache hit ratio | 82-100% | 59-100% |
| What LLM sees | Catalog + markers + user text | Raw messages |

## Catalog Format (LLM sees this)

```
[APHRODITE COMPRESSION CATALOG]
Active CCR markers: 3 (5.4KB compressed from ~56KB bytes)
  <<<CCR:263abec4b07dd0286ba25b37>>> - terminal output (2968 bytes)
  <<<CCR:f329cc6c2c8109f5f199edc8>>> - terminal output (1053 bytes)
  <<<CCR:7a03b189cc052cf6728c6073>>> - tool result (3547 bytes)
Retrieve any marker with: aphrodite_retrieve(key=hash)
```

## Files

- `.hermes/SESSION-COMPRESSED.md` — Full turn-by-turn log with compression
- `.hermes/SESSION-NORMAL.md` — Full turn-by-turn log without compression
