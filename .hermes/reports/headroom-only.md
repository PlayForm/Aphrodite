# Headroom Proxy - Passthrough Performance Report

**2026-06-16 | v0.5.61**

---

## Scenario

30-turn Hermes coding session, 150 tool calls, 1M context window. Headroom proxy
only (`--no-optimize`). No CCR compression - raw passthrough of every tool
result.

## Architecture

```
hermes → headroom(:9799) → aphrodite(:9798) → DeepSeek API
```

Three hops. Headroom holds the SSE connection, aphrodite acts as a dumb relay -
no compression, no caching, no inline store.

## Key Metrics

| Metric                  | Value                   | Notes                                         |
| ----------------------- | ----------------------- | --------------------------------------------- |
| Context fill            | 93.2K / 1M after 28 min | 9.3% of window                                |
| Tokens saved            | 0                       | Passthrough - all output reaches LLM verbatim |
| Cache hits              | 0 / 0                   | No caching layer                              |
| Cache miss penalty      | N/A                     | Every request is a full round-trip            |
| Tool output compression | None                    | Raw passthrough, no CCR                       |
| Compression latency     | 0ms                     | No compression to perform                     |
| LLM latency             | 5-6s per call           | Full SSE wait every time                      |
| Inline store            | None                    | No LRU, no zlib                               |
| CCR catalog             | None                    | LLM never sees catalog                        |
| Persistent stats        | None                    | No state survives restart                     |
| Provider chain length   | 3 hops                  | hermes → headroom → aphrodite → API           |

## Context Growth Pattern

Conversation grows linearly with every tool call. Each tool result's full
content is serialized verbatim into the message history. After 150 tool calls:

- Raw terminal output: ~50K tokens (uncompressed)
- Raw tool results: ~30K tokens (uncompressed)
- Raw LLM responses: ~13K tokens
- Total: ~93K tokens consumed in 28 min

At this rate, a 1M context window fills in approximately **5 hours** of
continuous work.

## Compression Potential (unrealized)

Content that would compress well under CCR (but is currently passthrough):

| Content Type          | Typical Volume  | CCR Ratio | CCS            | Saved (if enabled) |
| --------------------- | --------------- | --------- | -------------- | ------------------ |
| Terminal build output | 2-15K per call  | 123x      | ~20-120 tokens | 98-99%             |
| File reads (code)     | 1-5K per call   | 43x       | ~25-115 tokens | 97-98%             |
| File reads (large)    | 5-50K per call  | 148x      | ~35-340 tokens | 99%+               |
| Search results        | 0.5-3K per call | 50x       | ~10-60 tokens  | 98%                |
| Diff output           | 1-10K per call  | 60x       | ~15-165 tokens | 98%+               |
| Error traces          | 1-5K per call   | 80x       | ~12-62 tokens  | 98%+               |

All of this content reaches the LLM as full raw text - no markers, no
deduplication, no caching.

## Limitations

- **No deduplication**: read the same file 5 times? The LLM sees it 5 times
  verbatim.
- **No caching**: identical tool output from consecutive calls is re-sent fresh.
- **No stats**: zero visibility into what's consuming context.
- **No retrieval**: ask "what did we see in that file?" - LLM must remember from
  raw text.
- **3-hop latency**: 33% more network hops than direct aphrodite → API.
- **Context waste**: every token sent is a token the LLM must process.

## When Passthrough Makes Sense

- Rapid prototyping / debugging compression profiles
- Short ad-hoc sessions (<5 turns)
- Benchmarking baseline (control group)
- Tasks where compression adds unacceptable latency (though CCR is 0.2-4.3ms)

---

## Quick Reference

```bash
# Run with headroom only (no compression)
hermes --profile headroom-passthrough

# Verify no compression running
curl -s :9798/stats | jq '.ccr_created'
# → 0 (no compression happening)

# Estimate context fill in real-time
curl -s :9798/stats | jq '.tokens_saved'
# → 0 (no savings)
```
