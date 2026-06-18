# Aphrodite & Headroom

Aphrodite is the primary product. Headroom is our fork - a custom, modified
dependency that Aphrodite extends.

---

## Part 1: What Aphrodite Adds (on top of Headroom)

| Layer                     | Aphrodite                                                                               | Headroom (stock)         |
| ------------------------- | --------------------------------------------------------------------------------------- | ------------------------ |
| **Hermes plugin**         | ✅ Full Hermes Agent plugin - hooks, 13 tools, context engine, smoke tests              | ❌ No Hermes integration |
| **Proxy binaries**        | ✅ Dual-proxy mode (:9797 cache + :9798 token) with TOML config                         | Single proxy, CLI only   |
| **Tool relay**            | ✅ Bidirectional `/tool/relay` - LLM calls Hermes tools through proxy                   | ❌                       |
| **CCR endpoints**         | ✅ `/ccr/create`, `/ccr/list`, `/ccr/{hash}` REST API                                   | Basic CCR store          |
| **Content classifier**    | ✅ 28‑type classifier (diff, build, terminal, code, JSON, table…)                       | Generic                  |
| **Preview templates**     | ✅ TOML‑driven `[type:key=val]` format, 3 families (compact/balance/code_first)         | ❌                       |
| **Context engine**        | ✅ Async middle‑message compression with head/tail protection                           | ❌                       |
| **Auto‑expand**           | ✅ Configurable marker resolution (off by default)                                      | ❌                       |
| **HEALTH + Prometheus**   | ✅ `/health` endpoint, `/metrics` → 31 Prometheus metrics                               | Basic only               |
| **Python settings store** | ✅ In‑memory mutable store, API‑driven reload, hot‑reload from TOML                     | ❌                       |
| **Config file watcher**   | ✅ Proxy + plugin auto‑detect `aphrodite.toml` changes                                  | ❌                       |
| **Hermes skills**         | ✅ 13 bundled skills for agent operation                                                | ❌                       |
| **Live container**        | ✅ `APHRODITE_LIVE_CONTAINER` mode for streaming `read_file` via CCR                    | ❌                       |
| **Rhai scripting**        | ✅ Feature‑gated hook injection (`--features scripting`)                                | ❌                       |
| **Auto‑download binary**  | ✅ Detects platform, downloads from GitHub releases, validates magic bytes              | ❌                       |
| **Concurrency**           | ✅ Multi‑worker, shared CCR store, SQLite WAL, token cache                              | Single‑worker            |
| **Rust‑Python parity**    | ✅ Identical CCR hash, marker format, inline store between Rust proxy and Python plugin | Rust only                |

---

## Part 2: What We Rewrote in Headroom (PlayForm fork)

Headroom is tracked as a git submodule at `vendor/headroom/` and maintained as a
**custom fork** (`github.com/PlayForm/headroom`). Original upstream is
`github.com/chopratejas/headroom`.

| Area                     | Change                                                                             |
| ------------------------ | ---------------------------------------------------------------------------------- |
| **Branding**             | PlayForm identity, 💋 em‑quad spacing, Aphrodite‑compatible naming                 |
| **CCR hash**             | BLAKE3 → SHA‑256 (24‑char hex) for Rust‑Python parity                              |
| **CCR marker**           | `<<<CCR:hash\|type\|size>>>` format shared across Rust and Python                  |
| **Compression pipeline** | Absorptive preview pipeline - new content types auto‑formatted                     |
| **Tool relay**           | Added Hermes‑specific tool relay protocol: `POST /tool/relay` with async callbacks |
| **Content types**        | Extended from generic to 28 typed categories with TOML‑driven templates            |
| **API surface**          | Added `/ccr/create`, `/ccr/list`, `/ccr/{hash}` programmatic endpoints             |
| **Retrieve**             | Full `POST /retrieve` with zstd decompression + query filtering + pagination       |
| **Multi‑proxy**          | Dual‑proxy spawn from single TOML (`[[proxies]]` with name/mode/listen)            |
| **Prometheus**           | 31 metrics, latency histogram, per‑type compression counters, EMAs                 |
| **Build system**         | Version auto‑bump, release automation (`scripts/auto-release.sh`)                  |
| **Testing**              | Smoke test suite, benchmark pipeline, verification checklist                       |
| **Cargo deps**           | Upgraded to latest (axum 0.7+, reqwest 0.13+, notify 8+)                           |

---

## Part 3: How They Ship Together

### Binary (Rust proxy)

The Aphrodite binary is a single executable that embeds Headroom‑core as a
library dependency:

```
crates/aphrodite/
  ├── src/main.rs          # Proxy binary - spawns :9797 + :9798
  ├── src/proxy.rs         # Handler, compression, tool relay
  ├── src/retrieve.rs      # /retrieve endpoint
  └── src/config.rs        # MultiConfig from aphrodite.toml

vendor/headroom/
  └── crates/headroom-core/  # Compression engine (PlayForm fork)
```

**Installation**: The binary is auto‑downloaded on first Hermes session start,
or built locally via `cargo build --release`. Location:
`~/.hermes/aphrodite/aphrodite`.

### Python plugin (Hermes integration)

The plugin ships as a directory symlinked into Hermes' profile:

```
~/.hermes/profiles/<profile>/plugins/aphrodite/
  → /path/to/repo/plugins/aphrodite/

Hermes auto‑discovers it via `plugin.yaml`:
  - 5 lifecycle hooks (on_session_start, transform_tool_result, …)
  - 13 tools (aphrodite_retrieve, compress, stats, rebuild, …)
  - Context engine (AphroditeContextEngine)
  - TOML‑driven config (aphrodite.toml)
```

**Auto‑start**: `on_session_start` hook launches both proxies if not already
running, verifies version, and wires the context engine.

### Hermes sees

```
Session start
  → Hermes loads plugin (symlink resolve)
  → on_session_start hook fires
    → _ensure_binary() - download or build
    → Launch :9797 cache proxy
    → Launch :9798 token proxy
    → Verify health
    → Reload config
  → Context engine active
  → Tools registered
  → Hooks wired
```
