<p align="center">
  <img src="assets/aphrodite.svg" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> CCR compression proxy + absorptive preview pipeline for Hermes Agent.  
> Sub‑ms compress, 10× ratio, dual‑mode, LLM‑native output formatting.

[![release](https://img.shields.io/badge/release-v0.5.107-blue)](https://github.com/PlayForm/Aphrodite/releases)
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
  Absorptive preview pipeline (classify → format → CCR marker)
```

| Mode  | Port  | CCR Backend | Threshold | Features                  |
| :---- | :---: | :---------- | :-------: | :------------------------ |
| Cache | :9797 | In‑memory   |   >8 KB   | Preview, zero persistence |
| Token | :9798 | SQLite      |   >1 KB   | Tool relay, durability    |

---

## Absorptive CCR Previews 🧠

Every compressed tool output gets a **content‑aware, LLM‑native preview** instead of a raw text truncation. A 10‑type classifier runs on the content, then a structured preview is generated that lets the LLM decide whether to retrieve — without burning context.

### Without Aphrodite → With Aphrodite

| Content type | Your agent sees without Aphrodite (`result[:120]`) | Your agent sees with Aphrodite (structured preview) |
| :----------- | :-------------------------------------------------- | :-------------------------------------------------- |
| Git diff | `diff --git a/foo.rs b/foo.rs --- a/foo.rs +++ b/foo.rs @@ -1 +1,2 @@ +added -old` | `[diff:1f +12/-3 42L foo.rs]` |
| Build output | `Compiling foo v1.0 Compiling bar v2.0 error[E0425]: cannot find value warning: unused variable` | `[build:1E 2W 142L]` |
| Traceback | `Traceback (most recent call last): File "x.py", line 42 AttributeError: 'NoneType'` | `[error:AttributeError 'NoneType' has no attribute 'x']` |
| Terminal command | `$ cargo build Compiling foo v1.0 Finished in 2.3s exit code: 0` | `[terminal:cargo build exit=0]` |
| Git log | `afd634b save 6959c69 release(aphrodite): v0.5.104 96d52ff tune(coding)` | `[commit:afd634b release(aphrodite): v0.5.104]` |
| Rust error | `error[E0308]: mismatched types --> src/main.rs:10:5` | `[error:E0308 src/main.rs:10:5 8L]` |
| Table output | `a b 1 2 3 4 5 6` | `[table:12 rows 15L]` |
| JSON blob | `{"total_items": 2, "by_type": {"tool": {"count": 2}}, "items": [...]}` | `[json:total_items,by_type,items 30L]` |
| Plain text | `some plain text output more lines here` | `[text:some plain text output more lines here]` |

**The agent sees ~15 tokens of structured metadata instead of 120+ characters of noise.** It can pattern-match `[diff:` across dozens of markers and instantly decide which ones to retrieve.

### Classifier pipeline

```
content → _classify_content()
          ├─ diff      (match: diff --git, ---, +++, counts +/-, files)
          ├─ terminal  (match: exit code: N in last 5 lines)
          ├─ build     (match: Compiling in first 30 lines)
          ├─ error     (match: error[E, Traceback, panic, Error:)
          ├─ commit    (match: hex hash ^[a-f0-9]{7,40})
          ├─ json      (match: valid JSON parse)
          ├─ search    (match: file:line: pattern)
          ├─ tabular   (match: | in ≥3 lines)
          ├─ process   (match: session_id, pid)
          └─ text      (fallback)
          ↓
_make_ccr_preview() → [type:key=val ...]  (≤120 chars, pipe‑safe)
```

### Why `[type:...]` format

- **LLM‑native** — no emojis, no decorative separators. Pure structured text.
- **Scannable** — the type prefix lets the LLM pattern‑match across many markers.
- **Compact** — the LLM sees `[diff:1f +12/-3 42L foo.rs]` instead of a 120‑char raw dump.
- **Actionable** — enough detail to decide "retrieve" vs "skip" without expanding.

---

## LLM‑Native Output Formatting 📋

The `aphrodite_catalog`, `aphrodite_stats`, `aphrodite_diff`, and `aphrodite_files` tools return structured, token‑efficient output designed for LLM consumption — no emojis, no decorative cruft. Your agent reads clean machine‑parseable text instead of decorative human‑facing output.

### Without Aphrodite → With Aphrodite

**`aphrodite_catalog`**

| Your agent sees without Aphrodite | Your agent sees with Aphrodite |
| :----- | :---- |
| `📦 Aphrodite Catalog — 2 items · 4.8KB saved · 2 turns · 0 files` | `Catalog: 2 items 4.8KB saved 2 turns 0 files` |
| Raw JSON `{"total_items":2,"by_type":{...},"items":[...]}` | Markdown table with hash, type, size, preview |

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

### Compression pipeline

```
Tool output > threshold
  → _classify_content()      [free — regex, no API call]
  → _make_ccr_preview()      [free — string format]
  → _compress_via_proxy()    [SHA-256 → cache check → API call only on miss]
  → CCR marker returned
```

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

**Median: 23× fewer tokens** burned on tool output. Your agent gets context back for reasoning, not raw data dumps.

### `_ESSENTIAL_TOOLS` — what gets compressed

| Tool | Without Aphrodite | With Aphrodite |
| :--- | :---------------- | :------------- |
| `read_file`, `skill_view`, `session_search`, `memory` | Raw output fills context | Compressed via CCR; agent retrieves on demand |
| `aphrodite_*` tools (catalog, stats, etc.) | Verbose emoji-heavy text | LLM‑native compact format |

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

### Aphrodite vs Headroom — division of labor

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

Aphrodite owns the addressing layer (hash, store, retrieve). Headroom owns the reduction layer (AST, ML, log compression). Together: compressed content + content-addressable retrieval + structure-aware previews.

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

---

## Performance ⚡

| Metric                       | Value          |
| :--------------------------- | :------------: |
| Compression latency (avg)    | 0.9 ms         |
| Retrieval latency (avg)      | 3.4 ms         |
| Classification latency       | <0.1 ms        |
| Preview generation           | <0.05 ms       |
| Compression ratio EMA        | 10.0×          |
| Tokens saved                 | 12M+           |
| Context saved per turn (median) | ~23× vs full expansion |
| Worker threads (default)     | 4× CPU, min 32 |
| Connection pool per host     | 100            |

---

## Tools 🔧

| Tool                   | Description                                          |
| :--------------------- | :--------------------------------------------------- |
| `aphrodite_retrieve`   | Resolve `<<<CCR:hash\|type>>>` markers                |
| `aphrodite_compress`   | Compress content via CCR with type hint               |
| `aphrodite_stats`      | Proxy health, engine status, inline store size        |
| `aphrodite_rebuild`    | Rebuild aphrodite binary from source                  |
| `aphrodite_files`      | Tracked file references, grouped by tool              |
| `aphrodite_diff`       | Conversation turn history with summaries              |
| `aphrodite_search`     | Search CCR store by keyword or type (trigram‑indexed) |
| `aphrodite_test`       | Smoke test suite: quick, full, matrix, pipeline       |
| `aphrodite_catalog`    | Full CCR catalog with hashes, types, sizes, previews  |
| `aphrodite_reclassify` | Retroactive metadata enrichment for unclassified CCR  |

---

## Plugin Structure 🧩

```
plugins/aphrodite/
  __init__.py      — version, exports, proxy auto‑launch
  _core.py         — constants, thresholds, CCR regex, CATALOG_MODE
  _inline.py       — zlib fallback compression (works without proxy)
  _marker.py       — CCR marker formatting, parsing, _classify_content (10 types),
                     _make_ccr_preview (absorptive pipeline)
  _binary.py       — binary download + platform detection
  _proxy.py        — proxy lifecycle (env, health, launch, headroom context)
  _tools.py        — 10 tool handlers + JSON schemas
  _hooks.py        — transform_tool_result (absorptive previews, LLM‑native formatting),
                     transform_terminal, pre/post LLM, _ESSENTIAL_TOOLS guard
  _engine.py       — AphroditeContextEngine (opt‑in via APHRODITE_CONTEXT_ENGINE=1)
  _resolve.py      — recursive marker expansion (up to 3 levels deep)
```

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

## Profiles 👥

| Profile                     | Proxy        | Compression |
| :-------------------------- | :----------: | :---------: |
| `aphrodite-proxy-cache`     | :9797 cache  | Disabled    |
| `aphrodite-proxy-token`     | :9798 token  | 50%         |
| `aphrodite-compress-light`  | None         | Light       |
| `aphrodite-compress-medium` | None         | Medium      |
| `aphrodite-compress-aggressive` | None     | Aggressive  |

---

## MCP Integration 🔌

```yaml
# ~/.hermes/config.yaml
mcp_servers:
  wezterm:
    command: REPLACED/.local/bin/wezterm-mcp
    enabled: true
```

Requires `pip install mcp`. Tools auto‑discovered on Hermes restart:

| MCP Tool | Description |
| :------- | :---------- |
| `mcp_wezterm_list_panes` | List all WezTerm panes with IDs, titles, CWDs |
| `mcp_wezterm_send_text` | Send text to any pane (escape `\n` for Enter) |
| `mcp_wezterm_get_buffer` | Read N lines of scrollback from a pane |
| `mcp_wezterm_capture_snapshot` | Dump all pane buffers |

---

## Env Vars

| Variable | Values | Default | Effect |
| :------- | :----- | :-----: | :----- |
| `APHRODITE_DEBUG` | `0`, `1` | `0` | Debug logging for all hooks |
| `APHRODITE_CONTEXT_ENGINE` | `0`, `1` | `0` | Register AphroditeContextEngine |
| `APHRODITE_CATALOG` | `compact`, `full`, `tool` | `compact` | Per‑turn catalog injection mode |
| `APHRODITE_PASSTHROUGH` | `0`, `1` | `0` | Bypass all compression (dev mode) |
| `APHRODITE_CODE_MULTIPLIER` | float | `2` | Code content threshold multiplier |

---

*Single Rust binary. Zero forced dependencies. CC0‑1.0.* | [Security policy](SECURITY.md)
