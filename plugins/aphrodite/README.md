# Aphrodite 💋 Hermes Plugin

> **CCR compression plugin for Hermes Agent — sub‑ms tool output compression, 28‑type classifier, 12 tools, context engine.**

Aphrodite intercepts tool output before it reaches the LLM and replaces it with compact, structured previews. The agent sees 15 tokens of metadata instead of 500 tokens of raw text — and retrieves the full content only when it actually needs it.

[![plugin](https://img.shields.io/badge/plugin-v1.62.21-purple)](plugin.yaml)
[![hermes](https://img.shields.io/badge/hermes-≥0.16.0-blue)](https://github.com/NousResearch/hermes-agent)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Installation

```bash
# Symlink into your Hermes profile
ln -s "$(pwd)" ~/.hermes/profiles/<your-profile>/plugins/aphrodite

# Enable the plugin
hermes plugins enable aphrodite

# Restart Hermes
hermes
```

The plugin auto-downloads the `aphrodite` binary from [PlayForm/Aphrodite releases](https://github.com/PlayForm/Aphrodite/releases) on first launch. No manual build required.

## Configuration

All settings live in `aphrodite.toml` (searched in: CWD → `~/.hermes/aphrodite/` → repo root). See the [monorepo](https://github.com/PlayForm/Aphrodite) for the full schema.

```toml
# Minimal example — place in ~/.hermes/aphrodite/aphrodite.toml
[defaults]
api_url = "https://api.deepseek.com"
model = "deepseek-v4-pro"

[[proxies]]
name = "cache"
listen = "0.0.0.0:9797"
mode = "cache"

[[proxies]]
name = "token"
listen = "0.0.0.0:9798"
mode = "token"
tool_relay = true

[compression]
engine_threshold_pct = 45
context_engine = true
```

### Essential env vars

```bash
export APHRODITE_API_KEY=<your-upstream-api-key>
```

## Plugin structure

```
├── __init__.py          # Entry point — proxy auto‑launch, version exports
├── plugin.yaml          # Hermes plugin manifest (12 tools, 5 hooks)
├── _core/               # Constants, TOML loader, config resolvers, settings
├── _engine.py           # ContextEngine — compresses middle turns to CCR
├── _hooks/              # Hermes hook handlers (transform, catalog, stats, …)
├── _marker/             # 28‑type classifier, template renderer, marker parse
├── _proxy/              # Proxy lifecycle (env, health, launch, markers)
├── _resolve.py          # Recursive CCR marker expansion (3 levels deep)
├── _binary.py           # Binary auto‑download + platform detection
├── _tools.py            # 12 aphrodite_* tool handlers + JSON schemas
├── _inline.py           # zlib fallback (works without proxy)
├── _automation.py       # Rhai scripting engine
├── pyproject.toml       # Python ≥3.11, no runtime deps
└── skills/              # 9 bundled skills (compression, proxy, tools, …)
```

## Tools

| Tool | Description |
| :--- | :--- |
| `aphrodite_retrieve` | Resolve `<<<CCR:hash\|type>>>` markers |
| `aphrodite_compress` | Compress content via CCR with type hint |
| `aphrodite_stats` | Proxy health, engine status, inline store |
| `aphrodite_rebuild` | Rebuild binary + restart proxies |
| `aphrodite_files` | Tracked file references grouped by tool |
| `aphrodite_diff` | Conversation turn history with summaries |
| `aphrodite_search` | Search CCR store by keyword or type |
| `aphrodite_test` | Smoke test suite (quick / full / pipeline) |
| `aphrodite_catalog` | Full CCR catalog with hashes, types, sizes |
| `aphrodite_reclassify` | Retroactive metadata enrichment |
| `aphrodite_prefetch` | Background file read — markers instantly, files load concurrently |
| `aphrodite_prefetch_status` | Prefetch queue status |
| `aphrodite_poll_container` | Container health/heartbeat |

## More

- **[Monorepo](https://github.com/PlayForm/Aphrodite)** — full docs, benchmark data, Rust proxy source
- **[Hermes Agent](https://github.com/NousResearch/hermes-agent)** — the agent framework this plugin targets

---

*CC0‑1.0 — public domain. A PlayForm project.*
