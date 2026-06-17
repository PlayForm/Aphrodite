<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> **Your LLM burns 90% of its context on tool output it never reads. We fix that.**
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.  
> Sub‑ms compress, 12,800× max ratio, 28‑type classifier, TOML‑driven.  
> *One binary. Zero dependencies. 12.5M tokens saved.*

[![release](https://img.shields.io/badge/release-v0.5.123-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.14-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## The Problem

Every time your agent runs a tool — `cargo build`, `search_files`, `read_file` — the raw output floods its context window. Thousands of tokens of compilation logs. Gigantic accessibility trees. Verbose JSON blobs. Your agent spends its precious context budget **reading noise** instead of reasoning.

**Aphrodite intercepts tool output before it reaches the LLM and replaces it with a compact, structured preview.** The agent sees 15 tokens of metadata instead of 500 tokens of raw text — and retrieves the full content only when it actually needs it.

---

## How It Works ⚙️

```
| Your Agent                  Aphrodite Proxy              LLM API
|    │                            │                          │
|    │──── tool call ────────────►│                          │
|    │                            │── forward to API ───────►│
|    │                            │◄── API response ────────│
|    │                            │                          │
|    │                    ┌───────┴───────┐                  │
|    │                    │ 28-type       │                  │
|    │                    │ classifier    │                  │
|    │                    │    ↓          │                  │
|    │                    │ TOML template │                  │
|    │                    │    ↓          │                  │
|    │                    │ CCR store     │                  │
|    │                    └───────┬───────┘                  │
|    │                            │                          │
|    │◄── [build:0E 0W 1L] ─────│  15 tokens, not 500       │
|    │                            │                          │
|    │── aphrodite_retrieve() ──►│  only when needed        │
|    │◄── full content ──────────│                          │
|    │                            │                          │
|    │── aphrodite_prefetch() ──►│  background reads        │
|    │◄── markers instantly ─────│  files load concurrently │
```

Three steps, all under 1ms:
1. **Classify** — 28-type regex classifier identifies what the output is
2. **Template** — TOML‑driven templates produce a compact `[type:key=val]` preview
3. **Store** — SHA-256 hash → SQLite/in‑memory → `<<<CCR:hash|type|size>>>` marker

---

## Architecture 🏗️

| Mode  | Port  | Backend | Threshold | Best for |
| :---- | :---: | :------ | :-------: | :------- |
| Cache | :9797 | In‑memory | >8 KB | Speed, transient sessions |
| Token | :9798 | SQLite | >1 KB | Durability, tool relay |

Both modes share the same classifier, template engine, and preview pipeline. The difference is persistence — cache mode is ephemeral, token mode survives restarts.

---

## Agent Compatibility 🤝

Aphrodite sits as an OpenAI-compatible proxy — any agent that speaks the OpenAI API can route through it. Below are the agents we've verified or that have clean integration paths.

### Direct Integration (OpenAI-compatible proxy)

| Agent | Type | Integration | Compression |
|-------|------|-------------|-------------|
| **Hermes Agent** | Native | Built-in plugin; `on_start()` auto-launches | Full pipeline |
| **Aider** | Open source | `--openai-api-base http://127.0.0.1:9798/v1` | High — context-heavy diffs |
| **OpenHands** | Open source (MIT) | Custom provider config; clean agent loop | High — multi-level compression |
| **Codex CLI** | Open source (Apache 2.0) | Pluggable provider; `--model` + `--base-url` | Medium-high |
| **Cline / Roo Code** | Open source (VS Code) | MCP server or custom API endpoint | Medium-high — MCP protocol |
| **Continue.dev** | Open source (VS Code/JB) | Custom provider config; model-agnostic | Medium |
| **Sourcegraph Cody** | Enterprise | Self-host; custom endpoint config | Medium |
| **PostHog Code** | Analytics | Configurable endpoint | Low-medium — niche |
| **Qodo** | Enterprise | Proxy insertion in deployment pipeline | Medium |

### MCP-Native Integration

| Agent | Type | Integration |
|-------|------|-------------|
| **Cline / Roo Code** | VS Code extension | Aphrodite as MCP server for context services |
| **Cloudflare Agents SDK** | Edge platform | AI Gateway interception; worker-based |
| **Vercel AI SDK** | Framework | `wrapLanguageModel` middleware; custom provider |

### Future SDK Targets

| SDK | Potential | Path |
|-----|-----------|------|
| **Vercel AI SDK** | High | Custom provider + middleware wrapping |
| **Cloudflare Agents SDK** | Medium | AI Gateway + Durable Objects compression |
| **MCP Protocol** | High | Standardized context compression service |
| **OpenAI Agents SDK** | Medium | Custom provider via `base_url` override |

> **Context window management is the #1 cost bottleneck across ALL coding agents.** Aphrodite's compression addresses this universally — any agent that makes tool calls benefits from 23× median token savings on tool output.

---

## Absorptive CCR Previews 🧠

> **"Absorptive" means the classifier learns from every output it sees. New content of the same type automatically gets the same structured treatment — no manual template writing needed.**

### Without Aphrodite → With Aphrodite

| Content type | Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----------- | :-------------------------------- | :----------------------------- |
| Git diff | raw unified diff header | `[diff:1f +12/-3 42L foo.rs]` |
| Build output | raw compile lines + errors | `[build:1E 2W 142L]` |
| Traceback | raw Python traceback | `[error:AttributeError 'NoneType']` |
| Terminal | raw stdout + exit code | `[terminal:cargo build exit=0]` |
| Git log | raw commit list | `[commit:afd634b release(v0.5.104)]` |
| Rust error | raw error[E] block | `[error:E0308 src/main.rs:10:5 8L]` |
| Table output | raw pipe-delimited text | `[table:12 rows 15L]` |
| JSON blob | raw nested JSON | `[json:total_items,by_type 30L]` |
| Web search | raw result array | `[search:10 results 'rust tokio' 5L]` |
| Browser snapshot | raw accessibility tree | `[dom:342 elements 500 total]` |
| Console log | raw log array | `[log:42 entries 3E 5W 100L]` |
| File write | raw confirmation JSON | `[file:src/main.rs 1234B]` |
| Code (Rust) | raw file content | `[code_rust:3fns, 2structs, 1impl] fn format(...); fn build(...) 200L` |
| Image gen | raw prompt + URL | `[image:a photorealistic cat...]` |
| Todo list | raw task array | `[todo:5 items 3 pending]` |
| Memory ops | raw memory store | `[memory:3 entries]` |
| Cron jobs | raw job config | `[cron:active]` |
| Plain text | raw text | `[text:first 110 chars...]` |

### The agent's thought process

```
Without Aphrodite:
  "I see 500 tokens of build output... scrolling... error? no... warning? no...
   ok it passed. That was 500 tokens I'll never get back."

With Aphrodite:
  "[build:0E 0W 1L] — clean build. Next task."
  15 tokens. Agent keeps reasoning.
```

### Classifier — 28 content types

```
content → _classify_content()
          ├─ diff              (unified diff headers)
          ├─ terminal          (exit code patterns)
          ├─ build_output      (Compiling/Finished/test lines)
          ├─ build_error       (error[E] codes + locations)
          ├─ error             (Traceback/panic/Error:)
          ├─ commit            (hex hash + subject)
          ├─ json              (top-level keys extraction)
          ├─ json_list         (array item count)
          ├─ search_files      (file:line: match patterns)
          ├─ search_results    (JSON with total_count/query)
          ├─ tabular           (| in ≥3 lines)
          ├─ process_output    (session_id + pid + uptime)
          ├─ write_file        (status=written, path, bytes)
          ├─ log               (level/message array with error/warn counts)
          ├─ browser_snapshot  (elements array, total_elements)
          ├─ web_search        (title+url or results array)
          ├─ image_generate    (image/prompt keys)
          ├─ todo              (todos array with statuses)
          ├─ memory            (success+target/entries)
          ├─ cronjob           (schedule+id, status)
          ├─ code_rust         (fn, struct, impl extraction)
          ├─ code_python       (def, class, import extraction)
          ├─ code_go           (func, type, interface extraction)
          ├─ code_js           (function, class, => arrow extraction)
          ├─ code_ts           (interface + JS patterns)
          ├─ code_sh           (#!/ + shell patterns)
          ├─ code              (generic language detection)
          └─ text              (fallback)
          ↓
_make_ccr_preview() → {family}:{template}  (≤120 chars, pipe‑safe)
```

### Why `[type:...]` format

- **LLM‑native** — no emojis, no decorative separators. Pure structured text the model can parse instantly.
- **Scannable** — the type prefix lets the LLM pattern‑match across dozens of markers in a single pass.
- **Compact** — `[diff:1f +12/-3 42L foo.rs]` vs 120‑char raw dump. 8× smaller on average.
- **Actionable** — enough detail to decide "retrieve" vs "skip" without expanding.
- **Model‑aware** — 3 template families adapt preview style per model (compact for Claude, code_first for DeepSeek, balance for GPT).

---

## LLM‑Native Output Formatting 📋

> **Your agent reads clean, machine‑parseable text instead of decorative human‑facing output with emojis and bold formatting. Every token counts.**

Aphrodite's own tools (`catalog`, `stats`, `diff`, `files`) return structured output designed for LLM consumption — no emojis, no markdown decoration, no wasted tokens.

### Without Aphrodite → With Aphrodite

**`aphrodite_catalog`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `📦 Aphrodite Catalog — 2 items · 4.8KB saved` | `Catalog: 2 items 4.8KB saved 2 turns 0 files` |
| Raw JSON blob | Markdown table with hash, type, size, preview |

**`aphrodite_stats`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `💋 Aphrodite Stats` / `✅ active` | `Aphrodite Stats` / `on` / `off` |

**`aphrodite_diff`** / **`aphrodite_files`** — same principle: structured over decorated.

---

## What You Save 💰

> **Every token of tool output you compress is a token your agent can use for reasoning, planning, and code generation. Context is the most expensive resource in LLM economics — we make it go further.**

### Your agent's context budget

| Content type | Without Aphrodite | With Aphrodite | Savings |
| :----------- | ----------------: | -------------: | ------: |
| Git diff (42L) | ~350 tok | ~15 tok | **23×** |
| Build output (142L) | ~1,400 tok | ~10 tok | **140×** |
| Traceback | ~45 tok | ~12 tok | **3.8×** |
| Terminal output | ~200 tok | ~10 tok | **20×** |
| Git log (8 commits) | ~100 tok | ~20 tok | **5×** |
| Table (50 rows) | ~650 tok | ~8 tok | **81×** |
| JSON blob (30 keys) | ~400 tok | ~10 tok | **40×** |
| Web search (10 results) | ~800 tok | ~15 tok | **53×** |
| Browser snapshot (342 el) | ~5,000 tok | ~12 tok | **416×** |
| Console log (42 entries) | ~1,200 tok | ~15 tok | **80×** |

**Median: 23× fewer tokens burned on tool output.** In a typical coding session with 50+ tool calls, that's **15,000–50,000 tokens saved** — enough for an entire extra conversation turn of reasoning.

### Features that save context

| Feature | What it does | Real-world impact |
|---------|-------------|-------------------|
| Classifier poll | Suppresses CCR for clean outputs (0E/0W, exit=0) | Silent builds don't waste a single token |
| Code structure‑map | LLM sees fn/struct/class sigs, not raw code | Agent navigates 500‑line files without retrieving them |
| Batch tool calls | Multiple independent tools execute concurrently in one turn | 2–3× faster task completion without extra round-trips |
| Background prefetch | File read + compress runs in daemon threads | Large files load concurrently; agent continues reasoning immediately |
| Model‑aware templates | Preview style adapts to model family | Claude gets compact metadata, DeepSeek gets code excerpts |
| TOML‑driven config | All thresholds + templates in one file, no recompile | Tune aggressiveness in 30 seconds |
| Context engine | Compresses middle conversation turns to CCR | Long sessions stay within context window automatically |

---

## Performance ⚡

> **Sub‑millisecond compression with zero API calls. The classifier runs in <0.1ms using pure regex — no model inference, no network round‑trip, no token cost.**

### Proxy benchmark *(verified 2026-06-17)*

| Size   | Text    | Code   | JSON   | Ratio      | Tokens Saved |
| :----- | :-----: | :----: | :----: | :--------: | :----------: |
| 1 KB   | 0.4ms   | 0.3ms  | 0.5ms  | 26×        | 240          |
| 10 KB  | 0.6ms   | 0.7ms  | 3.5ms* | 256×       | 2,500        |
| 50 KB  | 0.7ms   | 0.6ms  | 1.0ms  | 1,280×     | 12,800       |
| 100 KB | 1.1ms   | 1.0ms  | 1.1ms  | 2,560×     | 25,600       |
| 500 KB | 2.1ms   | 7.9ms* | 2.8ms  | 12,800×    | 128,000      |

*Outliers: single slow iteration in 5-iteration average. p95 ≤ 19.9ms.

| Metric                       | Value          |
| :--------------------------- | :------------: |
| Compression latency (avg)    | 1.6 ms         |
| Compression latency (min)    | 0.3 ms         |
| Retrieval latency (avg)      | 0.7 ms         |
| Retrieval p50                | 0.4 ms         |
| Benchmark pass rate          | 19/19 ✅        |
| Smoke test pass rate         | 13/13 ✅        |
| Classification latency       | <0.1 ms        |
| Preview generation           | <0.05 ms       |
| Worker threads (default)     | 4× CPU, min 32 |

### Cumulative proxy savings *(running session)*

| Metric                     | Value            |
| :------------------------- | :--------------: |
| Total tokens saved         | **12.5M**        |
| Requests compressed        | 165 / 412        |
| Body bytes saved           | **67%**          |
| Request bytes              | 60.6 MB          |
| Response bytes             | 20.1 MB          |
| Average saved/compression  | 75,528 tokens    |
| Per benchmark run          | 507,000 tokens   |
| CCR entries                | 165              |
| Cache hits                 | 20               |

### Real‑world token savings

> **2 MB of tool output compresses to ~960 bytes of CCR markers — a 3,000:1 effective ratio. In a typical coding session with 50+ tool calls, that's 15,000–500,000 tokens saved per session. The proxy has saved 12.5 million tokens cumulatively — that's $250+ in API costs at current pricing alone.**

### Classifier coverage

| Metric                  | Value          |
| :---------------------- | :------------: |
| Content types detected  | 28             |
| JSON sub‑types          | 12             |
| Code languages          | 6              |
| Template families       | 3              |
| Classification speed    | <0.1 ms        |

---

## Compression Strategies 🧬

> **Aphrodite owns the *addressing* layer — where content lives and how to find it. Headroom owns the *reduction* layer — how to make content smaller while keeping it meaningful.**

Aphrodite's CCR layer is **content-addressed storage** — every piece of content gets a SHA-256 hash and lives in SQLite or in‑memory. That gives us deduplication (identical content = one hash) but not semantic reduction.

[Headroom] (partner proxy at :9799) brings the semantic layer — specialized compressors that understand *what* the content is and reduce it intelligently.

[Headroom]: https://github.com/chopratejas/headroom

### Headroom compressor roster

| Strategy       | Target      | Technique                          | Ratio  |
| :------------- | :---------- | :--------------------------------- | :----: |
| `CODE_AWARE`   | Source code | tree-sitter AST — signatures kept, bodies compressed | 5–8× |
| `SMART_CRUSHER`| JSON arrays | Structural dedup                   | —      |
| `SEARCH`       | grep output | Dedup + summarize matches           | —      |
| `LOG`          | Build/test  | Error/warning extraction            | —      |
| `KOMPRESS`     | Free text   | ML-based semantic                   | 3–5×   |
| `DIFF`         | Git diffs   | File-level summary                  | —      |
| `HTML`         | Web content | Tag-aware                          | —      |
| `MIXED`        | Chat output | Split → route → reassemble          | —      |
| `PASSTHROUGH`  | Sub-threshold | Identity                           | 1×     |

### The full picture

```
content → Aphrodite (address + store)
            SHA-256 → cache check → store → <<<CCR:hash>>>
              ↓
          Headroom (understand + reduce)
            classify → route → AST/log/ML compress → smaller content
              ↓
          Aphrodite (preview + retrieve)
            structured template → [code:3fns|2structs] → LLM browses
```

Together: **content-addressed retrieval + semantic reduction + structure-aware previews.** Each layer does what it does best, and the LLM only pays for what it actually needs.

---

## Quick Start 🚀

> **30 seconds from clone to compression.**

```bash
# 1. Build (one command)
cargo build --release -p aphrodite

# 2. Run (both proxies start automatically)
aphrodite

# 3. Verify
curl http://127.0.0.1:9798/health
# → {"status":"ok","version":"v0.5.121"}

# Dev loop with auto-reload
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Configuration — everything in one file

```toml
# aphrodite.toml — all features, no recompile needed
[compression]
engine_threshold_pct = 45    # compress at 45% context
tool_threshold_token = 512   # token proxy threshold (bytes)
classifier_poll = true       # suppress CCR for clean outputs
context_engine = true        # engine on by default

[previews]
model_family = "code_first"  # compact | code_first | balance
code_structure_map = true    # show fn/struct/class sigs

[prompts]
retrieve_guidance = "minimal" # no retrieval bait in markers
ccr_marker_hint = false      # clean CCR markers
```

7 TOML sections, 54 template strings, all overridable via `APHRODITE_*` env vars. No recompile. No restart. Just edit and go.

---

### Prefetch Workflow ⚡

> **Read many files in parallel. The agent gets CCR markers instantly and continues reasoning while files load in background threads.**

```bash
# Batch-read files concurrently — markers return immediately
aphrodite_prefetch(paths=["src/main.rs", "src/lib.rs", "Cargo.toml"])

# Response (instant):
# {
#   "prefetching": 3,
#   "markers": [
#     {"hash": "a1b2c3d4e5f6", "path": "src/main.rs", "type": "code_rust", "size": 12450},
#     {"hash": "b2c3d4e5f6a7", "path": "src/lib.rs", "type": "code_rust", "size": 8320},
#     {"hash": "c3d4e5f6a7b8", "path": "Cargo.toml", "type": "text", "size": 420}
#   ],
#   "note": "Files loading in background. Use TOC to check, aphrodite_retrieve(hash) to fetch."
# }

# Later, when the agent needs the content:
# aphrodite_retrieve(hash="a1b2c3d4e5f6")
# → full file content (already loaded by then)
```

The agent can issue `aphrodite_prefetch()` and immediately call other tools — the reads proceed on daemon threads. Use `aphrodite_catalog(mode='toc')` to check which files are done loading, then retrieve when ready.

---

## Tools 🔧

| Tool                   | Description                                          |
| :--------------------- | :--------------------------------------------------- |
| `aphrodite_retrieve`   | Resolve `<<<CCR:hash\|type>>>` markers                |
| `aphrodite_compress`   | Compress content via CCR with type hint               |
| `aphrodite_stats`      | Proxy health, engine status, inline store size        |
| `aphrodite_rebuild`    | Rebuild binary, kill proxies, restart (auto)          |
| `aphrodite_files`      | Tracked file references, grouped by tool              |
| `aphrodite_diff`       | Conversation turn history with summaries              |
| `aphrodite_search`     | Search CCR store by keyword or type (trigram‑indexed) |
| `aphrodite_test`       | Smoke test suite: quick, full, matrix, pipeline       |
| `aphrodite_catalog`    | Full CCR catalog with hashes, types, sizes, previews  |
| `aphrodite_reclassify` | Retroactive metadata enrichment for unclassified CCR  |
| `aphrodite_prefetch`   | Background file read + compress — returns markers instantly; files load concurrently via daemon threads |

---

## Under the Hood 🧩

```
plugins/aphrodite/
  __init__.py      — version, exports, proxy auto‑launch
  _core.py         — constants, TOML loader, config resolvers, code structure extractor
  _inline.py       — zlib fallback (works without proxy)
  _marker.py       — 28-type classifier, template renderer, CCR markers
  _binary.py       — binary download + platform detection
  _proxy.py        — lifecycle (env, health, launch, version query)
  _tools.py        — 11 tool handlers + JSON schemas
  _hooks.py        — transform_tool_result, terminal hook, pre/post LLM, rebuild
  _engine.py       — ContextEngine (default-on, TOML toggle, 45% threshold)
  _resolve.py      — recursive marker expansion (3 levels deep)
```

**Single Rust binary.** 10 Python modules. Zero forced dependencies. CC0‑1.0 — public domain.

---

*Ready to save context?* [Install now](#quick-start) • [Read the docs](docs/) • [Report an issue](https://github.com/PlayForm/Aphrodite/issues) • [Security policy](SECURITY.md)
