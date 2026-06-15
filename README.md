# aphrodite

<p align="center"><img src="assets/aphrodite.svg" width="80"></p>

> Generic LLM proxy with CCR + tool relay - any OpenAI-compatible API.

[![release](https://img.shields.io/badge/release-v0.4.0-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![tests](https://img.shields.io/badge/tests-6/6-green)](.)

## Benchmarks

| File | Size | Compressed | Ratio | Latency |
|------|------|------------|-------|---------|
| LICENSE | 7.0KB | 24B | 290x | 40ms |
| README | 2.5KB | 24B | 103x | 64ms |
| 20KB text | 20KB | 24B | 833x | - |
| **Retrieve** | 20KB | - | - | **27ms avg** |

## Quick Start

```bash
# Multi-proxy from config
aphrodite  # reads aphrodite.toml → starts :9797 + :9798

# Cache mode only
aphrodite --mode cache --listen :9797 --api-key $APHRODITE_API_KEY

# Token mode with tool relay
aphrodite --mode token --listen :9798 --api-key $APHRODITE_API_KEY --tool-relay

# Dev: single cargo watch, auto-rebuild
APHRODITE_API_KEY=sk-... cargo watch -x 'run -p aphrodite'
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Upstream probe + version |
| GET | `/stats` | Latency buckets, CCR hits/misses |
| GET | `/history` | Last 50 request ring buffer |
| GET | `/version` | Crate version |
| POST | `/retrieve` | Resolve CCR markers |
| POST | `/tool/relay` | headroom_retrieve / headroom_compress |
| POST | `/ccr/create` | Programmatic CCR entry |
| GET | `/ccr/list` | Entry count + backend info |
| ANY | `/*path` | LLM API pass-through |

## Hermes Config

```yaml
providers:
  aphrodite-cache:
    api_key_env: APHRODITE_API_KEY
    provider: deepseek
    base_url: http://127.0.0.1:9797
  aphrodite-token:
    api_key_env: APHRODITE_API_KEY
    provider: deepseek
    base_url: http://127.0.0.1:9798
fallback_providers:
  - deepseek-direct
```

## Modes

| Mode | Port | CCR | Threshold | Features |
|------|------|-----|-----------|----------|
| Cache | :9797 | In-memory | >8KB | Preview preserved |
| Token | :9798 | SQLite | >1KB | Tool relay, injection |

## Build

```bash
cargo build --release -p aphrodite
# → target/release/aphrodite (~9MB)
```

## Architecture

```
Hermes → aphrodite (:9797/:9798) → any LLM API
              ↓ CCR store
         InMemoryCcrStore (cache) / SqliteCcrStore (token)
              ↓ Tool relay
         POST /tool/relay ← bidirectional Hermes↔proxy
```
