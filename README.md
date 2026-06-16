<p align="center">
  <img src="assets/aphrodite.svg" alt="Aphrodite" width="120">
</p>

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> CCR compression proxy for Hermes Agent  -  sub‑ms compress, 10× ratio, dual‑mode.

[![release](https://img.shields.io/badge/release-v0.5.68-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.13-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Architecture 🏗️

```
Hermes → aphrodite (:9797/:9798) → any LLM API
              ↓
  InMemoryCcrStore / SqliteCcrStore
              ↓
  Tool relay via POST /tool/relay
```

| Mode  | Port  | CCR Backend | Threshold | Features                  |
| :---- | :---: | :---------- | :-------: | :------------------------ |
| Cache | :9797 | In‑memory   |   >8 KB   | Preview, zero persistence |
| Token | :9798 | SQLite      |   >1 KB   | Tool relay, durability    |

---

## Quick Start 🚀

```bash
# Build
cargo build --release -p aphrodite

# Run both proxies
aphrodite

# Single mode
aphrodite --mode cache --listen :9797 --api-key $APHRODITE_API_KEY
aphrodite --mode token --listen :9798 --api-key $APHRODITE_API_KEY --tool-relay

# Dev loop
source .env.sh
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

---

## Performance ⚡

| Metric                       | Value          |
| :--------------------------- | :------------: |
| Compression latency (avg)    | 0.9 ms         |
| Retrieval latency (avg)      | 3.4 ms         |
| Compression ratio EMA        | 10.0×          |
| Tokens saved                 | 12M+           |
| Worker threads (default)     | 4× CPU, min 32 |
| Connection pool per host     | 100            |

---

## Tools 🔧

| Tool                   | Description                        |
| :--------------------- | :--------------------------------- |
| `aphrodite_retrieve`   | Resolve `<<<CCR:hash|type>>>`      |
| `aphrodite_compress`   | Compress content via CCR           |
| `aphrodite_stats`      | Proxy stats + latency histogram    |
| `aphrodite_rebuild`    | Rebuild CCR from conversation      |
| `aphrodite_files`      | Tracked files with CCR markers     |
| `aphrodite_diff`        | Diff compressed artifacts          |
| `aphrodite_search`     | Search CCR store                   |
| `aphrodite_test`       | Round‑trip compress/retrieve       |
| `aphrodite_catalog`    | Full CCR catalog listing           |

---

## Metrics 📊

Prometheus endpoint at `/metrics`. Docker image included:

```bash
docker run -d --name aphrodite-prometheus -p 9090:9090 \
  -v ./prometheus.yml:/etc/prometheus/prometheus.yml \
  --add-host=host.docker.internal:host-gateway \
  prom/prometheus
```

31 metrics covering: requests, compression, CCR hits/misses, cache,
tool relay, notifications, upstream errors, latency histograms,
body bytes, inline CCR, and store size.

---

## Profiles 👥

| Profile                     | Proxy        | Compression |
| :-------------------------- | :----------: | :---------: |
| `aphrodite-proxy-cache`     | :9797 cache  | Disabled    |
| `aphrodite-proxy-token`     | :9798 token  | 50%         |
| `aphrodite-compress-light`  | None         | Light       |
| `aphrodite-compress-medium` | None         | Medium      |
| `aphrodite-compress-aggressive` | None     | Aggressive  |

---

## Plugin 🧩

```
plugins/aphrodite/
  __init__.py      -  version, exports, proxy auto‑launch
  _core.py         -  constants, thresholds, CCR regex
  _inline.py       -  zlib fallback compression
  _marker.py       -  CCR marker formatting + parsing
  _binary.py       -  binary download + platform detection
  _proxy.py        -  proxy lifecycle (env, health, launch)
  _tools.py        -  9 tool handlers + JSON schemas
  _hooks.py        -  transform, pre/post LLM, terminal
  _engine.py       -  ContextEngine for compression pipeline
  _resolve.py      -  recursive marker expansion
```

---

## Dev Setup 🛠️

```bash
# Build
cargo build --release -p aphrodite

# Source environment
source ~/.hermes/.env.sh
source .env.sh

# Start proxy
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'

# Verify
curl -s http://127.0.0.1:9798/health
```

---

*Single Rust binary. Zero forced dependencies. CC0‑1.0.* | [Security policy](SECURITY.md)
