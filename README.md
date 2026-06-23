<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> **Your LLM burns 90% of its context on output it never reads. We fix that.**
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.
> Sub-ms compress, 12,800× max ratio, 28-type classifier, TOML-driven.
> _One binary. Zero dependencies. Millions of tokens saved._

[![release](https://img.shields.io/badge/release-v1.0.3-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v2.0.1-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Install ⚡

### As a Hermes plugin (recommended)

```bash
# Clone the standalone plugin repo
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/plugins/aphrodite
hermes plugins enable aphrodite
hermes
```

On first launch, the plugin auto-downloads the `aphrodite` binary from
[releases](https://github.com/PlayForm/Aphrodite/releases). No Rust toolchain needed.

### From source (monorepo)

```bash
git clone https://github.com/PlayForm/Aphrodite.git
cd Aphrodite
cargo build --release -p aphrodite
# Binary: target/release/aphrodite
# Dylib:  target/release/libaphrodite.dylib
```

### What changes after install

```
~/.hermes/
├── plugins/
│   └── aphrodite/          ← symlink to Aphrodite-Hermes
├── aphrodite/
│   ├── aphrodite           ← auto-downloaded binary (~12 MB)
│   └── ccr.db              ← SQLite CCR store (created on first run)
└── profiles/<name>/
    └── plugins/
        └── aphrodite → ~/.hermes/plugins/aphrodite
```

The plugin registers **12 tools** (`aphrodite_*`), **5 hooks**, and a **context engine** -
all routed through the Rust dylib. Two proxies launch automatically on `:9797` (cache)
and `:9798` (token).

---

## The Problem

Every time your agent reads a file, runs a build, searches code, or opens a
browser - the raw output floods its context window. Thousands of tokens of
compilation logs. Gigantic accessibility trees. Verbose JSON blobs. Your agent
spends its precious context budget **reading noise** instead of reasoning.

**Aphrodite intercepts output before it reaches the LLM and replaces it with a
compact, structured preview.** The agent sees 15 tokens of metadata instead of
500 tokens of raw text - and retrieves the full content only when it actually
needs it.

---

## How It Works ⚙️

```
 ANY OUTPUT ──────► Aphrodite ──────► Agent (preview, not raw)
                       │
                       ├─ tool output    → [build:2E 0W 14L]
                       ├─ terminal       → [terminal:cargo build exit=0]
                       ├─ file read      → [code_rust:3fns 414L]
                       ├─ search results → [search:10 results 5L]
                       ├─ build logs     → [build:1E 2W 142L]
                       ├─ browser snap   → [dom:342 elements]
                       ├─ JSON blobs     → [json:total_items,by_type]
                       └─ tracebacks     → [error:AttributeError]

    Agent decides:
    • Preview is enough → skip retrieval, keep reasoning
    • Needs detail     → aphrodite_retrieve(hash) → full content

    Context engine (automatic):
    • Session hits 45% context → middle turns auto-compressed to CCR
    • Agent never hits context window ceiling
```

Four layers, all under 1ms:

1. **Classify** - 28-type regex classifier identifies content (<0.1ms)
2. **Template** - TOML-driven templates produce `[type:key=val]` previews
3. **Store** - SHA-256 → SQLite/in-memory → `<<<CCR:hash|type|size>>>` marker
4. **Decide** - Agent reads preview, retrieves only when needed

---

## Architecture 🏗️

```
crates/aphrodite/          ← Core compression engine (binary + cdylib)
  ├── proxy.rs             ← HTTP proxy: classify → compress → store → preview
  ├── hooks.rs             ← transform_tool_result, transform_terminal_output
  ├── resolve.rs           ← CCR marker resolution (nested, recursive)
  ├── stage2.rs            ← Semantic reduction (JSON, build, diff, code)
  ├── struct_extract.rs    ← Code structure extraction (Rust, Python, Go, JS/TS)
  ├── state.rs             ← Session state, inline store, LRU
  ├── catalog.rs, session.rs, marker.rs, prefetch.rs, config_loader.rs
  └── lib.rs               ← 17 C ABI functions for dylib loading

crates/aphrodite-hermes/   ← Hermes-specific integration (cdylib)
  ├── tools.rs             ← 12 tool dispatch handlers
  ├── schemas.rs           ← JSON Schema definitions
  └── skills.rs            ← Bundled Hermes skills

plugins/aphrodite/         ← Thin Python loader (~145 lines)
  └── __init__.py          ← loads dylib, registers hooks/tools/engine via C ABI
```

| Mode  | Port  | Backend   | Threshold | Best for                  |
| :---- | :---: | :-------- | :-------: | :------------------------ |
| Cache | :9797 | In-memory |   >8 KB   | Speed, transient sessions |
| Token | :9798 | SQLite    |   >1 KB   | Durability, tool relay    |

All compression logic lives in the Rust dylib. Python is a thin FFI loader.
Hot-reload: rebuild the dylib → mtime change detected → next call picks up new
code automatically. 990 Rust tests + 116 Python tests, all passing.

---

## What You Save 💰

| Content type              | Without Aphrodite | With Aphrodite |  Savings |
| :------------------------ | ----------------: | -------------: | -------: |
| Git diff (42L)            |          ~350 tok |        ~15 tok |  **23×** |
| Build output (142L)       |        ~1,400 tok |        ~10 tok | **140×** |
| Traceback                 |           ~45 tok |        ~12 tok | **3.8×** |
| Terminal output           |          ~200 tok |        ~10 tok |  **20×** |
| Table (50 rows)           |          ~650 tok |         ~8 tok |  **81×** |
| JSON blob (30 keys)       |          ~400 tok |        ~10 tok |  **40×** |
| Web search (10 results)   |          ~800 tok |        ~15 tok |  **53×** |
| Browser snapshot (342 el) |        ~5,000 tok |        ~12 tok | **416×** |

**Median: 23× fewer tokens on tool output.** In a session with 50+ tool calls,
that's 15,000-50,000 tokens saved - enough for an entire extra reasoning turn.

---

## Tools 🔧

| Tool                   | Description                                          |
| :--------------------- | :--------------------------------------------------- |
| `aphrodite_retrieve`   | Resolve `<<<CCR:hash\|type>>>` markers                |
| `aphrodite_compress`   | Compress content via CCR with type hint               |
| `aphrodite_stats`      | Proxy health, engine status, inline store size        |
| `aphrodite_rebuild`    | Rebuild binary, kill proxies, restart                 |
| `aphrodite_files`      | Tracked file references, grouped by tool              |
| `aphrodite_diff`       | Conversation turn history with summaries              |
| `aphrodite_search`     | Search CCR store by keyword or type                   |
| `aphrodite_test`       | Smoke test suite: quick, full, matrix, pipeline       |
| `aphrodite_catalog`    | Full CCR catalog with hashes, types, sizes, previews  |
| `aphrodite_reclassify` | Retroactive metadata enrichment for unclassified CCR  |
| `aphrodite_prefetch`   | Background file read + compress; markers return instantly |

---

## Under the Hood 🧩

> **`./plugins/aphrodite/` is a separate repo** - it lives at
> [PlayForm/Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes).
> This monorepo tracks it as a git submodule.

```
plugins/aphrodite/          ← Standalone Hermes plugin (git submodule)
  __init__.py               ← 145-line Python loader (ctypes FFI)
  plugin.yaml               ← 12 tools, 5 hooks, context engine
  download.sh               ← Binary auto-downloader
  binaries/                 ← Platform-native dylib + proxy binary
  README.md                 ← Standalone install instructions

crates/aphrodite/           ← Core engine (binary + cdylib)
  src/
    lib.rs                  ← 17 C ABI functions (session, hooks, catalog, …)
    proxy.rs                ← HTTP proxy server (:9797/:9798)
    hooks.rs                ← transform_tool_result, terminal, pre/post LLM
    session.rs              ← Turn lifecycle, conversation index, git cache
    state.rs                ← AphroditeState, inline store, LRU, markers
    marker.rs               ← CCR marker generation + parse (<<<CCR:…>>>)
    catalog.rs              ← Full/compact/TOC catalog display
    resolve.rs              ← Recursive CCR marker expansion (3 levels)
    stage2.rs               ← Semantic reduction for JSON, build, diff, code
    struct_extract.rs       ← Code structure maps (Rust, Python, Go, JS/TS)
    config_loader.rs        ← TOML + env var config loading
    prefetch.rs             ← Background file read + compress threads

crates/aphrodite-hermes/    ← Hermes bridge
  src/
    lib.rs                  ← Universal dispatch (14 hooks → Rust functions)
    tools.rs                ← 12 tool handler implementations
    schemas.rs              ← JSON Schema for all tools
    skills.rs               ← Bundled skill registration for Hermes

vendor/headroom/            ← Headroom fork (git submodule)
  crates/headroom-core/     ← Content transforms, tokenizer, smart crusher
```

990 Rust tests (across 3 crates) + 116 Python tests. CC0-1.0 - public domain.

---

## Developer Workflow 🛠️

```bash
# Terminal 1: cargo watch (rebuilds dylib on change)
APHRODITE_NO_AUTO_LAUNCH=1 cargo watch -x 'build -p aphrodite'

# Terminal 2: Hermes (loads hot-reloaded dylib)
hermes --profile dev-aphrodite
```

| What changes   | What happens                                              |
| -------------- | --------------------------------------------------------- |
| Any `.rs` file | cargo watch rebuilds → dylib mtime changes                |
| Next hook call | Plugin detects new mtime → reloads dylib                  |
| Any `.py` file | `/quit` + `hermes` restart (Hermes caches Python imports) |
| Proxy binary   | `aphrodite_rebuild` tool → kill + copy + restart          |

---

## Quick Start 🚀

```bash
# Build
cargo build --release -p aphrodite

# Run (both proxies start automatically)
aphrodite

# Verify
curl http://127.0.0.1:9798/health
# → {"status":"ok","version":"v1.0.3"}

# Dev loop with auto-reload
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Configuration - everything in one file

```toml
# aphrodite.toml - all features, no recompile needed
[compression]
engine_threshold_pct = 45    # compress at 45% context
tool_threshold_token = 512   # token proxy threshold (bytes)
classifier_poll = true       # suppress CCR for clean outputs
context_engine = true        # engine on by default

[previews]
model_family = "code_first"  # compact | code_first | balance
code_structure_map = true    # show fn/struct/class sigs

[prompts]
retrieve_guidance = "minimal"
ccr_marker_hint = false
```

7 TOML sections, 54 template strings, all overridable via `APHRODITE_*` env vars.

---

## Performance ⚡

| Size   | Text  |  Code  |  JSON  |   Ratio   |
| :----- | :---: | :----: | :----: | :-------: |
| 1 KB   | 0.4ms | 0.3ms  | 0.5ms  |    26×    |
| 10 KB  | 0.6ms | 0.7ms  | 3.5ms  |   256×    |
| 50 KB  | 0.7ms | 0.6ms  | 1.0ms  |  1,280×   |
| 100 KB | 1.1ms | 1.0ms  | 1.1ms  |  2,560×   |
| 500 KB | 2.1ms | 7.9ms  | 2.8ms  | 12,800×   |

| Metric                    |  Value   |
| :------------------------ | :------: |
| Compression latency (avg) |  1.6 ms  |
| Classification latency    | <0.1 ms  |
| Preview generation        | <0.05 ms |
| Benchmark pass rate       | 19/19 ✅ |
| Smoke test pass rate      | 13/13 ✅ |

---

## Relationship to Headroom

Aphrodite embeds [Headroom](https://github.com/PlayForm/Headroom) - our custom
fork tracked as a git submodule at `vendor/headroom/`. Headroom provides the
content transforms (classifier, smart crusher, tokenizer); Aphrodite adds the
preview pipeline, CCR storage, Hermes integration, and dual-proxy architecture.

→ [Full comparison: Aphrodite vs Headroom](docs/APHRODITE-HEADROOM.md)

---

## Contributing 🤝

| Want to…          | Start here                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------ |
| Report a bug      | [Open an issue](https://github.com/PlayForm/Aphrodite/issues/new?template=bug_report.md)   |
| Suggest a feature | [Start a discussion](https://github.com/PlayForm/Aphrodite/discussions/new?category=ideas) |
| Submit a PR       | [Fork & open a PR](https://github.com/PlayForm/Aphrodite/pulls)                            |
| Ask a question    | [Discussions Q&A](https://github.com/PlayForm/Aphrodite/discussions/new?category=q-a)      |

No contribution is too small. First-time contributor? **Especially** welcome.

---

⭐ **Like Aphrodite?** [Star the repo](https://github.com/PlayForm/Aphrodite) -
it helps others find it and makes our day.

_Built with ❤️ by [PlayForm](https://github.com/PlayForm)._
