# aphrodite

<p align="center"><img src="assets/aphrodite.svg" width="80"></p>

> CCR compression proxy for Hermes Agent - sub-ms compress, 10× ratio, dual-mode architecture.

[![release](https://img.shields.io/badge/release-v0.5.58-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.4-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![bench](https://img.shields.io/badge/bench-19/19-green)](.hermes/PERFORMANCE.md)
[![bugs](https://img.shields.io/badge/bugs-46/49-yellow)](.hermes/TASKS.md)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Quick Start

```bash
# Build the binary
cargo build --release -p aphrodite

# Run via Hermes (auto-launches proxy on :9798)
hermes --profile aphrodite-proxy-token

# Standalone proxy (both modes)
aphrodite  # reads aphrodite.toml → starts :9797 + :9798

# Single mode
aphrodite --mode cache --listen :9797 --api-key $APHRODITE_API_KEY
aphrodite --mode token --listen :9798 --api-key $APHRODITE_API_KEY --tool-relay

# Dev loop
source .env.sh
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

---

## Architecture

```
Hermes (plugin v1.62.4) → aphrodite (:9797/:9798) → any LLM API
                              ↓
      InMemoryCcrStore (:9797) / SqliteCcrStore (:9798)
                              ↓
          Tool relay via POST /tool/relay (HTTP ↔ Hermes hooks)
```

| Mode | Port | CCR Backend | Threshold | Features |
|------|------|-------------|-----------|----------|
| Cache | :9797 | In-memory | >8 KB | Preview preserved, zero persistence |
| Token | :9798 | SQLite | >1 KB | Tool relay, injection, durability |

Both proxies expose `/health`, `/stats`, `/history`, `/version`, `/retrieve`, `/ccr/create`, `/ccr/list`, and pass-through `/*path` to the upstream LLM API.

---

## Performance

| Metric | Value |
|--------|-------|
| **Benchmark tests** | 19/19 passed |
| **Compression latency (avg)** | 0.9 ms |
| **Retrieval latency (avg)** | 3.4 ms |
| **Compression ratio EMA** | 10.0× |
| **Tokens saved** | 12,341,104 |
| **CCR catalog entries** | 181 |
| **CCR database** | 18 MB (SQLite at `~/Library/Application Support/aphrodite/ccr.db`) |
| **CCR hits / created / misses** | 22 / 123 / 0 |

Sub-millisecond compression for payloads up to 100 KB. 500 KB text peaks at 4.3 ms average. Retrieval is 10/10 reliable at 3.4 ms average. Ratio scales linearly with content size (rolling-hash CCR).

---

## Tools (9)

All tools are prefixed `aphrodite_` and registered via plugin hooks:

| Tool | Description |
|------|-------------|
| `aphrodite_retrieve` | Resolve `<<<CCR:hash|type|size>>>` markers from proxy |
| `aphrodite_compress` | Compress content inline via zlib fallback |
| `aphrodite_stats` | Proxy statistics: latency buckets, CCR hit/miss rate |
| `aphrodite_rebuild` | Rebuild CCR database from current conversation |
| `aphrodite_files` | List tracked files with CCR markers |
| `aphrodite_diff` | Diff two compressed artifacts |
| `aphrodite_search` | Search CCR store by hash or content preview |
| `aphrodite_test` | Round-trip compress+retrieve test |
| `aphrodite_catalog` | List CCR catalog (modes: full, compact, tool) |

---

## Profiles

| Profile | Proxy | Compression | Description |
|---------|-------|-------------|-------------|
| `aphrodite-barebone` | None | N/A | Direct API, no plugins - baseline debugging |
| `aphrodite-proxy-cache` | :9797 cache | Disabled | In-memory CCR store, pass-through only |
| `aphrodite-proxy-token` | :9798 token | 50% | SQLite CCR + tool relay - primary profile |
| `aphrodite-compress-off` | None | Disabled | Plugin active, no compression |
| `aphrodite-compress-light` | None | Light | Minimal compression threshold |
| `aphrodite-compress-medium` | None | Medium | Balanced compress/retrieve |
| `aphrodite-compress-aggressive` | None | Aggressive | Maximum compression, lower threshold |

---

## Hermes Config

```yaml
providers:
  aphrodite-cache:
    base_url: http://127.0.0.1:9797
    api_key_env: APHRODITE_UPSTREAM_API_KEY
    provider: deepseek
  aphrodite-token:
    base_url: http://127.0.0.1:9798
    api_key_env: APHRODITE_UPSTREAM_API_KEY
    provider: deepseek
fallback_providers:
  - deepseek-direct
```

Enable the context engine:
```bash
hermes config set context.engine aphrodite
```

Debug mode:
```bash
APHRODITE_DEBUG=1 hermes --profile aphrodite-proxy-token
```

---

## Dev Setup

```bash
# 1. Build the proxy
cargo build --release -p aphrodite

# 2. Source environment
source ~/.hermes/.env.sh           # APHRODITE_API_KEY, etc.
source .env.sh                     # project-level overrides

# 3. Start proxy in background
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'

# 4. Launch Hermes (separate pane/terminal)
hermes --profile aphrodite-proxy-token -m deepseek-v4-pro

# 5. Verify
curl -s http://127.0.0.1:9798/health
curl -s http://127.0.0.1:9797/health
```

**WezTerm layout** - 3-pane workflow:

| Pane | Role | Command |
|------|------|---------|
| 0 | Proxy | `RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'` |
| 1 | Hermes (barebone) | `APHRODITE_DEV=1 hermes -m deepseek-v4-pro` |
| 2 | Hermes (token proxy) | `hermes -p aphrodite-proxy-cache -m deepseek-v4-pro` |

---

## Plugin Structure

```
plugins/aphrodite/
  __init__.py     - version, module exports, proxy auto-launch
  _core.py        - constants, thresholds, CCR regex, inline store
  _inline.py      - zlib fallback compression
  _marker.py      - CCR marker formatting, parsing, preview embedding
  _binary.py      - binary download + platform detection
  _proxy.py       - proxy lifecycle (env, health, launch, alive checks)
  _tools.py       - 9 tool handlers + JSON schemas
  _hooks.py       - transform_tool_result, pre_llm, post_llm, terminal
  _engine.py      - ContextEngine for Hermes compression pipeline
  _resolve.py     - CCR resolve with recursive marker expansion
  plugin.yaml     - v1.62.4, 9 tools, 5 hooks, context engine
```

---

## Bug Status

| Severity | Count | Fixed |
|----------|-------|-------|
| Critical | 6 | 0 |
| High | 6 | 1 |
| Medium | 16 | 2 |
| Low | 15 | 0 |
| Infra | 6 | 0 |
| **Total** | **49** | **3** |

Full bug audit in [.hermes/TASKS.md](.hermes/TASKS.md).

---

## Build

```bash
cargo build --release -p aphrodite
# → target/release/aphrodite (~9 MB statically linked)
```

Aphrodite is a single Rust crate at `crates/aphrodite/`, using the workspace resolver. Depends on `headroom-core` (vendored at `vendor/headroom/`) for rolling-hash based content-defined chunking.

---

## Recent

- **v0.5.58 / plugin v1.62.4** - benchmark suite, toolchain, full changelog
- **46 bugs resolved** across 3 audit-fix waves (v0.5.51-v0.5.54)
- **7 fresh fixes** in latest release
- **Modular refactor** - 1656-line `__init__.py` split into 9 atomic modules
- **Benchmark suite** - 19 tests, sub-ms compression, 12M+ tokens saved
- **Profile matrix** - 7 profiles covering barebone → full proxy → aggressive compress
- **CCR catalog toggle** - full/compact/tool display modes

---

*Single Rust binary. Zero forced dependencies. CC0-1.0.*
