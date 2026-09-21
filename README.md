<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋

> [!NOTE]
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.
> Up to 610× compression on the standard corpus (132× overall), ~10 ms end-to-end,
> type-aware classifier, TOML-driven, dylib hot-reload.
> _One binary. Zero dependencies. Millions of tokens saved._

[![release](https://img.shields.io/static/v1?label=release&message=v1.5.1&color=blue)](https://github.com/PlayForm/Aphrodite/releases)
[![crates.io](https://img.shields.io/static/v1?label=crates.io&message=aphrodite&color=orange)](https://crates.io/crates/aphrodite)
[![plugin](https://img.shields.io/static/v1?label=plugin&message=v2.1.5&color=purple)](https://github.com/PlayForm/Aphrodite-Hermes/blob/Current/plugin.yaml)
[![rust](https://img.shields.io/static/v1?label=rust&message=1.88%2B&color=orange)](https://www.rust-lang.org)
[![license](https://img.shields.io/static/v1?label=license&message=CC0-1.0&color=lightgrey)](https://github.com/PlayForm/Aphrodite/tree/Development/LICENSE)

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
> Use the Hermes plugin method on Windows too - `download.ps1` is a native
> PowerShell equivalent. See [docs/install/windows.md](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/windows.md).

### Option B: cargo install (standalone binary)

Prefer a source checkout (Option A above) for the Hermes plugin - it is a git
folder by design, and `cargo install` alone does not ship the plugin code.
The two routes are alternatives: if you install via git clone, you do not need
`cargo install`, and vice versa.

**`Terminal`** (binary + config only)

```sh
cargo install aphrodite        # proxy binary
cargo install aphrodite-hermes # dylib + helper bin
aphrodite setup                # config + data dir under ~/.hermes/aphrodite
```

`cargo install` copies only `[[bin]]` targets into `~/.cargo/bin/` and never
links the plugin into Hermes. To use the Hermes plugin, follow Option A (git
clone + `ln -s`); `aphrodite setup` prints the exact link command.

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
context with raw output - compilation logs, accessibility trees, JSON blobs.
The agent spends its budget reading noise instead of reasoning.

Aphrodite intercepts output before it reaches the LLM and replaces it with a
compact, structured preview.
The agent sees ~15 tokens of metadata instead of hundreds - and retrieves the
full content only when it actually needs it.

---

## How It Works ⚙️

**`Pipeline`**

```text
 ANY OUTPUT ────────► Aphrodite ────────► Agent (preview, not raw)
                        │
                        ├─ build log    → [build:2E 5W 210L | error[E0432]: …]
                        ├─ cargo test   → [test:220 pass 0 fail 1 ignored | 0.31s]
                        ├─ terminal     → [terminal:14L exit code: 0]
                        ├─ Rust file    → [code:3fns|2structs fn main() 414L]
                        ├─ Python file  → [code:2fns|1class def handle() 87L]
                        ├─ grep/ripgrep → [grep:4 hits in 3 files | src/x.rs:12 …]
                        ├─ search_files → [search:15 hits in 3 files | src/x.rs:12 …]
                        ├─ git status   → [git:2M 1A 1D 3?? | src/x.rs +N more]
                        ├─ git log      → [gitlog:2 commits | abc1234 fix… → def5678 feat…]
                        ├─ diff         → [diff:2F +7/-3 12L | src/main.rs Cargo.toml]
                        ├─ ls / find    → [ls:12 files 0 dirs | .rs×10]
                        ├─ JSON blob    → [json:30 keys 1L | status, error, …]
                        ├─ web page     → [html:8h 42a 3img 342L | Aphrodite Docs]
                        ├─ compiler err → [error:2L 120B | error: could not compile …]
                        ├─ app log      → [log:12L 540B | [INFO aphrodite] proxy starting]
                        └─ plain text   → [text:3L 50B | first line hint …]

    Agent decides:
    • Preview is enough → skip retrieval, keep reasoning
    • Needs detail      → aphrodite_retrieve(hash) → full content
```

Four fast layers (classification 40-123 ns; whole compress step sub-millisecond):

1. **Classify** - type-aware classifier identifies content.
2. **Preview** - enriched, type-aware previews produced automatically.
3. **Store** - BLAKE3 → SQLite/in-memory → `<<<CCR:hash|type|size>>>` marker.
4. **Decide** - agent reads preview, retrieves only when needed.

The context engine auto-compresses middle turns to CCR as the session fills,
so the agent never hits the context ceiling.

---

## What Gets Compressed 📦

Aphrodite classifies every blob of tool output before compressing it - errors
stay visible while verbose logs get squeezed. The classifier (`preview.rs`,
wrapping the Headroom `content_detector`) returns one type per invocation,
first match wins, and each type carries its own compression threshold tier.

### Output Type Catalog

| Content type                             | Detected from                                                | Enriched preview shape                                                            |
| :--------------------------------------- | :----------------------------------------------------------- | :-------------------------------------------------------------------------------- |
| `build` / `build_output` / `build_error` | `Compiling` / `Finished` / `running` / `test` first line     | `[build:1E 1W 142L \| error[E0432]: unresolved import ...]` (first error message) |
| `diff`                                   | `diff --git` / `@@ -` / `+++` / `---` headers                | `[diff:2F +7/-3 12L \| src/main.rs Cargo.toml +N more]` (first file names)        |
| `git` (status)                           | porcelain status codes (`M`/`A`/`D`/`R`/`??`/`UU`, majority) | `[git:2M 2A 1D 3?? \| src/x.rs src/y.rs +5 more]` (code tallies + paths)          |
| `gitlog`                                 | `commit <hash>` blocks + `Author:`                           | `[gitlog:2 commits \| abc1234 fix… → def5678 feat…]` (first→last hash/subject)    |
| `ls` / `dir`                             | `ls -l` mode strings / bare path tokens (majority)           | `[ls:3 files 2 dirs \| .rs×2 .md×1]` (counts + top extensions)                    |
| `test` / `test_output`                   | `test result:` / `=== RUN` / pytest / jest summaries         | `[test:220 pass 0 fail 1 ignored \| 0.31s]` (or `\| FAIL name` on failure)        |
| `grep` / `ripgrep`                       | `path:line:match` hits (majority)                            | `[grep:4 hits in 3 files \| src/preview.rs:12 …]` (hits, files, first location)   |
| `code_rust`                              | `fn`/`impl`/`struct`/`enum` + (`->` or `&` or `use`)         | `[code:3fns\|2structs fn main() 414L]` (structure map + first signature)          |
| `code_python`                            | `def` + (`import`/`class`/`from`/`self.`)                    | `[code:2fns\|1class def handle() 87L]`                                            |
| `code_go`                                | (`func`/`package`) + `import (`                              | `[code:4fns func main() 210L]`                                                    |
| `code_js` / `code_ts`                    | (`function`/`const`/`=>`) + (`import`/`export`)              | `[code:3fns export default 145L]`                                                 |
| `code` (generic)                         | `fn`/`def`/`class`/`import` fallback                         | `[code:2fns 96L]`                                                                 |
| `search`                                 | search-result JSON (`total_count` + `query`)                 | `[search:15 hits in 3 files \| src/x.rs:12 …]`                                    |
| `json` / `json_array`                    | starts with `{` / `[` and parses as JSON                     | `[json:5items 3L]` or `[json:30 keys 1L \| status, error, …]`                     |
| `tool_output`                            | JSON containing `exit_code` or `"status"`                    | `[tool_output:1L 58B \| {"exit_code": 0, "output": "ok"}]`                        |
| `terminal`                               | shell/exit-code traces (hook override)                       | `[terminal:14L exit code: 0]` (exit code / last output line)                      |
| `error`                                  | first-line `error`/`Error`/`Traceback`/`panic`               | `[error:2L 120B \| error: could not compile \`aphrodite\`]`                       |
| `log`                                    | `[INFO/WARN/ERROR/DEBUG/TRACE]` or timestamp prefix          | `[log:12L 540B \| [2026-09-15T10:00:00Z INFO aphrodite] …]`                       |
| `linter`                                 | `error[E`/`error:`/`warning:`/`clippy`/`tsc`                 | `[linter:4L 260B \| error[E0308]: mismatched types]`                              |
| `text` (fallback)                        | unrecognized content                                         | `[text:3L 50B \| some unrecognizable prose here]` (first non-empty line)          |

> [!NOTE]
>
> When the classifier only reaches a generic bucket (`text`/`terminal`/`log`),
> an Aphrodite-side semantic detector (`detect_semantic_type`) upgrades it to
> the high-signal shapes - `git`, `gitlog`, `grep`, `ls`, `test` - before the
> preview is built. Detection is conservative (line-prefix patterns, majority
> votes), so ordinary prose is never mis-tagged.
> Full taxonomy: [docs/classification/content-types.md](https://github.com/PlayForm/Aphrodite/tree/Development/docs/classification/content-types.md).

### Threshold Tiers

Each type maps to a compression threshold group - the higher the multiplier,
the longer content stays visible in context before being compressed:

| Tier           | Types                                                    |    Multiplier     |
| :------------- | :------------------------------------------------------- | :---------------: |
| Error          | `error`                                                  |        ×8         |
| Code           | `code_rust`, `code_python`, `code_go`, `code_js`, `code` |        ×4         |
| Diff / tracked | `diff`, `git`, `text`                                    |        ×2         |
| Default        | `tool_output`, `json`, everything else                   |        ×1         |
| Noisy (BASE)   | `linter`, `build_output`, `log`                          | ×1 (never halved) |

### Marker Format

Compressed content becomes a CCR marker. Every retrieval entry point
(`aphrodite_retrieve`, `/retrieve`, `resolve`) accepts the marker body
directly, with or without the `|type|size` suffix:

```text
[build:1E 1W 142L | error[E0432]: unresolved import ...]
<<<CCR:0e3a5c9f2b8d4e6a1c3f5b7d9e0a2c4b6d8f0a1e|build|1420>>>
```

- **Hash** - full 40-char lowercase BLAKE3 digest (`[0-9a-f]{40}`).
- **Type** - one of the content types above.
- **Size** - original byte count.
- **Preview** - the enriched preview line above the marker, so the agent can
  decide without retrieving.

---

## Architecture 🏗️

### Two modes

Aphrodite compresses output in **two modes**:

| Mode   | How it intercepts                                                                                                                  | Coverage                                                                          |
| :----- | :--------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------- |
| Plugin | Hermes hooks (`transform_tool_result`, `transform_terminal_output`, context engine) intercept output directly - no API round-trip  | File reads, terminal output, search results, browser snapshots, every tool result |
| Proxy  | Reverse proxy between any client and an LLM API; Chat Completions responses compressed via CCR; tool relay for bidirectional calls | OpenAI-compatible clients, Claude, any LLM API                                    |

### Layout

**`Repository`**

```text
crates/aphrodite/          ← Core engine (binary + cdylib)
  main.rs                  ← CLI entry, version intercept, config bootstrap
  proxy.rs                 ← HTTP proxy: classify → compress → store → preview; SSE pass-through; tool relay
  hooks.rs                 ← transform_tool_result, transform_terminal_output, session/LLM hooks
  preview.rs               ← Type detection + enriched preview builder (all types)
  resolve.rs               ← CCR marker resolution (recursive, nested markers)
  retrieve.rs              ← /retrieve endpoint + retrieval plumbing
  marker.rs                ← Marker parsing, hash normalization, validity checks
  stage2.rs                ← Semantic reduction (JSON minify, build, diff, code)
  struct_extract.rs        ← Code structure extraction (Rust, Python, Go, JS/TS)
  config.rs / config_loader.rs ← TOML schema, hot-reload, multi-proxy resolution
  state.rs                 ← AppState: counters, caches, adaptive EMA state
  directives.rs            ← Behavioral directive registry
  session.rs               ← Session state, turn history
  catalog.rs               ← CCR catalog listing (TOC, tool formats)
  prefetch.rs              ← Background file prefetch → CCR
  poll_worker.rs           ← Auto-backgrounding of slow tool calls
  flow.rs / setup.rs       ← Plugin bootstrap, `aphrodite setup` installer
  builtin_directives/      ← Shipped directive markdown (focus, foresight, cleanup, explore, lazy, ccr-handling)

crates/aphrodite-hermes/   ← Hermes bridge (cdylib)
  lib.rs                   ← FFI surface, hook dispatch
  tools.rs                 ← 13 tool dispatch handlers
  schemas.rs               ← JSON Schema definitions
  skills.rs                ← Bundled Hermes skills
  bin/                     ← Helper binaries

plugins/aphrodite/         ← Thin Python loader (ctypes FFI) + plugin manifest
  __init__.py              ← loads dylib, registers hooks/tools/engine
  plugin.yaml              ← Manifest: 6 hooks, 13 tools, context engine
  download.sh / download.ps1 ← Binary + dylib fetch from GitHub Releases
  directives/              ← Plugin-side directive set
  binaries/                ← Cached prebuilt binaries
```

### Data flow

**`Plugin mode`**

```text
 Tool executes → output intercepted by hook
      ↓
 classify → preview → store (SQLite / in-memory / inline)
      ↓
 Agent ← [type:enriched preview] (not raw output)
      ↓
 aphrodite_retrieve(hash) → full content (only when needed)
```

**`Proxy mode`**

```text
 Client → Aphrodite (:9797 / :9798) → Upstream LLM API
              ↓
     compress Chat Completions response
              ↓
 Client ← CCR markers replace raw content
```

### Dual listeners

| Listener | Port  | CCR backend                      | Threshold | Tool relay | Best for                  |
| :------- | :---: | :------------------------------- | :-------: | :--------: | :------------------------ |
| Cache    | :9797 | In-memory (DashMap, 10K entries) |   >8 KB   |     No     | Speed, transient sessions |
| Token    | :9798 | SQLite (persistent)              |   >1 KB   |    Yes     | Durability, tool relay    |

### Hooks

Six Hermes hooks drive the plugin (`provides_hooks` in `plugin.yaml`):

| Hook                        | Role                                                    |
| :-------------------------- | :------------------------------------------------------ |
| `on_session_start`          | Engine bootstrap, directive seeding, proxy health check |
| `transform_tool_result`     | Compress every tool result before it reaches the LLM    |
| `transform_terminal_output` | Compress terminal output with exit-code context         |
| `pre_llm_call`              | Inject directives, compress overflowing middle turns    |
| `post_llm_call`             | Capture savings, update adaptive thresholds             |
| `pre_tool_call`             | Prefetch-aware dispatch, auto-background slow calls     |

### Management endpoints

The proxy exposes loopback-only management routes (auth via
`APHRODITE_MGMT_TOKEN` when set; `/health` and `/metrics` are exempt):

| Route         | Method | Role                                         |
| :------------ | :----: | :------------------------------------------- |
| `/health`     |  GET   | Liveness probe (public)                      |
| `/stats`      |  GET   | JSON counters, EMA, per-type compression     |
| `/metrics`    |  GET   | Prometheus text format (loopback only)       |
| `/retrieve`   |  POST  | Resolve `<<<CCR:hash\|type\|size>>>` markers |
| `/ccr/create` |  POST  | Programmatic CCR creation                    |
| `/ccr/list`   |  GET   | Catalog listing                              |
| `/ccr/{hash}` | DELETE | Evict an entry                               |
| `/reload`     |  POST  | Hot-reload `aphrodite.toml`                  |
| `/tool/relay` |  POST  | Bidirectional tool relay (token mode)        |

> [!NOTE]
>
> All compression logic lives in the Rust dylib; Python is a thin FFI loader.
> Hot-reload: rebuild the dylib → mtime change detected → next call picks up
> new code automatically.

> [!NOTE]
>
> `plugins/aphrodite/` is a separate repo
> ([PlayForm/Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes)),
> tracked here as a git submodule.

---

## Tools 🔧

Thirteen tools ship with the plugin:

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

---

## Configuration 🎛️

Everything lives in `aphrodite.toml` - no recompile needed.
Edit + save (or `POST /reload`) applies changes immediately.

**`aphrodite.toml`**

```toml
[compression]
tool_threshold_token = 256   # token proxy threshold (bytes)
tool_threshold_cache = 2048  # cache proxy threshold (bytes)
terminal_threshold  = 512    # terminal output threshold (bytes)
inline_threshold    = 1024   # inline-vs-durable CCR storage cutoff (bytes)
code_multiplier     = 3.0    # multiply threshold for code_* content types
```

Each `[compression]` field is overridable via an `APHRODITE_*` env var
(see [docs/config/env-vars.md](https://github.com/PlayForm/Aphrodite/tree/Development/docs/config/env-vars.md)).

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

| Content type              |    Without |    With |  Savings |
| :------------------------ | ---------: | ------: | -------: |
| Git diff (42L)            |   ~350 tok | ~15 tok |  **23×** |
| Build output (142L)       | ~1,400 tok | ~10 tok | **140×** |
| Terminal output           |   ~200 tok | ~10 tok |  **20×** |
| JSON blob (30 keys)       |   ~400 tok | ~10 tok |  **40×** |
| Browser snapshot (342 el) | ~5,000 tok | ~12 tok | **416×** |

**Median: 23× fewer tokens on tool output.**
End-to-end latency is 8-40 ms (includes the HTTP round-trip);
classification alone is 40-123 ns.

Benchmarks are reproducible:
`cargo run --release -p aphrodite --example bench_01_corpus`
(`bench_02_threshold`, `bench_03_retrieve`, `bench_04_ema`).

### Real-world savings (measured, Sep 2026)

Retroactive analysis of 200 real Hermes sessions (6,843 API calls, Sep 18-20) -
multi-agent atomization fan-outs, merge/PR work, plugin integration - by
reconstructing every call's payload from the session transcript: tokens as
stored (CCR markers included) vs. the same payload with every marker expanded
to its original content (stock Hermes).

| Metric                                           |                       Value |
| :----------------------------------------------- | --------------------------: |
| Context tokens with markers (what the model saw) |                 424,959,588 |
| Context tokens expanded (stock Hermes, no CCR)   |                 581,302,405 |
| **Context tokens saved**                         |     **156,342,817 (26.9%)** |
| Average saved per API call                       |                  22,921 tok |
| Sessions that saved anything                     |                   183 / 200 |
| Median session saved ratio                       |                       22.9% |
| Retrieval re-entry (tax)                         | 2,275 calls / 2,011,583 tok |

Real-world per-output compression (orig tokens -> ~33-tok marker):

| Content       | Compression |
| :------------ | ----------: |
| `terminal`    |     **74×** |
| `table`       |     **61×** |
| `source_code` |     **45×** |
| `ls`          |     **37×** |
| `diff`        |     **23×** |
| `build`       |     **23×** |
| `search`      |     **20×** |

Savings scale with tool-output volume and session length - chat-only sessions
gain ~0%, a light developer session ~5%, long tool-heavy sessions 20-25%, and
pure-tool workloads (simulated diff+ls+build+search loop) **76-82%**.

> [!NOTE]
>
> **Confidence.** The causal result - CCR removes context pressure for large
> repeated tool outputs, and tool-heavy usage benefits far more than chat -
> is high. The exact percentage is a band: the marker's byte size is the only
> trace of the original content (the inline store is session-scoped), so
> savings were estimated with a calibrated chars/tok ratio; sensitivity across
> 3.0-4.5 chars/tok spans ~24-33%, centered ~27%. A paired pre/post request
> capture in the proxy would convert this estimate into an exact measurement.

---

## Relationship to Headroom 🔗

Aphrodite embeds [Headroom](https://github.com/PlayForm/headroom) - a custom fork tracked
as a git submodule at `vendor/headroom/`.
Headroom provides the content transforms (classifier, smart crusher, tokenizer);
Aphrodite adds the preview pipeline, CCR storage, Hermes integration, and
dual-proxy architecture.

→ See [docs/architecture/10-component.md](https://github.com/PlayForm/Aphrodite/tree/Development/docs/architecture/10-component.md) for how the fork integrates.

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

Released under [CC0-1.0](https://github.com/PlayForm/Aphrodite/tree/Development/LICENSE) - public domain.

---

_Built with ❤️ by PlayForm._

[Aphrodite]: https://github.com/PlayForm/Aphrodite
