# aphrodite

<p align="center"><img src="assets/aphrodite.svg" width="80" alt="aphrodite"></p>

> Chat Completions proxy wrapping headroom-core with CCR + tool relay

[![release](https://img.shields.io/badge/release-v0.3.0-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)

## Overview

Aphrodite is a Rust proxy between Hermes and DeepSeek's Chat Completions API,
providing **CCR (Compress-Cache-Retrieve)** for tool outputs in real-time.

Two modes:
- **Cache** (:9797) — in-memory CCR, lightweight, >8KB threshold, preview preserved
- **Token** (:9798) — SQLite CCR, aggressive, >1KB threshold, tool injection + tool relay

## Quick Start

```bash
# Install via Hermes plugin
hermes skills install https://github.com/PlayForm/Aphrodite

# Or run directly
aphrodite --mode cache --listen 127.0.0.1:9797 --deepseek-key $DEEPSEEK_API_KEY
aphrodite --mode token --listen 127.0.0.1:9798 --deepseek-key $DEEPSEEK_API_KEY --tool-relay
```

## Endpoints

| Port | Mode | CCR | Threshold | Features |
|------|------|-----|-----------|----------|
| :9797 | Cache | In-memory | >8KB | Preview preserved |
| :9798 | Token | SQLite | >1KB | Tool relay, inject `headroom_retrieve` |

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | `ok` |
| GET | `/stats` | Live proxy statistics |
| POST | `/retrieve` | Resolve CCR markers |
| POST | `/tool/relay` | `headroom_retrieve` / `headroom_compress` |
| POST | `/ccr/create` | Programmatic CCR |
| GET | `/ccr/list` | List CCR entries |
| ANY | `/*path` | Chat Completions pass-through |

## Build

```bash
cargo build --release -p aphrodite
# Binary: target/release/aphrodite (~10MB)
```

```text
  ╭──────────╮
  │  Hermes  │
  ╰────┬─────╯
       │ POST /v1/chat/completions
  ╭────▼─────╮
  │ aphrodite│──── CCR store (SQLite / in-memory)
  ╰────┬─────╯
       │ forward
  ╭────▼─────╮
  │ DeepSeek │
  ╰──────────╯
```
