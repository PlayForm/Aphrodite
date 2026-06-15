# aphrodite

<p align="center"><img src="assets/aphrodite.svg" width="80"></p>

> Generic LLM proxy with CCR + tool relay. Works with any OpenAI-compatible API.

[![release](https://img.shields.io/badge/release-v0.3.1-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)

## Overview

Aphrodite sits between Hermes and any LLM API, providing **CCR (Compress-Cache-Retrieve)** 
for tool outputs in real-time. Two modes:

| Mode | Port | CCR | Threshold | Features |
|------|------|-----|-----------|----------|
| Cache | :9797 | In-memory | >8KB | Preview preserved |
| Token | :9798 | SQLite | >1KB | Tool relay, inject `headroom_retrieve` |

## Quick Start

```bash
# Any provider — just set --api-key and --api-url
export APHRODITE_API_KEY="sk-..."

# DeepSeek (default)
aphrodite --mode cache --listen 127.0.0.1:9797

# OpenAI
aphrodite --mode token --listen 127.0.0.1:9798 \
  --api-url https://api.openai.com \
  --model gpt-4o --tool-relay

# Anthropic (via OpenRouter)
aphrodite --mode token --listen 127.0.0.1:9798 \
  --api-url https://openrouter.ai/api \
  --model anthropic/claude-sonnet-4 --tool-relay

# Local (Ollama/LM Studio)
aphrodite --mode cache --listen 127.0.0.1:9797 \
  --api-url http://localhost:11434/v1 \
  --model llama3
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | `ok` |
| GET | `/stats` | Live proxy statistics |
| POST | `/retrieve` | Resolve CCR markers |
| POST | `/tool/relay` | Bidirectional tool execution |
| POST | `/ccr/create` | Programmatic CCR entry |
| GET | `/ccr/list` | List CCR entries |
| ANY | `/*path` | LLM API pass-through |

## Hermes Integration

```yaml
# ~/.hermes/config.yaml
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

## Dev

```bash
# Single pane, both modes, auto-rebuild
cargo watch -s 'cargo build -p aphrodite && \
  RUST_LOG=aphrodite=debug ./target/debug/aphrodite --mode cache --listen :9797 --dev & \
  RUST_LOG=aphrodite=debug ./target/debug/aphrodite --mode token --listen :9798 --tool-relay --dev & wait'
```

## Build

```bash
cargo build --release -p aphrodite
# Binary: target/release/aphrodite (~9MB)
```
