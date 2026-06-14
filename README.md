<p align="center">
  <img src="assets/logo.svg" alt="HermesCompress" width="180" height="180">
</p>

<h1 align="center">HermesCompress</h1>

<p align="center"><strong>Native headroom integration for Hermes Agent.</strong><br>
5 MCP tools + transparent middleware — zero monkey-patching.</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.1-purple?style=flat">
  <img src="https://img.shields.io/badge/savings-51--98%25-brightgreen?style=flat">
  <img src="https://img.shields.io/badge/latency-57--180ms-blue?style=flat">
  <img src="https://img.shields.io/badge/python-3.10+-orange?style=flat">
</p>

---

## Default Architecture

```
HERMES_HEADROOM_NATIVE=1 hermes --provider deepseek-direct
```

```
┌──────────────────────────────────────────────────────┐
│                  Hermes Agent                        │
│                                                      │
│  ┌────────────────────────────────────────────┐     │
│  │  llm_request middleware                    │     │
│  │                                            │     │
│  │  1. Scan messages for <<ccr:HASH>> markers │     │
│  │  2. Resolve ALL via re.sub callback        │     │
│  │  3. Compress inline via headroom ONNX      │     │
│  │     (SmartCrusher → Kompress → CacheAligner)│    │
│  └────────────────────────────────────────────┘     │
│                      ↓                               │
│            DeepSeek API (direct)                     │
└──────────────────────────────────────────────────────┘
```

| Component | Role |
|-----------|------|
| Middleware | Scans + resolves CCR markers, compresses inline |
| `hermes_compress` | Python library: `Compress`, `CompressOption`, `Proxy` |
| headroom ONNX | Kompress model — AST-aware, dedup-capable |

---

## Benchmarks

Inline compression via native middleware (direct API, 50-message session):

| Payload | Pipeline Stage | Savings |
|---------|---------------|:-------:|
| terminal | SmartCrusher | **97.9%** |
| read_file | CodeCompressor | **78.7%** |
| web_search | Kompress | **70.4%** |
| execute_code | SmartCrusher+Compaction | **63.2%** |
| search_files | ContentRouter | **54.8%** |
| cronjob | Full Pipeline | **51.0%** |

Live session numbers (direct + native, 46 messages):

| Metric | Value |
|--------|-------|
| Compression | **81-84%** |
| Latency | 123-179ms |
| Tokens saved | 31K-38K per turn |

---

## Quick Start

### Default (zero config)

```bash
# HERMES_HEADROOM_NATIVE=1 is already in ~/.hermes/.env
# deepseek-direct is the default provider
hermes
```

### Mode reference

| Mode | Command | Compression | Use case |
|------|---------|:-----------:|----------|
| Direct + native | `hermes` | **81-84%** inline | Default — max savings |
| Cache proxy | `hermes --provider deepseek-proxy-cache` | Prefix-freeze | Stable, cost savings |
| Token proxy | `hermes --provider deepseek-proxy-token` | 46-70% proxy | ⚠️ Re-compression |

---

## Tools

The LLM has access to 5 tools — registered unconditionally:

| Tool | Purpose | Example |
|------|---------|---------|
| `headroom_compress` | Compress content on demand | `headroom_compress(content="...")` |
| `headroom_retrieve` | Resolve CCR markers | `headroom_retrieve(hash="<<ccr:abc,...>>", path="/file")` |
| `headroom_stats` | Proxy compression stats | `headroom_stats()` |
| `headroom_proxy_start` | Start proxy server | `headroom_proxy_start(port=8787, mode="cache")` |
| `headroom_proxy_stop` | Stop proxy server | `headroom_proxy_stop(port=8787)` |

`headroom_retrieve` tries local file read first (fastest), then proxy cache, with optional BM25 `query` search.

---

## Provider Configuration

```yaml
# ~/.hermes/config.yaml
model:
  provider: deepseek-direct  # default
  default: deepseek-v4-pro

providers:
  deepseek-direct:
    base_url: https://api.deepseek.com/v1
    api_key_env: DEEPSEEK_API_KEY
    provider: deepseek

  deepseek-proxy-cache:
    base_url: http://127.0.0.1:8787/v1
    api_key_env: DEEPSEEK_API_KEY
    provider: deepseek

  deepseek-proxy-token:
    base_url: http://127.0.0.1:8788/v1
    api_key_env: DEEPSEEK_API_KEY
    provider: deepseek
```

---

## Plugin Structure

```
plugins/headroom/__init__.py   # 280 lines — single file
├── 5 tool handlers            # compress, retrieve, stats, proxy_start, proxy_stop
├── _resolve_ccr_in_messages   # re.sub callback — resolves ALL <<ccr:HASH>> markers
├── _compress_inline           # hermes_compress.Compress singleton
├── _on_llm_request            # middleware entry point (proxy-aware)
└── register()                 # registration surface
```

**Zero monkey-patching.** Uses Hermes' native `ctx.register_middleware("llm_request", ...)` and `ctx.register_tool(...)` APIs.

---

## Proxy Management

```bash
# Start dual proxies
.venv/bin/python scripts/proxy-dual.py

# Or from within Hermes (tools):
headroom_proxy_start(port=8787, mode="cache")
headroom_proxy_start(port=8788, mode="token")

# Stats
curl http://127.0.0.1:8787/stats
curl http://127.0.0.1:8788/stats
```

---

## Test Suite

```bash
.venv/bin/python -m pytest tests/test_headroom_plugin.py -v
```

13 tests covering: multi-CCR resolution, `[N items compressed]` format, cache reuse,
proxy detection (127.0.0.1 / localhost / 0.0.0.0 / remote), compress failure fallback,
hash extraction, and no-mutation guarantee.

---

## For Hermes Agent Developers

A standalone patch is at `patches/hermes-headroom-native.patch`. Apply to `~/.hermes/hermes-agent/`:

```bash
cd ~/.hermes/hermes-agent
git apply /path/to/HermesCompress/patches/hermes-headroom-native.patch
```

Adds `agent/headroom_native.py` — a standalone module with the same middleware logic,
exposed as `register(ctx)` for integration into Hermes core.
