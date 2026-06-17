# Changelog

## v0.7.0 — Atomization + Live Containers (2026-06-17)

### Plugin Atomization — 29 Nested Modules

Three monolithic files split into deeply-nested single-responsibility modules:

| Package    | Modules                                                                                                                                                | Max lines |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| `_core/`   | config, store, template, struct, state, **init**                                                                                                       | 244       |
| `_hooks/`  | catalog, classify, diff, files, git, live, prefetch, rebuild, reclassify, search, session, session_helpers, stats, terminal, test, transform, **init** | 245       |
| `_marker/` | classify, compress, marker, parse, preview, **init**                                                                                                   | 246       |

Each file exports exactly one function. No file exceeds 250 lines. Originals
preserved as `.py.bak`.

### Live Containers — Streaming Terminal Output

`aphrodite_poll_container(hash)` — LLM never blocks on terminal output:

- Process runs in background thread, output streams to container
- Marker returned instantly — `<<<LIVE:hash|terminal|streaming>>>`
- Poll anytime for partial output + status (running/done/error)
- Content accumulates as process runs

### CCR_UNRESOLVED Fix — Dual-Store Guarantee

Every proxy compress/fetch now mirrors to inline zlib store:

- `_resolve_one`: proxy fetch → `_inline_store_put`
- `_transform_terminal_hook`: proxy compress → `_inline_store_put`
- `_transform_tool_result`: proxy compress → `_inline_store_put`
- Content always in both proxy SQLite AND inline store

### Persistent Markers — Session Resume

`_recent_markers` persists across restarts:

- `atexit` saves last 100 markers to `~/.hermes/aphrodite/recent-markers.json`
- `on_start()` restores on session begin
- TOC populated from previous session immediately

### Prefetch + ETA Schedule

`aphrodite_prefetch(paths)` — background file read + compress:

- Threads read files, classify, compress via proxy
- Markers returned instantly — agent continues
- `aphrodite_prefetch_status()` — live ETA schedule per file
- Status: LOADING → READY (with elapsed time) → ERROR

### TOC — Table of Contents with Retrieve? Recommendations

`aphrodite_catalog(mode='toc')` — compact decision table:

- Shows every CCR entry with hash, type, size, preview
- Retrieve? column: NO for clean outputs, YES for content worth retrieving
- Agent checks TOC before any retrieval — eliminates blind retrieval reflex

### Classifier Expansion

From 10 types → 28 types:

- New: write_file, log, browser_snapshot, web_search, image_generate, todo,
  memory, cronjob, session_search
- New language support: code_ts (TypeScript), code_sh (Shell)
- All 28 types have TOML templates per model family

### Classifier Poll — Zero-Token Clean Outputs

`_classifier_says_skip()` suppresses CCR for inert content:

- 0E/0W builds → preview inline, no marker
- exit=0 terminals → skip CCR
- 0-match searches → skip CCR
- TOML toggle: `[compression].classifier_poll = true`

### Model-Aware Templates

Three template families per model:

- `compact` (Claude): `[type:key=val]` metadata only
- `code_first` (DeepSeek): code signatures before metadata
- `balance` (GPT): metadata + first signature
- TOML: `[previews].model_family = "code_first"`

### Code Structure Maps

Code previews show navigable structure:

- Rust: fn signatures, struct counts, impl counts
- Python: def signatures, class counts
- Go: func signatures, type counts
- JS/TS: function signatures, class counts
- Shell: function detection

### TOML-Driven Configuration

All features configurable in `aphrodite.toml`:

- `[compression]` — 14 knobs (thresholds, engine, classifier poll, code
  multiplier)
- `[previews]` — 4 knobs (model_family, code_structure_map, preview_max_chars)
- `[prompts]` — 3 knobs (retrieve_guidance, ccr_marker_hint,
  catalog_intent_hints)
- `[templates.preview.{family}]` — 18 per-type format strings × 3 families
- `[templates.marker]` — CCR block format + hint string
- `[templates.prompts]` — 5 prompt templates
- `[templates.reverse]` — 25-type key map

### Retrieval Bait Removal

All explicit `(use aphrodite_retrieve)` instructions removed:

- Terminal/build CCR markers: clean pointers, no bait
- Catalog entries: no per-marker retrieve commands
- Session injection: "retrieve if preview doesn't tell you enough"
- Context engine: "use if needed" instead of "retrieve with:"
- Proxy guidance: "retrieve only if preview hints at useful content"

### Agent Compatibility Documentation

22 platforms researched and documented:

- 9 direct integration (Hermes, Aider, OpenHands, Codex, Cline, Continue, Cody,
  PostHog, Qodo)
- 3 MCP-native (Cline, Cloudflare, Vercel)
- 4 future SDK targets (Vercel AI SDK, Cloudflare Agents SDK, MCP Protocol,
  OpenAI Agents SDK)

### Context Engine Default-On

`[compression].context_engine = true` — no `APHRODITE_CONTEXT_ENGINE=1` needed.
Engine registers automatically at plugin load.

### Post-Rebuild Proxy Auto-Restart

`aphrodite_rebuild()` now: kill proxies → copy binary → restart both → query
version. One call replaces the binary without manual intervention.

### CI — Multi-Platform Builds

- 4 targets: Linux x86_64, macOS arm64, macOS x86_64, Windows x86_64
- Binary naming: full Rust triple (e.g., `aphrodite-aarch64-apple-darwin`)
- Shared cache between Check and Build workflows
- Nightly toolchain everywhere
- Tag trigger: `Aphrodite/v*` — single run per release

### Release Automation

- `scripts/auto-release.sh --minor` for feature bumps
- All 4 version locations auto-bumped: Cargo.toml, \_core/config.py,
  pyproject.toml, **init**.py
- `scripts/release-notes.sh` — shell-safe template generator
- Tag format: `Aphrodite/v*`

### Plugin Lifecycle

- `on_start()` auto-launches both proxies on session begin
- Binary auto-downloaded from GitHub releases if missing
- Plugin symlinks to repo for instant code updates
- `env_passthrough` configured for API key forwarding

### Tools: 14 (was 10)

| Tool                        | Description                    |
| --------------------------- | ------------------------------ |
| `aphrodite_retrieve`        | Resolve CCR markers            |
| `aphrodite_compress`        | Compress content via proxy     |
| `aphrodite_stats`           | Proxy health, engine status    |
| `aphrodite_rebuild`         | Rebuild, kill proxies, restart |
| `aphrodite_files`           | Tracked file references        |
| `aphrodite_diff`            | Turn history                   |
| `aphrodite_search`          | Trigram-indexed CCR search     |
| `aphrodite_test`            | Smoke test suite               |
| `aphrodite_catalog`         | Full catalog + TOC mode        |
| `aphrodite_reclassify`      | Retroactive metadata           |
| `aphrodite_prefetch`        | Background file read           |
| `aphrodite_prefetch_status` | Live ETA schedule              |
| `aphrodite_poll_container`  | Streaming terminal output      |
| `aphrodite_benchmark`       | Performance benchmark          |
