# Profile Compression Matrix

7 profiles with distinct compression strategies for the aphrodite plugin + proxy.

## Quick Reference

```
Profile                    Engine    Hermes Comp.   Proxy        Use Case
─────────────────────────────────────────────────────────────────────────────
aphrodite-barebone         default   OFF             direct       No aphrodite, raw DeepSeek
aphrodite-compress-off     aphrodite OFF             :9797 cache  Engine only, no Hermes comp
aphrodite-proxy-cache      aphrodite OFF             :9797 cache  Engine only, in-memory CCR
aphrodite-proxy-token ★    aphrodite OFF             :9798 token  Engine only, SQLite CCR (current dev)
aphrodite-compress-light   aphrodite ON              :9797 cache  DOUBLE compress
aphrodite-compress-medium  aphrodite ON              :9797 cache  DOUBLE compress
aphrodite-compress-aggressive aphrodite ON           :9797 cache  DOUBLE compress (max)
```

## Key

- **Engine**: `aphrodite` = context engine active (compresses middle messages to CCR at threshold). `default` = Hermes built-in compressor only.
- **Hermes Comp.**: `ON` = `compression.enabled: true` in config.yaml. This is Hermes' own summarizer — a SECOND compression layer on top of the aphrodite engine.
- **Proxy**: :9797 = cache proxy (in-memory CCR, >8KB threshold). :9798 = token proxy (SQLite CCR, >1KB threshold, tool relay). `direct` = no proxy.
- **DOUBLE compress**: Both the aphrodite context engine AND Hermes' built-in compressor fire independently. Valid but can degrade quality — they don't coordinate.

## Engine Threshold

All aphrodite-engine profiles share the same threshold (50% by default). It's in the plugin code (`_engine.py`), not per-profile config. To override:

```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1  # fire at 1% fill
APHRODITE_ENGINE_MIN_MSGS=1      # no minimum messages
APHRODITE_ENGINE_PROTECT_FIRST=0 # don't protect first
APHRODITE_ENGINE_PROTECT_LAST=0  # don't protect last
```

Only `aphrodite-barebone` skips the aphrodite plugin entirely (`toolsets: '["hermes-cli"]'`, no `aphrodite` toolset).

## Provider Routing

All non-barebone profiles route through the aphrodite proxy (cache or token mode). The proxy acts as middleware — it forwards requests to DeepSeek API while compressing large responses. Config example:

```yaml
# aphrodite-token provider (:9798)
providers:
  aphrodite-token:
    api_key_env: APHRODITE_API_KEY
    base_url: http://127.0.0.1:9798
    max_tokens: 65536
    provider: deepseek
```

## Discovered

2026-06-16 — enumerated all 7 profiles via `hermes config` during benchmark session.
