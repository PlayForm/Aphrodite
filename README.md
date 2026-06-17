<p align="center">
  <img src="assets/aphrodite.svg" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> CCR compression proxy + absorptive preview pipeline for Hermes Agent.  
> Sub‑ms compress, 1,280× median ratio, 28‑type classifier, TOML‑driven.

[![release](https://img.shields.io/badge/release-v0.5.119-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.14-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

---

## Architecture 🏗️

```
Hermes → aphrodite (:9797/:9798) → any LLM API
              ↓
  InMemoryCcrStore / SqliteCcrStore
              ↓
  Tool relay via POST /tool/relay
              ↓
  Absorptive preview pipeline (classify → template → CCR marker)
```

| Mode  | Port  | CCR Backend | Threshold | Features                  |
| :---- | :---: | :---------- | :-------: | :------------------------ |
| Cache | :9797 | In‑memory   |   >8 KB   | Preview, zero persistence |
| Token | :9798 | SQLite      |   >1 KB   | Tool relay, durability    |

---

## Absorptive CCR Previews 🧠

Every compressed tool output gets a **content‑aware, LLM‑native preview** instead of a raw text truncation. A **28‑type classifier** runs on the content, then a structured preview is generated via TOML‑driven templates that adapt per model family.

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

**The agent sees ~15 tokens of structured metadata instead of 120+ characters of noise.** It can pattern‑match `[diff:` across dozens of markers and instantly decide which ones to retrieve.

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

- **LLM‑native** — no emojis, no decorative separators. Pure structured text.
- **Scannable** — the type prefix lets the LLM pattern‑match across many markers.
- **Compact** — the LLM sees `[diff:1f +12/-3 42L foo.rs]` instead of a 120‑char raw dump.
- **Actionable** — enough detail to decide "retrieve" vs "skip" without expanding.
- **Model‑aware** — 3 template families (compact/Claude, code_first/DeepSeek, balance/GPT) adapt preview style per model.

---

## LLM‑Native Output Formatting 📋

The `aphrodite_catalog`, `aphrodite_stats`, `aphrodite_diff`, and `aphrodite_files` tools return structured, token‑efficient output designed for LLM consumption.

### Without Aphrodite → With Aphrodite

**`aphrodite_catalog`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `📦 Aphrodite Catalog — 2 items · 4.8KB saved` | `Catalog: 2 items 4.8KB saved 2 turns 0 files` |
| Raw JSON blob | Markdown table with hash, type, size, preview |

**`aphrodite_stats`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `💋 Aphrodite Stats` / `✅ active` / `❌ down` | `Aphrodite Stats` / `on` / `off` |
| `**Proxy:**` / `**Engine:**` / `**Inline:**` | `proxy:` / `engine:` / `inline:` |

**`aphrodite_diff`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `📜 Turn History — 2 turns` | `Turn History: 2 turns` |

**`aphrodite_files`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `📁 Referenced Files — 0 files` | `Referenced Files: 0 files` |

---

## Context Savings 💰

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

**Median: 23× fewer tokens** burned on tool output. Your agent gets context back for reasoning, not raw data dumps.

### Compression pipeline

```
Tool output > threshold
  → _classify_content()      [free — regex, no API call]
  → _make_ccr_preview()      [free — TOML template render]
  → _classifier_says_skip()  [clean outputs skip CCR entirely]
  → _compress_via_proxy()    [SHA-256 → cache check → API call only on miss]
  → CCR marker returned
```

### Features that save context

| Feature | What it does | Savings |
|---------|-------------|---------|
| Classifier poll | Suppresses CCR for clean outputs (0E/0W, exit=0) | Zero‑token markers for noise |
| Code structure‑map | LLM sees fn/struct/class sigs, not raw code | 5–8× context for code files |
| Model‑aware templates | Preview style adapts to model family | Claude gets compact, DeepSeek gets code excerpts |
| TOML‑driven config | All thresholds + templates configurable, no recompile | Tune per‑deployment |
| Context engine | Compresses middle turns to CCR at 45% threshold | 2/5 protect, 8 min msgs |

---

## Performance ⚡

### Proxy benchmark

| Metric                       | Value          |
| :--------------------------- | :------------: |
| Compression latency (avg)    | 0.9 ms         |
| 1KB compress                 | 0.1‑0.2 ms     |
| 100KB compress               | 0.5‑0.8 ms     |
| 500KB compress               | 1.5‑2.7 ms     |
| Retrieval latency (avg)      | 1.4 ms         |
| Retrieval p50                | 0.2 ms         |
| Classification latency       | <0.1 ms        |
| Preview generation           | <0.05 ms       |
| Compression ratio range      | 26× – 12,800×  |
| Compression ratio median     | 1,280×         |
| Worker threads (default)     | 4× CPU, min 32 |
| Connection pool per host     | 100            |

### Classifier coverage

| Metric                  | Value          |
| :---------------------- | :------------: |
| Content types detected  | 28             |
| JSON sub‑types          | 12             |
| Code languages          | 6 (Rust, Python, Go, JS, TS, Shell) |
| Template families       | 3 (compact, code_first, balance) |
| Classification speed    | <0.1 ms (regex, no API) |

---

## Compression Strategies 🧬

Aphrodite's CCR layer is **content-addressed storage** — every piece of content gets a SHA-256 hash and lives in SQLite or in-memory. That gives us deduplication (identical content = one hash) but not semantic reduction.

[Headroom] (our partner proxy at :9799) brings the semantic layer — it routes content through specialized compressors that actually reduce what the LLM sees.

[Headroom]: https://github.com/chopratejas/headroom

### Headroom compressor roster

| Strategy       | Target      | Technique                          | Ratio  |
| :------------- | :---------- | :--------------------------------- | :----: |
| `CODE_AWARE`   | Source code | tree-sitter AST — imports/sigs/types kept, bodies compressed | 5–8× |
| `SMART_CRUSHER`| JSON arrays | Structural dedup                   | —      |
| `SEARCH`       | grep output | Dedup + summarize matches           | —      |
| `LOG`          | Build/test  | Error/warning extraction            | —      |
| `KOMPRESS`     | Free text   | ML-based semantic                   | 3–5×   |
| `DIFF`         | Git diffs   | File-level summary                  | —      |
| `HTML`         | Web content | Tag-aware                          | —      |
| `MIXED`        | Chat output | Split → route → reassemble          | —      |
| `PASSTHROUGH`  | Sub-threshold | Identity                           | 1×     |

### Division of labor

```
content → Aphrodite (CCR addressing)
            SHA-256 → cache check → store → marker
              ↓
          Headroom (semantic reduction)
            classify → route → AST/log/ML compress → reduced content
              ↓
          CCR marker with navigable preview
            [code:3fns|2structs crate::proxy]  ← LLM browses structure
```

Aphrodite owns the addressing layer. Headroom owns the reduction layer. Together: compressed content + content‑addressable retrieval + structure‑aware previews.

---

## Quick Start 🚀

```bash
# Build
cargo build --release -p aphrodite

# Run both proxies
aphrodite

# Single mode
aphrodite --mode cache --listen :9797 --api-key $APHRODITE_API_KEY
aphrodite --mode token --listen :9798 --api-key $APHRODITE_API_KEY --tool-relay

# Dev loop
source .env.sh
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Configuration

All features driven by `aphrodite.toml`:

```toml
[compression]
engine_threshold_pct = 45    # compress at 45% context
tool_threshold_token = 512   # token proxy threshold
classifier_poll = true       # suppress CCR for clean outputs
context_engine = true        # default-on

[previews]
model_family = "code_first"  # compact | code_first | balance
code_structure_map = true    # show fn/struct sigs

[prompts]
retrieve_guidance = "minimal" # no retrieval bait
ccr_marker_hint = false      # clean markers
```

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

---

## Profiles 👥

| Profile                      | Proxy        | Engine     |
| :--------------------------- | :----------: | :--------: |
| `dev-aphrodite`              | :9798 token  | on (45%)   |
| `aphrodite-proxy-cache`      | :9797 cache  | Disabled   |
| `aphrodite-proxy-token`      | :9798 token  | 50%        |
| `aphrodite-compress-light`   | None         | Light      |
| `aphrodite-compress-medium`  | None         | Medium     |
| `aphrodite-compress-aggressive` | None      | Aggressive |

---

## Plugin Structure 🧩

```
plugins/aphrodite/
  __init__.py      — version, exports, proxy auto‑launch
  _core.py         — constants, thresholds, TOML loader, config resolvers
  _inline.py       — zlib fallback compression (works without proxy)
  _marker.py       — 28-type classifier, template-driven previews, CCR markers
  _binary.py       — binary download + platform detection
  _proxy.py        — proxy lifecycle (env, health, launch, version query)
  _tools.py        — 10 tool handlers + JSON schemas
  _hooks.py        — transform_tool_result (absorptive previews), terminal hook,
                     pre/post LLM, rebuild handler (kill+restart)
  _engine.py       — AphroditeContextEngine (default-on, TOML toggle)
  _resolve.py      — recursive marker expansion (up to 3 levels deep)
```

---

## Configuration Reference

### `aphrodite.toml` sections

| Section | Keys | Controls |
|---------|------|---------|
| `[compression]` | 14 | Thresholds, engine, auto-expand, catalog, classifier poll, code multiplier |
| `[previews]` | 4 | model_family, code_structure_map, preview_max_chars |
| `[prompts]` | 3 | retrieve_guidance, ccr_marker_hint, catalog_intent_hints |
| `[templates.preview.{family}]` | 18 × 3 | Per-type format strings with `{variable}` substitution |
| `[templates.marker]` | 2 | CCR block format + hint string |
| `[templates.prompts]` | 5 | session_inject, engine_offload, auto_expand, catalog_warn, search_hint |
| `[templates.reverse]` | 25 | Type alias map for granular routing |

### Env vars (override TOML)

| Variable | TOML key | Default | Effect |
| :------- | :------- | :-----: | :----- |
| `APHRODITE_DEBUG` | — | `0` | Debug logging |
| `APHRODITE_PASSTHROUGH` | — | `0` | Bypass all compression |
| `APHRODITE_MODEL` | `defaults.model` | `deepseek-v4-pro` | Sets model family for template selection |
| All `APHRODITE_*` vars | Corresponding TOML keys | See `aphrodite.toml` | Env overrides TOML |

---

## Metrics 📊

31 Prometheus metrics at `/metrics`. Docker:

```bash
docker run -d --name aphrodite-prometheus -p 9090:9090 \
  -v ./prometheus.yml:/etc/prometheus/prometheus.yml \
  --add-host=host.docker.internal:host-gateway \
  prom/prometheus
```

Covers: requests, compression, CCR hits/misses, cache, tool relay, notifications, upstream errors, latency histograms, body bytes, inline CCR, store size.

---

## MCP Integration 🔌

Aphrodite works with any MCP server. Add servers to your Hermes config:

```yaml
# ~/.hermes/config.yaml
mcp:
  servers:
    your-server:
      command: /path/to/mcp-server
      enabled: true
```

Then add `mcp` to your `toolsets` and `platform_toolsets`. Tools are auto‑discovered on Hermes restart.

---

*Single Rust binary. Zero forced dependencies. CC0‑1.0. All config in `aphrodite.toml`.* | [Security policy](SECURITY.md)
