<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="720">
</p>

---

# [Aphrodite] 💋

> [!NOTE]
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.
> Up to 610× compression on the standard corpus (132× overall), ~10 ms end-to-end,
> type-aware classifier, TOML-driven, dylib hot-reload.
> _One binary. Zero dependencies. Millions of tokens saved._

[![release](https://img.shields.io/static/v1?label=release&message=v1.3.9&color=blue)](https://github.com/PlayForm/Aphrodite/releases)
[![crates.io](https://img.shields.io/static/v1?label=crates.io&message=aphrodite&color=orange)](https://crates.io/crates/aphrodite)
[![plugin](https://img.shields.io/static/v1?label=plugin&message=v2.0.10&color=purple)](https://github.com/PlayForm/Aphrodite-Hermes/blob/Current/plugin.yaml)
[![rust](https://img.shields.io/static/v1?label=rust&message=1.88%2B&color=orange)](https://www.rust-lang.org)
[![license](https://img.shields.io/static/v1?label=license&message=CC0-1.0&color=lightgrey)](LICENSE)

---

## Install ⚡

Aphrodite ships as a Hermes plugin (Rust dylib + standalone proxy binary).
No Rust toolchain is required for the common path.

### As a Hermes plugin (recommended)

**`Terminal`**

```sh
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/plugins/aphrodite
hermes plugins enable aphrodite
hermes
```

On first launch the plugin auto-downloads the `aphrodite` binary from
[releases](https://github.com/PlayForm/Aphrodite/releases).

> [!IMPORTANT]
>
> Use the Hermes plugin method on Windows too — `download.ps1` is a native
> PowerShell equivalent. See [docs/install/windows.md](docs/install/windows.md).

### Via cargo

**`Terminal`**

```sh
cargo install aphrodite          # proxy binary
cargo install aphrodite-hermes   # dylib + helper bin
aphrodite setup                  # plugin structure + config + symlink
```

`cargo install` copies only `[[bin]]` targets into `~/.cargo/bin/`.
The `libaphrodite_hermes` dylib must come from a source checkout or the
release-download flow above.

### From source

**`Terminal`**

```sh
git clone https://github.com/PlayForm/Aphrodite.git
cd Aphrodite
git submodule update --init --recursive
cargo build --release -p aphrodite -p aphrodite-hermes
```

---

## The Problem 🔥

Every file read, build, code search, or browser open floods the agent's
context with raw output — compilation logs, accessibility trees, JSON blobs.
The agent spends its budget reading noise instead of reasoning.

Aphrodite intercepts output before it reaches the LLM and replaces it with a
compact, structured preview.
The agent sees ~15 tokens of metadata instead of hundreds — and retrieves the
full content only when it actually needs it.

---

## How It Works ⚙️

**`Pipeline`**

```text
 ANY OUTPUT ──────► Aphrodite ──────► Agent (preview, not raw)
                       │
                       ├─ build logs  → [build:1E 1W 142L | error[E0432]: …]
                       ├─ terminal    → [terminal:14L exit code: 0]
                       ├─ file read   → [code:3fns|2structs fn main() 414L]
                       ├─ grep/ripgrep→ [grep:4 hits in 3 files | src/x.rs:12 …]
                       ├─ git status  → [git:2M 1A 1D 3?? | src/x.rs +N more]
                       ├─ diff        → [diff:2F +7/-3 12L | src/main.rs Cargo.toml]
                       └─ plain text  → [text:3L 50B | first line hint …]

    Agent decides:
    • Preview is enough → skip retrieval, keep reasoning
    • Needs detail      → aphrodite_retrieve(hash) → full content
```

Four fast layers (classification 40–123 ns; whole compress step sub-millisecond):

1. **Classify** — type-aware classifier identifies content.
2. **Preview** — enriched, type-aware previews produced automatically.
3. **Store** — BLAKE3 → SQLite/in-memory → `<<<CCR:hash|type|size>>>` marker.
4. **Decide** — agent reads preview, retrieves only when needed.

The context engine auto-compresses middle turns to CCR as the session fills,
so the agent never hits the context ceiling.

---

## Architecture 🏗️

**`Layout`**

```text
crates/aphrodite/          ← Core engine (binary + cdylib)
  proxy.rs                 ← HTTP proxy: classify → compress → store → preview
  hooks.rs                 ← transform_tool_result, transform_terminal_output
  resolve.rs               ← CCR marker resolution (recursive)
  stage2.rs                ← Semantic reduction (JSON, build, diff, code)
  struct_extract.rs        ← Code structure extraction (Rust, Python, Go, JS/TS)

crates/aphrodite-hermes/   ← Hermes bridge (cdylib)
  tools.rs                 ← 14 tool dispatch handlers
  schemas.rs               ← JSON Schema definitions
  skills.rs                ← Bundled Hermes skills

plugins/aphrodite/         ← Thin Python loader (ctypes FFI)
  __init__.py              ← loads dylib, registers hooks/tools/engine
```

| Mode  | Port  | Backend   | Threshold | Best for                  |
| :---- | :---: | :-------- | :-------: | :------------------------ |
| Cache | :9797 | In-memory |   >8 KB   | Speed, transient sessions |
| Token | :9798 | SQLite    |   >1 KB   | Durability, tool relay    |

All compression logic lives in the Rust dylib; Python is a thin FFI loader.
Hot-reload: rebuild the dylib → mtime change detected → next call picks up new
code automatically.

> [!NOTE]
>
> `plugins/aphrodite/` is a separate repo
> ([PlayForm/Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes)),
> tracked here as a git submodule.

---

## Tools 🔧

| Tool                        | Description                                              |
| :-------------------------- | :------------------------------------------------------- |
| `aphrodite_retrieve`        | Resolve `<<<CCR:hash\|type\|size>>>` markers             |
| `aphrodite_compress`        | Compress content via CCR with type hint                  |
| `aphrodite_stats`           | Proxy health, engine status, inline store size           |
| `aphrodite_rebuild`         | Rebuild binary, kill proxies, restart                    |
| `aphrodite_files`           | Tracked file references, grouped by tool                 |
| `aphrodite_diff`            | Conversation turn history with summaries                 |
| `aphrodite_search`          | Search CCR store by keyword or type                      |
| `aphrodite_directive`       | List/swap/add/remove/reset behavioral directives         |
| `aphrodite_test`            | Smoke test suite: quick (1 check), full (3 checks)       |
| `aphrodite_catalog`         | Full CCR catalog with hashes, types, sizes, previews     |
| `aphrodite_reclassify`      | Retroactive metadata enrichment for unclassified CCR     |
| `aphrodite_prefetch`        | Read + compress files on demand; markers returned inline |
| `aphrodite_prefetch_status` | Live prefetch schedule: loading, ready, errors           |
| `aphrodite_navigate`        | S2 context navigation: zoom into stored recall index     |

---

## Configuration 🎛️

Everything lives in `aphrodite.toml` — no recompile needed.
Edit + save (or `POST /reload`) applies changes immediately.

**`aphrodite.toml`**

```toml
[compression]
tool_threshold_token = 256   # token proxy threshold (bytes)
tool_threshold_cache = 2048  # cache proxy threshold (bytes)
terminal_threshold  = 512    # terminal output threshold (bytes)
inline_threshold    = 1024  # inline-vs-durable CCR storage cutoff (bytes)
code_multiplier     = 3.0    # multiply threshold for code_* content types
```

Each `[compression]` field is overridable via an `APHRODITE_*` env var.

> [!TIP]
>
> **Directives** seed short behavioral instructions injected each turn,
> swappable mid-conversation via `aphrodite_directive`.
> Shipped set: `focus`, `foresight`, `cleanup`, `explore`, `lazy-eval`.

---

## Performance 📊

Standard corpus: up to **610×** on large low-entropy prose, **132×** overall
(106 KB → 800 B).
Cache and token modes measure identical ratios;
20/20 compressed, 20/20 retrieve round-trips OK.

| Content type              | Without     | With        |  Savings |
| :------------------------ | ----------: | ----------: | -------: |
| Git diff (42L)            |   ~350 tok  |    ~15 tok  |  **23×** |
| Build output (142L)       | ~1,400 tok  |    ~10 tok  | **140×** |
| Terminal output           |   ~200 tok  |    ~10 tok  |  **20×** |
| JSON blob (30 keys)       |   ~400 tok  |    ~10 tok  |  **40×** |
| Browser snapshot (342 el) | ~5,000 tok  |    ~12 tok  | **416×** |

**Median: 23× fewer tokens on tool output.**
End-to-end latency is 8–40 ms (includes the HTTP round-trip);
classification alone is 40–123 ns.

Benchmarks are reproducible: `cargo run --release -p aphrodite --example bench_0N_*`.

---

## Relationship to Headroom 🔗

Aphrodite embeds [Headroom](docs/APHRODITE-HEADROOM.md) — a custom fork tracked
as a git submodule at `vendor/headroom/`.
Headroom provides the content transforms (classifier, smart crusher, tokenizer);
Aphrodite adds the preview pipeline, CCR storage, Hermes integration, and
dual-proxy architecture.

→ [Full comparison: Aphrodite vs Headroom](docs/APHRODITE-HEADROOM.md)

---

## Contributing 🤝

| Want to…          | Start here                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------ |
| Report a bug      | [Open an issue](https://github.com/PlayForm/Aphrodite/issues/new?template=bug_report.md)   |
| Suggest a feature | [Start a discussion](https://github.com/PlayForm/Aphrodite/discussions/new?category=ideas) |
| Submit a PR       | [Fork & open a PR](https://github.com/PlayForm/Aphrodite/pulls)                            |
| Ask a question    | [Discussions Q&A](https://github.com/PlayForm/Aphrodite/discussions/new?category=q-a)      |

No contribution is too small.
First-time contributors are especially welcome.

---

## License 📜

Released under [CC0-1.0](LICENSE) — public domain.

---

_Built with ❤️ by PlayForm._

[Aphrodite]: https://github.com/PlayForm/Aphrodite
