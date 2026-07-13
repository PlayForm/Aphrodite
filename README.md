<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# Aphrodite 💋 (`aphrodite`)

> **Your LLM burns 90% of its context on output it never reads. We fix that.**
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.
> Sub-ms compress, 12,800× max ratio, 28-type classifier, TOML-driven.
> _One binary. Zero dependencies. Millions of tokens saved._

[![release](https://img.shields.io/badge/release-v1.3.1-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![crates.io](https://img.shields.io/crates/v/aphrodite?color=orange)](https://crates.io/crates/aphrodite)
[![plugin](https://img.shields.io/badge/plugin-v2.0.6-purple)](plugins/aphrodite/plugin.yaml)
![rust](https://img.shields.io/badge/rust-1.88+-orange)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Install ⚡

Aphrodite ships two build artifacts from two crates: the `aphrodite` proxy
binary (standalone HTTP proxy, `:9797`/`:9798`) and the
`libaphrodite_hermes.{dylib,so,dll}` dylib (loaded in-process by the Hermes
plugin, no CLI of its own). You don't need to know this for the common path
below - it starts mattering the moment something needs installing by hand.

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

### Via cargo install

```bash
cargo install aphrodite          # proxy binary + libaphrodite.dylib
cargo install aphrodite-hermes   # hermes bridge dylib + helper bin
aphrodite setup                  # plugin structure + config + symlink
```

All three commands together: the proxy on `:9797/:9798`, the Hermes dylib
for hook/tool integration, and the plugin registered with Hermes.

> **Windows**: `plugins/aphrodite/download.ps1` is a native PowerShell
> equivalent of `download.sh` - no Git Bash/WSL needed. See
> [Windows install](docs/install/windows.md) for the fast path plus a fully
> manual walkthrough. Full per-platform guides (including which of
> Aphrodite's two build artifacts you actually need) live under
> [docs/install/](docs/install/README.md).

### `cargo install` + one-shot bootstrap

```bash
cargo install aphrodite aphrodite-hermes
aphrodite setup --api-key sk-... --api-url https://api.deepseek.com --model deepseek-v4-pro
```

`aphrodite setup` provisions `~/.hermes/aphrodite/` end to end - binaries,
dylibs, `aphrodite.toml`, `plugin.yaml`, and `hermes plugins enable
aphrodite` - in one command. **Its plugin-symlink step is currently a no-op
on Windows** (Unix-only code path); see
[docs/install/README.md](docs/install/README.md#the-aphrodite-setup-gap-on-windows)
if you hit that.

### From source (monorepo)

```bash
git clone https://github.com/PlayForm/Aphrodite.git
cd Aphrodite
git submodule update --init --recursive  # required - vendored deps live in submodules
cargo build --release -p aphrodite -p aphrodite-hermes
# Binary: target/release/aphrodite
# Dylibs: target/release/libaphrodite.dylib, target/release/libaphrodite_hermes.dylib
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

1. **Classify** - 28-type regex classifier identifies content (40–123 ns)
2. **Template** - TOML-driven templates produce `[type:key=val]` previews
3. **Store** - BLAKE3 → SQLite/in-memory → `<<<CCR:hash|type|size>>>` marker
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
  └── lib.rs               ← 22 C ABI functions for dylib loading

crates/aphrodite-hermes/   ← Hermes-specific integration (cdylib)
  ├── tools.rs             ← 12 tool dispatch handlers
  ├── schemas.rs           ← JSON Schema definitions
  └── skills.rs            ← Bundled Hermes skills

plugins/aphrodite/         ← Thin Python loader (423 lines)
  └── __init__.py          ← loads dylib, registers hooks/tools/engine via C ABI
```

| Mode  | Port  | Backend   | Threshold | Best for                  |
| :---- | :---: | :-------- | :-------: | :------------------------ |
| Cache | :9797 | In-memory |   >8 KB   | Speed, transient sessions |
| Token | :9798 | SQLite    |   >1 KB   | Durability, tool relay    |

All compression logic lives in the Rust dylib. Python is a thin FFI loader.
Hot-reload: rebuild the dylib → mtime change detected → next call picks up new
code automatically. The two Aphrodite crates carry unit tests (plus the full
vendored Headroom suite) - run `cargo test --workspace` to see the current
count and confirm they pass.

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

**Real example:** a 6-turn dev session (builds, tests, docs, git) compressed
216 KB of raw tool output into 12 markers — **~54,000 tokens → ~240 tokens**
(225× reduction), leaving 99.6% of the context window free for reasoning.

---

## Tools 🔧

| Tool                        | Description                                              |
| :-------------------------- | :------------------------------------------------------- |
| `aphrodite_retrieve`        | Resolve `<<<CCR:hash\|type>>>` markers                   |
| `aphrodite_compress`        | Compress content via CCR with type hint                  |
| `aphrodite_stats`           | Proxy health, engine status, inline store size           |
| `aphrodite_rebuild`         | Rebuild binary, kill proxies, restart                    |
| `aphrodite_files`           | Tracked file references, grouped by tool                 |
| `aphrodite_diff`            | Conversation turn history with summaries                 |
| `aphrodite_search`          | Search CCR store by keyword or type                      |
| `aphrodite_test`            | Smoke test suite: quick (1 check), full (3 checks)       |
| `aphrodite_catalog`         | Full CCR catalog with hashes, types, sizes, previews     |
| `aphrodite_reclassify`      | Retroactive metadata enrichment for unclassified CCR     |
| `aphrodite_prefetch`        | Read + compress files on demand; markers returned inline |
| `aphrodite_prefetch_status` | Live prefetch schedule: loading, ready, errors           |

---

## Under the Hood 🧩

> **`./plugins/aphrodite/` is a separate repo** - it lives at
> [PlayForm/Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes).
> This monorepo tracks it as a git submodule.

```
plugins/aphrodite/          ← Standalone Hermes plugin (git submodule)
  __init__.py               ← Python loader (ctypes FFI, thin shim)
  plugin.yaml               ← 12 tools, 5 hooks, context engine
  download.sh               ← Binary auto-downloader (macOS/Linux/Git Bash/WSL)
  download.ps1              ← Binary auto-downloader (native Windows PowerShell)
  binaries/                 ← Platform-native dylib + proxy binary
  README.md                 ← Standalone install instructions

crates/aphrodite/           ← Core engine (binary + cdylib)
  src/
    lib.rs                  ← 22 C ABI functions (session, hooks, catalog, …)
    proxy.rs                ← HTTP proxy server (:9797/:9798)
    hooks.rs                ← transform_tool_result, terminal, pre/post LLM
    session.rs              ← Turn lifecycle, conversation index, git cache
    state.rs                ← AphroditeState, inline store, LRU, markers
    marker.rs               ← CCR marker generation + parse (<<<CCR:…>>>)
    catalog.rs              ← Full/compact/TOC catalog display
    resolve.rs              ← Recursive CCR marker expansion (5 levels)
    stage2.rs               ← Semantic reduction for JSON, build, diff, code
    struct_extract.rs       ← Code structure maps (Rust, Python, Go, JS/TS)
    config_loader.rs        ← TOML + env var config loading
    prefetch.rs             ← On-demand file read + compress

crates/aphrodite-hermes/    ← Hermes bridge
  src/
    lib.rs                  ← Universal dispatch (5 hooks → Rust functions)
    tools.rs                ← 12 tool handler implementations
    schemas.rs              ← JSON Schema for all tools
    skills.rs               ← Bundled skill registration for Hermes

vendor/headroom/            ← Headroom fork (git submodule)
  crates/headroom-core/     ← Content transforms, tokenizer, smart crusher
```

The two Aphrodite crates carry unit tests (the vendored Headroom fork carries
its own suite) - run `cargo test --workspace` for the current count and
pass/fail status. CC0-1.0 - public domain.

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

### Git hooks

`git config core.hooksPath .githooks` (set automatically by `pnpm install`
via `package.json`'s `prepare` script) enables `.githooks/post-checkout` and
`.githooks/post-merge`, which run `git submodule update --init --recursive`
after every checkout/merge so the three vendored submodules
(`plugins/aphrodite`, `vendor/headroom`, `vendor/rtk`) always match the
commit pinned in this repo's index. This is deliberately pointer-respecting,
not `--remote`-tracking - submodule pins only move via an explicit, reviewed
action (e.g. `Maintain/scripts/release/auto-release.sh`).

---

## Quick Start 🚀

```bash
# Build
cargo build --release -p aphrodite

# Run (both proxies start automatically)
aphrodite

# Verify
curl http://127.0.0.1:9798/health
# -> {"status":"ok","version":"v1.2.2"}

# Dev loop with auto-reload
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Configuration - everything in one file

```toml
# aphrodite.toml - no recompile needed
[compression]
tool_threshold_token = 512   # token proxy threshold (bytes)
tool_threshold_cache = 4096  # cache proxy threshold (bytes)
inline_threshold = 2048      # inline-vs-durable CCR storage cutoff (bytes)
code_multiplier = 3.0        # multiply threshold for code_* content types
```

These four `[compression]` fields are live on the Rust proxy: each is
overridable via an `APHRODITE_*` env var, and editing + saving the file (or
`POST /reload`) applies the new value immediately, no restart. Other
`[compression]`/`[previews]`/`[prompts]` keys exist in the schema but either
only affect the separate Hermes-plugin dylib session or aren't consumed by
anything yet - see [`docs/config/aphrodite-toml.md`](docs/config/aphrodite-toml.md)
for the full, accurate breakdown of what's wired where.

---

## Performance ⚡

| Size   | Text  | Code  | JSON  |  Ratio  |
| :----- | :---: | :---: | :---: | :-----: |
| 1 KB   | 0.4ms | 0.3ms | 0.5ms |   26×   |
| 10 KB  | 0.6ms | 0.7ms | 3.5ms |  256×   |
| 50 KB  | 0.7ms | 0.6ms | 1.0ms | 1,280×  |
| 100 KB | 1.1ms | 1.0ms | 1.1ms | 2,560×  |
| 500 KB | 2.1ms | 7.9ms | 2.8ms | 12,800× |

| Metric                    |   Value   |
| :------------------------ | :-------: |
| Compression latency (avg) |  1.6 ms   |
| Classification latency    | 40–123 ns |
| Preview generation        | <0.05 ms  |

Benchmarks (`cargo run --release --example bench_0N_*`, see
`Maintain/examples/`) and the in-process smoke suite (`aphrodite_test`, 3
checks in `full` mode) are reproducible commands rather than fixed pass-rate

### Real-world savings (Hermes Agent, deepseek-v4-pro)

Measured via `.bench/e2e/hermes_z_ab.sh` — 4 standardized coding prompts,
A/B testing with aphrodite ON vs OFF, `focus` directive active:

| Prompt               | OFF (tokens) | ON (tokens) | Saved  | API calls OFF/ON |
|----------------------|-------------:|------------:|-------:|:----------------:|
| Read + summarize     |      444,236 |     178,636 | **60%** | 13 → 7 |
| Cargo build output   |       28,913 |      28,903 |  0%    | 2 → 2 |
| Search codebase      |       68,719 |      30,500 | **56%** | 4 → 2 |
| Read two docs        |       34,075 |      34,049 |  0%    | 2 → 2 |
| **Total**            |  **575,943** | **272,088** | **53%** | 21 → 13 |

Cost: $0.18 (OFF) → $0.15 (ON) — 18% cheaper, 38% fewer API calls.
numbers - run them directly to see current results.

---

## Relationship to Headroom

Aphrodite embeds Headroom - our custom fork tracked as a git submodule at
`vendor/headroom/`. Headroom provides the
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

_Built with ❤️ by PlayForm._
