<p align="center">
  <img src="assets/aphrodite.png" alt="Aphrodite" width="120">
</p>

---

# [Aphrodite] 💋 (`aphrodite`)

[Aphrodite]: https://github.com/PlayForm/Aphrodite

> **Your LLM burns 90% of its context on output it never reads. We fix that.**
>
> CCR compression proxy + absorptive preview pipeline for Hermes Agent.
> Sub‑ms compress, 12,800× max ratio, 28‑type classifier, TOML‑driven.
> _One binary. Zero dependencies. Millions of tokens saved._

[![release](https://img.shields.io/badge/release-v0.8.37-blue)](https://github.com/PlayForm/Aphrodite/releases)
[![plugin](https://img.shields.io/badge/plugin-v1.62.52-purple)](plugins/aphrodite/plugin.yaml)
[![rust](https://img.shields.io/badge/rust-1.80+-orange)](https://rust-lang.org)
[![license](https://img.shields.io/badge/license-CC0--1.0-lightgrey)](LICENSE)

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

Aphrodite intercepts **everything** before it hits the LLM context - tool
output, terminal output, file reads, search results, build logs, browser
snapshots, and more. Not just tool calls.

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
    • Session hits 45K tokens → middle turns auto-compressed to CCR
    • Agent never hits context window ceiling

    Prefetch (background):
    • aphrodite_prefetch(["main.rs", "lib.rs"]) → markers instant
    • Files load concurrently on daemon threads
    • Agent continues working while files load
```

Four layers, all under 1ms:

1. **Classify** - 28-type regex classifier identifies content (<0.1ms)
2. **Template** - TOML-driven templates produce `[type:key=val]` previews
3. **Store** - SHA-256 → SQLite/in-memory → `<<<CCR:hash|type|size>>>` marker
4. **Decide** - Agent reads preview, retrieves only when needed

---

## Architecture 🏗️

| Mode  | Port  | Backend   | Threshold | Best for                  |
| :---- | :---: | :-------- | :-------: | :------------------------ |
| Cache | :9797 | In‑memory |   >8 KB   | Speed, transient sessions |
| Token | :9798 | SQLite    |   >1 KB   | Durability, tool relay    |

Both modes share the same classifier, template engine, and preview pipeline. The
difference is persistence - cache mode is ephemeral, token mode survives
restarts.

---

## Hermes Integration 🤝

Aphrodite has **two modes** - a generic OpenAI-compatible proxy, and a native
Hermes plugin. The native integration gives Hermes deeper compression that other
agents can't get.

### Native Hermes (Full Pipeline)

When installed as a Hermes plugin, Aphrodite intercepts tool output at the
**hook level** - before it even reaches the LLM's context:

| Layer                       | What it does                                                            | Hermes-only? |
| --------------------------- | ----------------------------------------------------------------------- | :----------: |
| `on_session_start`          | Auto-launches both proxy processes (:9797, :9798)                       |      ✅      |
| `transform_tool_result`     | Intercepts every tool call return - compresses before the model sees it |      ✅      |
| `transform_terminal_output` | Compresses shell command output inline                                  |      ✅      |
| `pre_llm_call`              | Injects CCR catalog + retrieval guidance into system prompt             |      ✅      |
| `post_llm_call`             | Tracks compression metrics, updates proxy stats                         |      ✅      |
| `context_engine`            | Offloads middle conversation turns to CCR when context fills up         |      ✅      |
| `aphrodite_*` tools         | 12 tools injected directly into Hermes's tool namespace                 |      ✅      |
| `skills/`                   | 9 bundled skills auto-loaded for agents                                 |      ✅      |

**No Hermes core code is modified.** The plugin registers hooks in `plugin.yaml`
and Hermes wires them automatically. Install, enable, restart - that's it.

```yaml
# plugins/aphrodite/plugin.yaml
provides_hooks:
    - on_session_start # proxy launch
    - transform_tool_result # compress tool output
    - pre_llm_call # catalog injection
    - transform_terminal_output # terminal compression
    - post_llm_call # metrics tracking
provides_tools: # 12 aphrodite_* tools
provides_context_engine: true # long-session compression
```

### Generic Proxy (Any Client)

Any OpenAI-compatible client can route through the proxy at `:9798`:

```
Agent → Aphrodite (:9798) → LLM API
```

This gives you **CCR storage + retrieval** and the **classifier pipeline**, but
misses the Hermes-only hooks: no auto-launch, no tool result interception, no
context engine, no terminal compression, no skills.

### Comparison

| Feature                     |      Native Hermes      |     Generic Proxy      |
| --------------------------- | :---------------------: | :--------------------: |
| CCR compression             |           ✅            |           ✅           |
| Tool output interception    |       ✅ (hooks)        |           ❌           |
| Terminal output compression |        ✅ (hook)        |           ❌           |
| Context engine              |           ✅            |           ❌           |
| Auto-launch proxies         |           ✅            |           ❌           |
| Agent tools (aphrodite\_\*) |      ✅ (12 tools)      |           ❌           |
| Bundled skills              |      ✅ (9 skills)      |           ❌           |
| Prompt injection            |           ✅            |           ❌           |
| Works with any client       |    ❌ (Hermes only)     | ✅ (OpenAI-compatible) |
| Setup                       | `hermes plugins enable` |     Set `base_url`     |

```bash
# Environment variable - works for any OpenAI SDK client
export OPENAI_BASE_URL=http://127.0.0.1:9798/v1
export OPENAI_API_KEY=<your-key>   # forwarded as-is to upstream
```

```python
# Python openai SDK
from openai import OpenAI
client = OpenAI(base_url="http://127.0.0.1:9798/v1", api_key="...")
```

```typescript
// TypeScript / Node openai SDK
import OpenAI from "openai";

const client = new OpenAI({ baseURL: "http://127.0.0.1:9798/v1" });
```

### Claude Code

Claude Code uses an OpenAI-compatible transport when pointed at a custom
endpoint. Set the proxy URL via the `ANTHROPIC_BASE_URL` or the `--base-url`
flag so all tool-call traffic flows through Aphrodite before reaching
Anthropic's API (or any Claude-compatible proxy such as LiteLLM).

```bash
# Option A - environment variable
export ANTHROPIC_BASE_URL=http://127.0.0.1:9798/v1

# Option B - CLI flag
claude --base-url http://127.0.0.1:9798/v1 "build and test"
```

Because Claude Code's agentic loop can issue dozens of `Bash`, `Read`, and
`Write` tool calls per session, the browser-snapshot and build-output
compressors deliver the highest savings here - 416× on DOM snapshots, 140× on
build output.

> **Recommended model_family for Claude:** `compact` - Claude's native
> instruction-following is strong enough that minimal metadata previews
> (`[diff:1f +12/-3]`) outperform verbose ones. Set `model_family = "compact"`
> in `aphrodite.toml`.

### LLM Proxy Backends (Headroom, LiteLLM, Ollama, vLLM, …)

Aphrodite sits at the **front** of any OpenAI-compatible LLM proxy chain. Route
Aphrodite's upstream to whichever backend you run:

| Backend                | Upstream URL                     | Notes                                                         |
| :--------------------- | :------------------------------- | :------------------------------------------------------------ |
| **[Headroom]** (:9799) | `http://127.0.0.1:9799/v1`       | Default partner - semantic compression layer runs after CCR   |
| **LiteLLM**            | `http://127.0.0.1:4000/v1`       | Unified gateway for 100+ models; set `APHRODITE_UPSTREAM_URL` |
| **Ollama**             | `http://127.0.0.1:11434/v1`      | Local models; Ollama's `/v1` endpoint is OpenAI-compatible    |
| **vLLM**               | `http://127.0.0.1:8000/v1`       | High-throughput local inference                               |
| **llama.cpp server**   | `http://127.0.0.1:8080/v1`       | GGUF models; `--api-key` optional                             |
| **LM Studio**          | `http://127.0.0.1:1234/v1`       | GUI-managed local models                                      |
| **Jan**                | `http://127.0.0.1:1337/v1`       | Desktop inference server                                      |
| **Mistral API**        | `https://api.mistral.ai/v1`      | Cloud; set `APHRODITE_UPSTREAM_URL`                           |
| **Together AI**        | `https://api.together.xyz/v1`    | Cloud multi-model                                             |
| **Groq**               | `https://api.groq.com/openai/v1` | Ultra-fast inference                                          |

```toml
# aphrodite.toml - point upstream at any OpenAI-compatible backend
[proxy]
upstream_url = "http://127.0.0.1:9799/v1"   # default: Headroom
# upstream_url = "http://127.0.0.1:11434/v1" # Ollama
# upstream_url = "http://127.0.0.1:4000/v1"  # LiteLLM
```

Or override per-run with an env var:

```bash
APHRODITE_UPSTREAM_URL=http://127.0.0.1:11434/v1 aphrodite
```

### Headroom Defaults

Headroom (`:9799`) is the **default upstream** - when you run `aphrodite` with
no config, all requests flow
`agent → Aphrodite (:9798) → Headroom (:9799) → LLM API`. This gives you both
layers: CCR addressing from Aphrodite and semantic reduction from Headroom.

```toml
# aphrodite.toml - Headroom integration defaults
[headroom]
enabled = true
port    = 9799
url     = "http://127.0.0.1:9799/v1"

# Headroom strategy overrides (optional - Headroom selects automatically)
# strategy = "CODE_AWARE"   # force AST-level code compression
# strategy = "LOG"          # force log extraction
# strategy = "KOMPRESS"     # force ML semantic compression
```

Headroom auto-selects its compressor strategy based on content type. You can
override per-content-family in `aphrodite.toml` if you want to force a strategy
for a given classifier type.

### Direct Integration (OpenAI-compatible proxy)

| Agent                | Type                     | Integration                                  | Compression                         |
| -------------------- | ------------------------ | -------------------------------------------- | ----------------------------------- |
| **Hermes Agent**     | Native                   | Built-in plugin; `on_start()` auto-launches  | Full pipeline                       |
| **Claude Code**      | CLI                      | `ANTHROPIC_BASE_URL` or `--base-url`         | Very high - tool-heavy agentic loop |
| **Aider**            | Open source              | `--openai-api-base http://127.0.0.1:9798/v1` | High - context-heavy diffs          |
| **OpenHands**        | Open source (MIT)        | Custom provider config; clean agent loop     | High - multi-level compression      |
| **Codex CLI**        | Open source (Apache 2.0) | Pluggable provider; `--model` + `--base-url` | Medium-high                         |
| **Cline / Roo Code** | Open source (VS Code)    | MCP server or custom API endpoint            | Medium-high - MCP protocol          |
| **Continue.dev**     | Open source (VS Code/JB) | Custom provider config; model-agnostic       | Medium                              |
| **Sourcegraph Cody** | Enterprise               | Self-host; custom endpoint config            | Medium                              |
| **PostHog Code**     | Analytics                | Configurable endpoint                        | Low-medium - niche                  |
| **Qodo**             | Enterprise               | Proxy insertion in deployment pipeline       | Medium                              |

### MCP-Native Integration

| Agent                     | Type              | Integration                                     |
| ------------------------- | ----------------- | ----------------------------------------------- |
| **Cline / Roo Code**      | VS Code extension | Aphrodite as MCP server for context services    |
| **Cloudflare Agents SDK** | Edge platform     | AI Gateway interception; worker-based           |
| **Vercel AI SDK**         | Framework         | `wrapLanguageModel` middleware; custom provider |

### Future SDK Targets

| SDK                       | Potential | Path                                     |
| ------------------------- | --------- | ---------------------------------------- |
| **Vercel AI SDK**         | High      | Custom provider + middleware wrapping    |
| **Cloudflare Agents SDK** | Medium    | AI Gateway + Durable Objects compression |
| **MCP Protocol**          | High      | Standardized context compression service |
| **OpenAI Agents SDK**     | Medium    | Custom provider via `base_url` override  |

> **Context window management is the #1 cost bottleneck across ALL coding
> agents.** Aphrodite's compression addresses this universally - any agent that
> makes tool calls benefits from 23× median token savings on tool output.

---

## Absorptive CCR Previews 🧠

> **"Absorptive" means the classifier learns from every output it sees. New
> content of the same type automatically gets the same structured treatment - no
> manual template writing needed.**

### Without Aphrodite → With Aphrodite

| Content type     | Your agent sees without Aphrodite | Your agent sees with Aphrodite                                         |
| :--------------- | :-------------------------------- | :--------------------------------------------------------------------- |
| Git diff         | raw unified diff header           | `[diff:1f +12/-3 42L foo.rs]`                                          |
| Build output     | raw compile lines + errors        | `[build:1E 2W 142L]`                                                   |
| Traceback        | raw Python traceback              | `[error:AttributeError 'NoneType']`                                    |
| Terminal         | raw stdout + exit code            | `[terminal:cargo build exit=0]`                                        |
| Git log          | raw commit list                   | `[commit:afd634b release(v0.8.5)]`                                     |
| Rust error       | raw error[E] block                | `[error:E0308 src/main.rs:10:5 8L]`                                    |
| Table output     | raw pipe-delimited text           | `[table:12 rows 15L]`                                                  |
| JSON blob        | raw nested JSON                   | `[json:total_items,by_type 30L]`                                       |
| Web search       | raw result array                  | `[search:10 results 'rust tokio' 5L]`                                  |
| Browser snapshot | raw accessibility tree            | `[dom:342 elements 500 total]`                                         |
| Console log      | raw log array                     | `[log:42 entries 3E 5W 100L]`                                          |
| File write       | raw confirmation JSON             | `[file:src/main.rs 1234B]`                                             |
| Code (Rust)      | raw file content                  | `[code_rust:3fns, 2structs, 1impl] fn format(...); fn build(...) 200L` |
| Image gen        | raw prompt + URL                  | `[image:a photorealistic cat...]`                                      |
| Todo list        | raw task array                    | `[todo:5 items 3 pending]`                                             |
| Memory ops       | raw memory store                  | `[memory:3 entries]`                                                   |
| Cron jobs        | raw job config                    | `[cron:active]`                                                        |
| Plain text       | raw text                          | `[text:first 110 chars...]`                                            |

### The agent's thought process

```
Without Aphrodite:
  "I see 500 tokens of build output... scrolling... error? no... warning? no...
   ok it passed. That was 500 tokens I'll never get back."

With Aphrodite:
  "[build:0E 0W 1L] - clean build. Next task."
  15 tokens. Agent keeps reasoning.
```

### Classifier - 28 content types

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

- **LLM‑native** - no emojis, no decorative separators. Pure structured text the
  model can parse instantly.
- **Scannable** - the type prefix lets the LLM pattern‑match across dozens of
  markers in a single pass.
- **Compact** - `[diff:1f +12/-3 42L foo.rs]` vs 120‑char raw dump. 8× smaller
  on average.
- **Actionable** - enough detail to decide "retrieve" vs "skip" without
  expanding.
- **Model‑aware** - 3 template families adapt preview style per model (compact
  for Claude, code_first for DeepSeek, balance for GPT).

---

## LLM‑Native Output Formatting 📋

> **Your agent reads clean, machine‑parseable text instead of decorative
> human‑facing output with emojis and bold formatting. Every token counts.**

Aphrodite's own tools (`catalog`, `stats`, `diff`, `files`) return structured
output designed for LLM consumption - no emojis, no markdown decoration, no
wasted tokens.

### Without Aphrodite → With Aphrodite

**`aphrodite_catalog`**

| Your agent sees without Aphrodite              | Your agent sees with Aphrodite                 |
| :--------------------------------------------- | :--------------------------------------------- |
| `📦 Aphrodite Catalog - 2 items · 4.8KB saved` | `Catalog: 2 items 4.8KB saved 2 turns 0 files` |
| Raw JSON blob                                  | Markdown table with hash, type, size, preview  |

**`aphrodite_stats`**

| Your agent sees without Aphrodite  | Your agent sees with Aphrodite   |
| :--------------------------------- | :------------------------------- |
| `💋 Aphrodite Stats` / `✅ active` | `Aphrodite Stats` / `on` / `off` |

**`aphrodite_diff`** / **`aphrodite_files`** - same principle: structured over
decorated.

---

## What You Save 💰

> **Every token of output you compress is a token your agent can use for
> reasoning, planning, and code generation. Context is the most expensive
> resource in LLM economics - we make it go further.**

### Your agent's context budget

| Content type              | Without Aphrodite | With Aphrodite |  Savings |
| :------------------------ | ----------------: | -------------: | -------: |
| Git diff (42L)            |          ~350 tok |        ~15 tok |  **23×** |
| Build output (142L)       |        ~1,400 tok |        ~10 tok | **140×** |
| Traceback                 |           ~45 tok |        ~12 tok | **3.8×** |
| Terminal output           |          ~200 tok |        ~10 tok |  **20×** |
| Git log (8 commits)       |          ~100 tok |        ~20 tok |   **5×** |
| Table (50 rows)           |          ~650 tok |         ~8 tok |  **81×** |
| JSON blob (30 keys)       |          ~400 tok |        ~10 tok |  **40×** |
| Web search (10 results)   |          ~800 tok |        ~15 tok |  **53×** |
| Browser snapshot (342 el) |        ~5,000 tok |        ~12 tok | **416×** |
| Console log (42 entries)  |        ~1,200 tok |        ~15 tok |  **80×** |

**Median: 23× fewer tokens burned on tool output.** In a typical coding session
with 50+ tool calls, that's **15,000–50,000 tokens saved** - enough for an
entire extra conversation turn of reasoning.

### Features that save context

| Feature               | What it does                                                | Real-world impact                                                    |
| --------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------- |
| Classifier poll       | Suppresses CCR for clean outputs (0E/0W, exit=0)            | Silent builds don't waste a single token                             |
| Code structure‑map    | LLM sees fn/struct/class sigs, not raw code                 | Agent navigates 500‑line files without retrieving them               |
| Batch tool calls      | Multiple independent tools execute concurrently in one turn | 2–3× faster task completion without extra round-trips                |
| Background prefetch   | File read + compress runs in daemon threads                 | Large files load concurrently; agent continues reasoning immediately |
| Model‑aware templates | Preview style adapts to model family                        | Claude gets compact metadata, DeepSeek gets code excerpts            |
| TOML‑driven config    | All thresholds + templates in one file, no recompile        | Tune aggressiveness in 30 seconds                                    |
| Context engine        | Compresses middle conversation turns to CCR                 | Long sessions stay within context window automatically               |

---

## Performance ⚡

> **Sub‑millisecond compression with zero API calls. The classifier runs in
> <0.1ms using pure regex - no model inference, no network round‑trip, no token
> cost.**

### Proxy benchmark _(verified 2026-06-17)_

| Size   | Text  |  Code   |  JSON   |  Ratio  | Tokens Saved |
| :----- | :---: | :-----: | :-----: | :-----: | :----------: |
| 1 KB   | 0.4ms |  0.3ms  |  0.5ms  |   26×   |     240      |
| 10 KB  | 0.6ms |  0.7ms  | 3.5ms\* |  256×   |    2,500     |
| 50 KB  | 0.7ms |  0.6ms  |  1.0ms  | 1,280×  |    12,800    |
| 100 KB | 1.1ms |  1.0ms  |  1.1ms  | 2,560×  |    25,600    |
| 500 KB | 2.1ms | 7.9ms\* |  2.8ms  | 12,800× |   128,000    |

\*Outliers: single slow iteration in 5-iteration average. p95 ≤ 19.9ms.

| Metric                    |     Value      |
| :------------------------ | :------------: |
| Compression latency (avg) |     1.6 ms     |
| Compression latency (min) |     0.3 ms     |
| Retrieval latency (avg)   |     0.7 ms     |
| Retrieval p50             |     0.4 ms     |
| Benchmark pass rate       |    19/19 ✅    |
| Smoke test pass rate      |    13/13 ✅    |
| Classification latency    |    <0.1 ms     |
| Preview generation        |    <0.05 ms    |
| Worker threads (default)  | 4× CPU, min 32 |

### Cumulative proxy savings _(running session)_

| Metric                    |     Value      |
| :------------------------ | :------------: |
| Total tokens saved        |  **millions**  |
| Requests compressed       |   165 / 412    |
| Body bytes saved          |    **67%**     |
| Request bytes             |    60.6 MB     |
| Response bytes            |    20.1 MB     |
| Average saved/compression | 75,528 tokens  |
| Per benchmark run         | 507,000 tokens |
| CCR entries               |      165       |
| Cache hits                |       20       |

### Real‑world token savings

> **2 MB of raw output compresses to ~960 bytes of CCR markers - a 2,000:1
> effective ratio. In a typical coding session with 50+ operations, the proxy
> saves millions of tokens cumulatively. At current pricing, that's $250+ in API
> costs saved.**

### Classifier coverage

| Metric                 |  Value  |
| :--------------------- | :-----: |
| Content types detected |   28    |
| JSON sub‑types         |   12    |
| Code languages         |    6    |
| Template families      |    3    |
| Classification speed   | <0.1 ms |

---

## Compression Strategies 🧬

> **Aphrodite owns the _addressing_ layer - where content lives and how to find
> it. Headroom owns the _reduction_ layer - how to make content smaller while
> keeping it meaningful.**

Aphrodite's CCR layer is **content-addressed storage** - every piece of content
gets a SHA-256 hash and lives in SQLite or in‑memory. That gives us
deduplication (identical content = one hash) but not semantic reduction.

[Headroom] (partner proxy at :9799) brings the semantic layer - specialized
compressors that understand _what_ the content is and reduce it intelligently.

[Headroom]: https://github.com/PlayForm/Headroom

### Headroom compressor roster

| Strategy        | Target        | Technique                                            | Ratio |
| :-------------- | :------------ | :--------------------------------------------------- | :---: |
| `CODE_AWARE`    | Source code   | tree-sitter AST - signatures kept, bodies compressed | 5–8×  |
| `SMART_CRUSHER` | JSON arrays   | Structural dedup                                     |   -   |
| `SEARCH`        | grep output   | Dedup + summarize matches                            |   -   |
| `LOG`           | Build/test    | Error/warning extraction                             |   -   |
| `KOMPRESS`      | Free text     | ML-based semantic                                    | 3–5×  |
| `DIFF`          | Git diffs     | File-level summary                                   |   -   |
| `HTML`          | Web content   | Tag-aware                                            |   -   |
| `MIXED`         | Chat output   | Split → route → reassemble                           |   -   |
| `PASSTHROUGH`   | Sub-threshold | Identity                                             |  1×   |

### The full picture

```
tool output
    │
    ▼
┌─────────────────────────────────┐
│           Aphrodite             │
│                                 │
│  classify → template → store    │
│                                 │
│  preview: [code:3fns|2structs]  │
│  marker: <<<CCR:sha256|type>>>  │
└───────────────┬─────────────────┘
                │
                │  preview → agent (13 tok, not 215)
                ▼
             Agent
                │
                │  retrieve if needed
                ▼
┌─────────────────────────────────┐
│  Headroom (optional, :9799)     │
│                                 │
│  AST reduction / log extraction │
│  ML semantic compression        │
│  tree-sitter code-aware         │
└─────────────────────────────────┘
                │
                │  reduced content
                ▼
             Agent
```

Aphrodite owns **addressing + previews** (where content lives, what it means).
Headroom owns **reduction** (making content smaller while keeping it
meaningful). Each does what it does best. The agent pays only for what it
actually needs.

---

## Quick Start 🚀

> **30 seconds from clone to compression.**

> ⚠️ **Important:** This is the monorepo. To install the **Hermes plugin**,
> clone
> [`PlayForm/Aphrodite-Hermes`](https://github.com/PlayForm/Aphrodite-Hermes)
> instead - not this repo. The plugin lives in `./plugins/aphrodite/` as a git
> submodule. This monorepo is for developing the proxy + plugin together.

```bash
# 1. Build (one command)
cargo build --release -p aphrodite

# 2. Run (both proxies start automatically)
aphrodite

# 3. Verify
curl http://127.0.0.1:9798/health
# → {"status":"ok","version":"v0.8.37"}

# Dev loop with auto-reload
RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

### Configuration - everything in one file

```toml
# aphrodite.toml - all features, no recompile needed
[proxy]
upstream_url = "http://127.0.0.1:9799/v1"  # default: Headroom; swap for Ollama/LiteLLM/vLLM

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

[headroom]
enabled = true
port    = 9799
url     = "http://127.0.0.1:9799/v1"
```

7 TOML sections, 54 template strings, all overridable via `APHRODITE_*` env
vars. No recompile. No restart. Just edit and go.

---

### Prefetch Workflow ⚡

> **Read many files in parallel. The agent gets CCR markers instantly and
> continues reasoning while files load in background threads.**

```bash
# Batch-read files concurrently - markers return immediately
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

The agent can issue `aphrodite_prefetch()` and immediately call other tools -
the reads proceed on daemon threads. Use `aphrodite_catalog(mode='toc')` to
check which files are done loading, then retrieve when ready.

---

## Tools 🔧

| Tool                   | Description                                                                                             |
| :--------------------- | :------------------------------------------------------------------------------------------------------ |
| `aphrodite_retrieve`   | Resolve `<<<CCR:hash\|type>>>` markers                                                                  |
| `aphrodite_compress`   | Compress content via CCR with type hint                                                                 |
| `aphrodite_stats`      | Proxy health, engine status, inline store size                                                          |
| `aphrodite_rebuild`    | Rebuild binary, kill proxies, restart (auto)                                                            |
| `aphrodite_files`      | Tracked file references, grouped by tool                                                                |
| `aphrodite_diff`       | Conversation turn history with summaries                                                                |
| `aphrodite_search`     | Search CCR store by keyword or type (trigram‑indexed)                                                   |
| `aphrodite_test`       | Smoke test suite: quick, full, matrix, pipeline                                                         |
| `aphrodite_catalog`    | Full CCR catalog with hashes, types, sizes, previews                                                    |
| `aphrodite_reclassify` | Retroactive metadata enrichment for unclassified CCR                                                    |
| `aphrodite_prefetch`   | Background file read + compress - returns markers instantly; files load concurrently via daemon threads |

---

## Under the Hood 🧩

> **`./plugins/aphrodite/` is a separate repo** - it lives at
> [PlayForm/Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes), not
> inside this monorepo. If you want to install the plugin, clone THAT repo, not
> this one. This monorepo tracks it as a git submodule.

```
plugins/aphrodite/
  __init__.py          - entry point, version, proxy auto‑launch
  plugin.yaml          - Hermes plugin manifest (12 tools, 5 hooks)
  _core/               - constants, TOML loader, config resolvers, code structure extractor
  _engine.py           - ContextEngine (default-on, TOML toggle, 45% threshold)
  _hooks/              - Hermes hook handlers: transform_tool_result, terminal, pre/post LLM, rebuild
  _marker/             - 28‑type classifier, template renderer, CCR marker parse
  _proxy/              - proxy lifecycle: env, health, launch, version query
  _resolve.py          - recursive CCR marker expansion (3 levels deep)
  _binary.py           - binary auto‑download + platform detection
  _tools.py            - 12 tool handlers + JSON schemas
  _inline.py           - zlib fallback compression (works without proxy)
  _automation.py       - Rhai scripting engine
  pyproject.toml       - Python ≥3.11, no runtime deps
  skills/              - 9 bundled skills for agents (compression, proxy, tools, …)
```

**44 Python files across 5 packages.** Zero forced dependencies. CC0‑1.0 -
public domain.

---

## Relationship to Headroom

Aphrodite embeds [Headroom](https://github.com/PlayForm/headroom) - our **custom
fork** of the Headroom compression library. The fork is tracked as a git
submodule at `vendor/headroom/` and modified for Rust‑Python CCR parity, Hermes
tool relay, and PlayForm branding.

→ **[Full comparison: Aphrodite vs Headroom](docs/APHRODITE-HEADROOM.md)** -
what we add, what we rewrote, how they ship together.

→ **[Complete fork divergence](docs/HEADROOM-FORK-DIFF.md)** - every commit,
every deletion, every change between upstream Headroom and our fork.

---

## vs Headroom - Why Aphrodite Wins

> **Headroom compresses content. Aphrodite makes content optional.** They solve
> different problems - and Aphrodite's approach produces far greater savings in
> agent workflows.

| Metric             | Headroom (stock)                                                       | Aphrodite                                                                                 |
| ------------------ | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| What it does       | Semantic compression - makes content smaller while keeping it readable | Preview-first - replaces content with structured metadata, agent retrieves only if needed |
| Agent sees         | Smaller but still-readable content                                     | `[build:2E 0W 14L]` - 13 tokens of metadata                                               |
| Retrieval needed?  | No - content is already there, just smaller                            | Rarely - preview is usually enough                                                        |
| How it compresses  | ML model (Kompress), tree-sitter AST reduction, log extraction         | Pure regex classifier (<0.1ms) + TOML templates                                           |
| Hermes integration | None - proxy or library only                                           | Native plugin: hooks, context engine, 12 tools, 9 skills                                  |
| Dependencies       | Python + ML model (~100ms)                                             | Zero (Rust binary only)                                                                   |
| Token savings      | 30–80% (semantic reduction)                                            | 84%+ (preview skips content entirely)                                                     |
| Best for           | Long-form content that must be read                                    | Agent workflows where most output is scanned, not read                                    |

**Headroom shrinks content. Aphrodite skips it.** Headroom's ML compression is
powerful for content the agent must read - it keeps meaning while cutting size.
Aphrodite's preview-first approach is better for agent workflows where 95% of
output is noise the agent doesn't need. The structured preview (`[build:2E 0W]`,
`[diff:1f +3/-2]`) gives the agent enough metadata to act without ever seeing
the raw output.

Together, they're complementary - Aphrodite addresses and previews, Headroom
reduces what must be read. But for raw token savings in agent sessions,
Aphrodite's "skip it entirely" beats Headroom's "make it smaller" by 2–10×.

---

_Ready to save context?_ [Install now](#quick-start) • [Read the docs](docs/) •
[Report an issue](https://github.com/PlayForm/Aphrodite/issues) •
[Security policy](SECURITY.md)

---

## Contributing 🤝

We love contributions of every kind - code, docs, bug reports, ideas, or just
saying hi.

| Want to…          | Start here                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------ |
| Report a bug      | [Open an issue](https://github.com/PlayForm/Aphrodite/issues/new?template=bug_report.md)   |
| Suggest a feature | [Start a discussion](https://github.com/PlayForm/Aphrodite/discussions/new?category=ideas) |
| Submit a PR       | [Fork & open a PR](https://github.com/PlayForm/Aphrodite/pulls) - we review fast           |
| Ask a question    | [Discussions Q&A](https://github.com/PlayForm/Aphrodite/discussions/new?category=q-a)      |
| Improve the docs  | [Edit any page](docs/) and send a PR                                                       |

No contribution is too small. Typo fix? Welcome. Idea? Welcome. First-time
contributor? **Especially** welcome.

---

⭐ **Like Aphrodite?** [Star the repo](https://github.com/PlayForm/Aphrodite) -
it helps others find it and makes our day.

_Built with ❤️ by [PlayForm](https://github.com/PlayForm) - feedback always
welcome at [issues](https://github.com/PlayForm/Aphrodite/issues) or
[discussions](https://github.com/PlayForm/Aphrodite/discussions)._
