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
| **Content classifier**    | ✅ 26-type classifier (diff, build, terminal, code, JSON, table…)                       | Generic                  |
| **Preview templates**     | ✅ TOML-driven `[type:key=val]` format, 3 families (compact/balance/code_first)         | ❌                       |
| **Context engine**        | ✅ Async middle-message compression with head/tail protection                           | ❌                       |
| **Auto-expand**           | ✅ Configurable marker resolution (off by default)                                      | ❌                       |
| **HEALTH + Prometheus**   | ✅ `/health` endpoint, `/metrics` → 28 Prometheus metrics                               | Basic only               |
| **Python settings store** | ✅ In-memory mutable store, API-driven reload, hot-reload from TOML                     | ❌                       |
| **Config file watcher**   | ✅ Proxy + plugin auto-detect `aphrodite.toml` changes                                  | ❌                       |
| **Hermes skills**         | ✅ 9 bundled skills for agent operation                                                 | ❌                       |
| **Live container**        | ✅ `APHRODITE_LIVE_CONTAINER` mode for streaming `read_file` via CCR                    | ❌                       |
| **Rhai scripting**        | ✅ Feature-gated hook injection (`--features scripting`)                                | ❌                       |
| **Auto-download binary**  | ✅ Detects platform, downloads from GitHub releases, validates magic bytes              | ❌                       |
| **Concurrency**           | ✅ Multi-worker, shared CCR store, SQLite WAL, token cache                              | Single-worker            |
| **Rust-Python parity**    | ✅ Identical CCR hash, marker format, inline store between Rust proxy and Python plugin | Rust only                |

---

## Part 2: What We Rewrote in Headroom (PlayForm fork)

Headroom is tracked as a git submodule at `vendor/headroom/` and maintained as a
**custom fork** (`github.com/PlayForm/Headroom`). Original upstream is
`github.com/chopratejas/headroom`.

| Area                     | Change                                                                                                                                                                                                |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Branding**             | PlayForm identity, 💋 em-quad spacing, Aphrodite-compatible naming                                                                                                                                    |
| **CCR hash**             | Length 24→40 hex chars for collision safety at scale (millions of entries). Algorithm remains BLAKE3. Python side uses SHA-256 independently - hashes differ but are consistent within each language. |
| **CCR marker**           | `<<<CCR:hash\|type\|size>>>` format shared across Rust and Python                                                                                                                                     |
| **Compression pipeline** | Absorptive preview pipeline - new content types auto-formatted                                                                                                                                        |
| **Tool relay**           | Added Hermes-specific tool relay protocol: `POST /tool/relay` with async callbacks                                                                                                                    |
| **Content types**        | Extended from generic to 26 typed categories with TOML-driven templates                                                                                                                               |
| **API surface**          | Added `/ccr/create`, `/ccr/list`, `/ccr/{hash}` programmatic endpoints                                                                                                                                |
| **Retrieve**             | Full `POST /retrieve` with query filtering + pagination (no zstd decompression - every backend stores/returns content verbatim, so that branch was unreachable dead code and has been removed)        |
| **Multi-proxy**          | Dual-proxy spawn from single TOML (`[[proxies]]` with name/mode/listen)                                                                                                                               |
| **Prometheus**           | 28 metrics, latency histogram, per-type compression counters, EMAs                                                                                                                                    |
| **Build system**         | Version auto-bump, release automation (`scripts/auto-release.sh`)                                                                                                                                     |
| **Testing**              | Smoke test suite, benchmark pipeline, verification checklist                                                                                                                                          |
| **Cargo deps**           | Upgraded to latest (axum 0.7+, reqwest 0.13+, notify 8+)                                                                                                                                              |

---

## Part 3: How They Ship Together

### Binary (Rust proxy + dylib)

The Aphrodite crate produces BOTH a proxy binary and a cdylib from one build:

```
crates/aphrodite/
  ├── src/main.rs          # Proxy binary - spawns :9797 + :9798
  ├── src/lib.rs           # 25 C ABI functions - cdylib for Python loading
  ├── src/proxy.rs         # Handler, compression, tool relay
  ├── src/resolve.rs       # CCR marker resolution (nested, recursive)
  ├── src/stage2.rs        # Semantic reduction (JSON, build, diff, code)
  ├── src/struct_extract.rs # Code structure extraction
  ├── src/hooks.rs         # transform_tool_result, transform_terminal_output
  ├── src/state.rs         # Session state, inline store, LRU
  ├── src/retrieve.rs      # /retrieve endpoint
  └── src/config.rs        # MultiConfig from aphrodite.toml

crates/aphrodite-hermes/   # Hermes-specific integration (cdylib)
  ├── tools.rs             # 13 tool dispatch handlers
  ├── schemas.rs           # JSON Schema definitions
  └── skills.rs            # 9 bundled skills

vendor/headroom/
  └── crates/headroom-core/  # Compression engine (PlayForm fork)
```

**Installation**: The binary is auto-downloaded on first Hermes session start,
or built locally via `cargo build --release`. Location:
`~/.hermes/aphrodite/aphrodite`.

### Python plugin (Hermes integration)

The plugin ships as a directory symlinked into Hermes' profile:

```
~/.hermes/profiles/<profile>/plugins/aphrodite/
  → /path/to/repo/plugins/aphrodite/

Hermes auto-discovers it via `plugin.yaml`:
  - 5 lifecycle hooks (on_session_start, transform_tool_result, …)
  - 13 tools (aphrodite_retrieve, compress, stats, rebuild, …)
  - Context engine (AphroditeContextEngine)
  - TOML-driven config (aphrodite.toml)
```

**Auto-start**: `on_session_start` hook launches both proxies if not already
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

### Updating the vendored fork

There is no dedicated tool for this - it's a checklist, followed in order,
every time `vendor/headroom` gets a pin bump (report 08 F10/T7):

1. **Merge upstream into the fork.** Inside `vendor/headroom` (a separate
   git repo pointed at `PlayForm/Headroom`), merge `upstream/main` into the
   fork's own `Current` branch. Resolve conflicts there, not in the
   superproject - the fork is developed and reviewed independently.
2. **Run the fork's own test suite** (`cargo test --workspace --all-features`
   inside `vendor/headroom`, plus the Python suite if the merge touched
   `headroom/`). Do this _before_ touching the superproject at all - a
   broken fork should never even become a pin-bump candidate.
3. **Make a dedicated pin-bump commit** in the superproject: `git add
vendor/headroom && git commit`. One commit, one purpose - "bump
   headroom-core to <sha>" - never bundled with unrelated Aphrodite changes,
   so a bad bump is a one-line revert.
4. **Run the serde_json feature-parity check**
   (`cargo test -p aphrodite --test serde_json_features`, report 08 F2) -
   this is the regression class that has already bitten this repo once (see
   `HEADROOM-FORK-DIFF.md`'s 2026-07-11 merge section): losing
   `preserve_order`/`arbitrary_precision` feature unification breaks
   SmartCrusher anchor matching with no compile error.
5. **Run `cargo test -p aphrodite`** (the full Aphrodite suite against the
   newly-pinned headroom-core) to catch any API-surface drift the merge
   introduced.
6. **Refresh `HEADROOM-FORK-DIFF.md`'s baseline header** - update the
   "Current baseline" commit SHA/date at the top of the file so the next
   person diffing the fork knows where the last sync landed.

Submodule pins are dependency versions - treat a bump like any other
dependency upgrade: reviewed, isolated, tested before it ships. `git
submodule update --remote` inside an unattended script (see report 08 F3)
is exactly the anti-pattern this checklist exists to prevent.

---

→ **[Complete fork divergence analysis](HEADROOM-FORK-DIFF.md)** - every commit, every deleted file, every modified subsystem between upstream Headroom and our PlayForm fork.
