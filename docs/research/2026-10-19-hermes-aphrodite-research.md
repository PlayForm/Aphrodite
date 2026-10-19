# Hermes ↔ Aphrodite Research (2026-10-19)

Explorer report on how Aphrodite works, from two surfaces: the live Hermes
runtime home (`~/.hermes/`) and the Aphrodite source repository. All paths
verified on disk; repo paths shown relative to the repo root.

## Executive summary

Aphrodite is a **CCR (Compress-Cache-Retrieve) compression proxy + absorptive
preview pipeline for Hermes Agent** - "up to 610× compression on the standard
corpus (132× overall), ~10 ms end-to-end" (`README.md:11-13`). It intercepts
tool output before it reaches the LLM and replaces it with a compact type-aware
preview plus a `<<<CCR:hash|type|size>>>` marker; the agent retrieves full
content only when needed (`README.md:97-100,135-138`).

Two crates: `crates/aphrodite` is the agent-agnostic core engine (binary +
cdylib); `crates/aphrodite-hermes` is the Hermes-specific bridge (FFI dylib +
tool dispatch + schemas + directives). A thin Python loader
(`plugins/aphrodite/__init__.py`, ctypes) registers 6 hooks and 13 tools.
Compression runs in two modes: **plugin** (Hermes hooks intercept output
directly) and **proxy** (reverse proxy on 127.0.0.1:9797/9798 between any
client and an OpenAI-compatible API) (`README.md:247-254`).

On this machine, `~/.hermes/aphrodite/` holds `binaries/{aphrodite,
libaphrodite_hermes.dylib}`, `ccr.db` (SQLite table `ccr_entries`: hash, orig
BLOB, timestamps, TTL), 7 directive `.md` files, `session.current`
(`20261019_063036_eafb3d`, 22 bytes) and `proxy-stderr.log` (119 bytes:
"failed to bind 127.0.0.1:9797 - address already in use", i.e. a prior proxy
instance was already listening). `~/.hermes/config.yaml` lists `aphrodite` as
an enabled plugin toolset (line 582) with `allow_tool_override: true` (592-593)
and per-platform toolset memberships (959-999). Hermes's own `compression:`
block (lines 135-145) is a separate, *disabled* mechanism (`enabled: false`) -
unrelated to Aphrodite.

## 1. What is Aphrodite?

- Self-definition: "CCR compression proxy + absorptive preview pipeline for
  Hermes Agent. ... One binary. Zero dependencies. Millions of tokens saved."
  (`README.md:11-14`).
- Problem framing: "Every file read, build, code search, or browser open floods
  the agent's context with raw output ... Aphrodite intercepts output before it
  reaches the LLM and replaces it with a compact, structured preview. The agent
  sees ~15 tokens of metadata ... and retrieves the full content only when it
  actually needs it." (`README.md:93-100`).
- Core crate purpose: "aphrodite: core compression engine exposed as both an
  rlib (for the `aphrodite` binary and the `aphrodite-hermes` bridge) and a C
  ABI cdylib" with an agent-agnostic handle-based `aphrodite_*` C ABI
  (`crates/aphrodite/src/lib.rs:1-8`).
- Bridge crate purpose: "aphrodite-hermes: Hermes Agent-specific integration
  crate ... leaving the core `aphrodite` crate as a pure, agent-agnostic
  compression engine" (`crates/aphrodite-hermes/src/lib.rs:1-18`, incl. an
  ASCII architecture diagram of Python plugin → ctypes → dylib → rlib).

## 2. Architecture

Two modes (`README.md:247-254`):

| Mode | Interception | Coverage |
| --- | --- | --- |
| Plugin | Hermes hooks `transform_tool_result`, `transform_terminal_output`, context engine | every tool result, terminal output |
| Proxy | Reverse proxy on :9797 (cache) / :9798 (token) in front of any LLM API; compresses Chat Completions responses; tool relay for bidirectional calls | OpenAI-compatible clients |

Crate/module map (`README.md:258-295`, verified against `src/` listings):

- `crates/aphrodite/src/` (core): `main.rs` (CLI entry, config bootstrap),
  `proxy.rs` (HTTP proxy: classify → compress → store → preview; SSE; tool
  relay - `proxy_handler` at :1154, `handle_tool_relay` at :2241,
  `handle_ccr_create` at :2441), `hooks.rs` (the hook pipeline, :199-690),
  `preview/` (type detection + preview builders), `resolve.rs` (recursive CCR
  marker resolution), `retrieve.rs` (/retrieve), `templates.rs` (TOML
  templates), `marker.rs` (marker parsing), `stage2.rs` (semantic reduction:
  JSON minify, build, diff, code), `struct_extract.rs` (code structure
  extraction), `state/` (AppState, inline store, markers, events), `flow.rs`
  (per-turn injected-context assembler), `directives.rs` +
  `builtin_directives/` (shipped directive .md), `session.rs`, `catalog.rs`,
  `prefetch.rs`, `poll_worker.rs`, `home.rs` (runtime home layout), `setup/`
  (`aphrodite setup` installer), `config/` (TOML schema, hot reload).
  Proxy modules are feature-gated (`lib.rs:40-52`) so the Hermes dylib doesn't
  link axum/tokio; Rhai scripting is separately feature-gated (:55-59).
- `crates/aphrodite-hermes/src/` (bridge): `lib.rs` (FFI surface,
  `aphrodite_hermes_dispatch_tool` :603, `aphrodite_hermes_call_hook` :683,
  `aphrodite_hermes_get_schemas` :980, `aphrodite_hermes_proxy_health` :1010,
  `aphrodite_hermes_materialize_directives` :1032), `tools.rs`
  (`dispatch(name, args_json)` :23), `schemas.rs`, `directives.rs`,
  `debug.rs`, `bin/setup-helper.rs`.
- `plugins/aphrodite/` (repo): thin Python ctypes loader + `plugin.yaml`
  manifest (6 hooks, 13 tools) + `download.sh/.ps1` that fetch prebuilt
  binaries into `~/.hermes/aphrodite/binaries/` with SHA-256 verification
  (`README.md:43-49,289-294`).

Runtime home layout is defined by the core crate itself: "`binaries/`,
`directives/`, `ccr.db`, and the proxy logs" (`home.rs:5,60`),
`ccr_db_path() = runtime_home().join("ccr.db")` (`home.rs:113`).

## 3. Tool-output compression in depth (THE feature)

### Pipeline (four layers, `README.md:133-138`)

1. **Classify** - type-aware classifier; proxy uses the vendored Headroom
   `content_detector`; when the result is generic, an Aphrodite-side semantic
   detector (`resolve_effective_type`) upgrades to high-signal shapes
   (`git`, `gitlog`, `grep`, `ls`, `test`) via
   `resolve_type_and_classify_content` (`hooks.rs:292-315`); README calls this
   `detect_semantic_type` (`README.md:204-211`).
2. **Preview** - enriched per-type preview line, e.g.
   `[build:1E 1W 142L | error[E0432]: ...]` (`README.md:109-126`).
3. **Store** - `hash = headroom_core::ccr::compute_key(...)` (BLAKE3, 40 hex)
   (`hooks.rs:383`; `docs/ccr/marker-format.md` "Hashes are lowercase
   40-character hex digests ... BLAKE3, truncated to 160 bits"), then inline LRU
   put (`state.inline_store_put` at `hooks.rs:385`; budget default 256 MB per
   `docs/architecture/05-ccr-lifecycle.md` eviction table) or, in token proxy
   mode, `SqliteCcrStore.put` → `ccr.db` ("Token mode → SqliteCcrStore.put
   (TTL rows)"; "Cache mode → InMemoryCcrStore.put (cap 10k + TTL)" -
   lifecycle doc; SQLite backend at
   `vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs:89,145,340-351`).
4. **Emit/Decide** - marker `<<<CCR:hash|type|size>>>` plus preview is
   returned to the model; agent retrieves with `aphrodite_retrieve(hash)`
   only when needed (`hooks.rs:387-425`; `README.md:128-131`).

### What happens to a long cargo build

`transform_tool_result_inner` (`hooks.rs:317-426`): telemetry event recorded
(:329-331) → skip empty (:332-334) → track file references (:345-349) → skip
essential tools (`ESSENTIAL_TOOLS`, :352-354) → skip Aphrodite's own tools to
avoid an "infinite preview loop" (:356-361) → below-threshold skip (:363-368) →
live-container fast path if enabled (:370-380) → classify + hash + inline
store + preview + marker (:382-398) → JSON payload
`{status, compressed:true, hash, type, size, preview, marker}` (:411-419).
Terminal output goes through `transform_terminal_output*` (:429-446) with an
`exit code:` override to type `terminal`. Thresholds are per-proxy mode:
`tool_threshold_token = 256` bytes (token mode), `tool_threshold_cache = 2048`
(cache mode), `terminal_threshold = 256` (dylib), `inline_threshold = 1024`
(`aphrodite.toml.example:93-96`). Each content type has a threshold tier
(error ×8, code ×1.0 scaled by `code_multiplier`, diff/git/text ×2, default
×1; `README.md:213-224`).

### Marker format (`docs/ccr/marker-format.md`)

```
<<<CCR:hash|type|size>>>
```
"hash - BLAKE3 digest ... first 40 hex characters"; retrieval points tolerate the
`|type|size` suffix echoed back; "There is no prefix or fuzzy matching: the
full hash must be supplied." Inline-only entries use an `i:` prefix. The proxy
pipeline renders the preview first, the hook/FFI pipeline leads with the
marker line.

### ccr.db (verified live)

`sqlite3 ~/.hermes/aphrodite/ccr.db '.schema'`:

```sql
CREATE TABLE ccr_entries (
  hash          TEXT PRIMARY KEY,
  original      BLOB NOT NULL,
  created_at    INTEGER NOT NULL,
  ttl_seconds   INTEGER NOT NULL,
  last_accessed INTEGER NOT NULL
);
```

Matches the backend's `CREATE TABLE IF NOT EXISTS ccr_entries` in
`vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs:145` and the
TTL/`last_accessed` migration at :191-198. The token-mode proxy writes here
(`proxy.rs:2426,2631` "Token => 'sqlite'"); db path resolution:
`cli.ccr_db_path` → runtime home `ccr.db` (`proxy.rs:797-816`, `home.rs:113`).
Eviction: "Expired: TTL elapsed (ccr_ttl_seconds, default 3600s)" -
`docs/architecture/05-ccr-lifecycle.md`.

### Context engine (middle-turn archiving)

The `pre_llm_call` hook "auto-compresses overflowing middle turns to CCR as
the session fills, so the agent never hits the context ceiling"
(`README.md:140-142`); knobs `engine_threshold_pct = 30`,
`engine_protect_first = 2`, `engine_protect_last = 5`, `engine_min_msgs = 8`
(`aphrodite.toml.example:89-92`, marked LOAD-BEARING). `pre_llm_call` builds
the per-turn injected context via `flow::build_turn_context` (directives +
delta catalog) (`hooks.rs:665-684`); `post_llm_call` archives the largest
marker of the turn (`hooks.rs:690+`).

### Auto-expand

`[compression] auto_expand = true`, `auto_expand_limit = 102400`:
"resolve just-stored content inline instead of emitting a marker"
(`aphrodite.toml.example:97-98`). Implemented at `hooks.rs:363-368,400-424` -
under auto-expand no `<<<CCR:...>>>` ever reaches the model within the limit.

### Tool relay

`docs/tool-relay/tools.md`: "Aphrodite exposes 13 tools to the Hermes agent ...
Every tool is a thin relay into the Rust dylib" -
`aphrodite_compress`, `aphrodite_retrieve`, `aphrodite_stats`,
`aphrodite_files`, `aphrodite_diff`, `aphrodite_search`,
`aphrodite_directive`, `aphrodite_test`, `aphrodite_catalog`, ... (table at
tools.md). Proxy side: `handle_tool_relay` (`proxy.rs:2241`).

### Directive doc (live instance)

`~/.hermes/aphrodite/directives/ccr-handling.md` instructs the agent: "Some
tool output arrives as a short marker instead of inline text:
`<<<CCR:hash|type|size>>>`. ... That marker IS the content" - retrieve via
`aphrodite_retrieve(hash)`, search via `aphrodite_search`, catalog via
`aphrodite_catalog`, batch retrievals, prefer granular retrieval.

## 4. Other features catalog

- **Proxy mode** - dual proxies cache(:9797)/token(:9798), SSE pass-through,
  health endpoint (`README.md:311-319`; `proxy.rs:1154,2773`; docs/proxy/).
- **30-type classifier** with per-type preview templates and threshold tiers
  (`README.md:179-224`; docs/classification/content-types.md).
- **Stage-2 semantic reduction** - JSON minify, build-log, diff, code shaping
  (`README.md:270`).
- **Code structure extraction** for Rust/Python/Go/JS/TS previews
  (`README.md:271`).
- **TOML templates** for previews/markers/prompts + reverse key map
  (`README.md:148-152`; `templates.rs`; `aphrodite.toml.example:158-198`).
- **Auto-expand** (see §3; `aphrodite.toml.example:97-98`).
- **Context engine / middle-turn archiving + `engine_offload` prompt**
  (`README.md:156-159`).
- **Live containers** - instant `live` CCR markers with progressive chunked
  retrieval + ETA (`README.md:160-162`; `hooks.rs:370-380`).
- **Directives** - swappable behavioral .md files; built-in set: focus,
  foresight, ccr-handling, cleanup, explore, lazy, lazy-eval
  (`aphrodite.toml.example:204-210`; `builtin_directives/`); swapped at
  runtime via `aphrodite_directive("swap", "explore")`.
- **Flow budget** - hard cap 2600 chars for all per-turn injected context
  (`aphrodite.toml.example:216-221`; `flow.rs`).
- **Poll worker** - auto-backgrounding of slow tool calls
  (`README.md:278`; `poll_worker.rs`).
- **Prefetch** - background file prefetch into CCR (`README.md:277`).
- **Chain split** - opt-in fine-grained command splitting
  (`aphrodite.toml.example:112-114`; `chain_split.rs`).
- **Centers** - "traveling memory annotation embedded in CCR markers"
  (`docs/centers.md` - only v1 string annotation `;center=...` implemented,
  v2-v4 are roadmap sketches).
- **Rhai scripting** - feature-gated micro-scripts transforming hook payloads
  (`README.md:163-164`; `lib.rs:55-59`).
- **Metrics/health/stats** - `aphrodite_stats`, `/health` with counters,
  `aphrodite_stats_json` (`proxy.rs:505,2773`; docs/metrics/).
- **Setup/installer** - `aphrodite setup` writes runtime home + hooks-only
  loader (`README.md:69-76`; `setup/`).
- **13 relay tools + schemas** (see §3; `tools.rs`, `schemas.rs`).
- **Hermes skills/directives materialization** into the runtime home
  (`aphrodite_hermes_materialize_directives`, `lib.rs:1032`).

## 5. How Hermes hosts it (live, verified)

- `~/.hermes/aphrodite/`: `binaries/aphrodite`, `binaries/libaphrodite_hermes.dylib`,
  `ccr.db` (+ `-shm`/`-wal`), `directives/` (7 files: ccr-handling, cleanup,
  explore, focus, foresight, lazy-eval, lazy), `session.current`
  (contents `20261019_063036_eafb3d`, 22 bytes incl. newline), `proxy-stderr.log`
  (119 bytes: `Error: failed to bind listener "127.0.0.1:9797" ... Address already
  in use (os error 48)` - evidence the cache proxy was already running when a
  second instance started).
- `~/.hermes/config.yaml`: `plugins.enabled` includes `aphrodite` (line 582);
  `plugins.entries.aphrodite.allow_tool_override: true` (592-593); `aphrodite`
  appears in `known_plugin_toolsets` (1009) and per-platform toolset lists for
  teams/telegram/whatsapp/yuanbao etc. (959-999). Hermes's *own* `compression:`
  section (135-145, `enabled: false`, threshold 0.75, target_ratio 0.2) is an
  independent mechanism, NOT Aphrodite. `proxy:` block (933-935) is Hermes's
  docker-enforcement proxy setting, also unrelated.
- `~/.hermes/cli-config.yaml`: mentions compression only generically for
  worker runs (lines 33-34); no aphrodite keys.
- `~/.hermes/agent-hooks/`: `normalize-dashes.sh`,
  `normalize-dashes-for-execute-code.sh`, `normalize-tabs.sh` - wired as
  Hermes `hooks.post_tool_call` for `write_file|patch` (`config.yaml:600-606`).
  These are text-normalization hooks, not Aphrodite's.
- `~/.hermes/bin/`: auth-cloudflare, browser, browser-use, browser-use-tui,
  browseruse, bu, tirith, uv, uvx (Aphrodite binaries are NOT here; they live
  in `~/.hermes/aphrodite/binaries/` per `README.md:46-47`).
- `~/.hermes/.env` variable names only (values not read): includes
  `APHRODITE_API_KEY` (line 492) - matches `[defaults] api_key` precedence
  comment (`aphrodite.toml.example:71-73`).

## 6. Config surface (`aphrodite.toml.example`, 220 lines)

Priority: `env var > aphrodite.toml > hardcoded default`; search `./aphrodite.toml`
then `~/.hermes/aphrodite/aphrodite.toml` unless `APHRODITE_CONFIG_PATH` (:1-8).
Two independent parsers: the Rust HTTP proxy (`[[proxies]]`, `[defaults]`, four
hot-reload thresholds, auto_expand keys, `preview_max_chars`) and the Hermes
dylib session (`[compression]` engine/chain-split/poll/live_container keys,
`[defaults]`, `[flow]`, `[directives]`, `[prompts]`, `[templates]`) (:10-20).

Key knobs (name = default, meaning):

- `[[proxies]]` name/mode/listen/tool_relay/timeout - cache 9797 timeout 120,
  token 9798 timeout 300, `tool_relay = true` (:37-51).
- `[defaults]` `api_url = "https://api.openai.com"`, `model = "default-model"`,
  `ccr_ttl_seconds = 3600` (:68-70); per-proxy overrides incl. `ccr_db_path`,
  `api_key` (:61-64).
- `[compression]`: `engine_threshold_pct = 30`, `engine_protect_first = 2`,
  `engine_protect_last = 5`, `engine_min_msgs = 8` (:89-92);
  `tool_threshold_token = 256`, `tool_threshold_cache = 2048`,
  `terminal_threshold = 256`, `inline_threshold = 1024` (:93-96, first four
  hot-reload); `auto_expand = true`, `auto_expand_limit = 102400` (:97-98);
  `live_container = false`, `recursive_depth = 5` (:99-100);
  `max_request_body_size` unset = no cap (:101); `catalog_mode` and
  `classifier_poll` are DOCUMENTED BUT UNWIRED (:102-103); `code_multiplier =
  1.0` (:104); `context_engine = true` (status flag for the bridge, :105-109);
  `poll_worker = true`, `auto_reload = true` (:110-111); `chain_split = false`
  with min 2 / max 6 segments (:112-114).
- `[previews]` `model_family = "code_first"`, `code_structure_map = true`
  (unwired), `preview_max_chars = 0` (:120-123).
- `[prompts]` `retrieve_guidance = "minimal"`, `ccr_marker_hint = false`,
  `catalog_intent_hints = false`, and the long `session_inject` first-turn
  orientation text (:129-155).
- `[templates]` - optional preview family / marker / prompts / reverse
  templates (:158-198).
- `[directives]` `active = ["lazy", "lazy-eval", "foresight"]` (:210).
- `[flow]` `budget_chars = 2600` (:221).

## 7. Glossary (as docs define)

- **CCR (Compress-Cache-Retrieve)** - the storage+retrieval system behind
  markers (docs/README.md: "CCR (Compress-Cache-Retrieve) storage").
- **Marker** - `<<<CCR:hash|type|size>>>`, the compact stand-in for compressed
  content (docs/ccr/marker-format.md).
- **Preview** - the enriched type-aware one-line summary shown above/inside a
  marker (`README.md:135-138`).
- **Relay** - the 13 thin tool dispatchers from Hermes into the dylib
  (docs/tool-relay/tools.md).
- **Directive** - a swappable behavioral .md context file loaded into the
  session (`aphrodite.toml.example:204-210`).
- **Center** - a traveling memory annotation embedded in a CCR marker
  (`docs/centers.md`).
- **Flow** - the per-turn injected-context assembler with a char budget
  (`aphrodite.toml.example:216-221`).
- **Context engine** - middle-turn archiving to CCR driven by
  `engine_threshold_pct` etc. (`README.md:156-159`).
- **Auto-expand** - resolving stored content inline instead of emitting a
  marker (`aphrodite.toml.example:97`).
- **Live container** - instant progressive `live` markers for large results
  (`README.md:160-162`).
- **Runtime home** - `~/.hermes/aphrodite/` holding binaries/, directives/,
  ccr.db, logs (`home.rs:5`).

## File:line anchor index (most important)

- `README.md:11-14` - definition; `README.md:97-100` - problem/interception;
  `README.md:133-138` - four layers; `README.md:247-254` - two modes;
  `README.md:258-295` - crate layout.
- `crates/aphrodite/src/lib.rs:1-8` - core crate role; :40-52 - proxy
  feature gating.
- `crates/aphrodite-hermes/src/lib.rs:1-18` - bridge role + diagram;
  :603,:683,:1032 - FFI entry points.
- `crates/aphrodite/src/hooks.rs:317-426` - the compression pipeline;
  :383-398 - hash/store/preview/marker; :665-684 - pre_llm_call context.
- `crates/aphrodite/src/proxy.rs:1154,:2241,:2441,:2773` - proxy_handler,
  tool relay, ccr create, health.
- `vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs:89,145,340-351`
  - SQLite CCR store (ccr_entries).
- `aphrodite.toml.example:88-114` - compression knobs; :129-155 - prompts;
  :204-221 - directives/flow.
- Live: `~/.hermes/config.yaml:582,592-593,959-999,135-145`;
  `~/.hermes/aphrodite/directives/ccr-handling.md`; ccr.db `.schema`
  (ccr_entries); `proxy-stderr.log`; `session.current`.

## Open questions

1. Where exactly the Hermes plugin's Python loader lives on this machine -
   the task brief said `~/.hermes/plugins/` and `README.md:36` symlinks
   `~/.hermes/plugins/aphrodite`, but I did not enumerate `~/.hermes/plugins/`
   (out of bounded scope); config.yaml's `plugins.enabled: aphrodite`
   (line 582) confirms registration.
2. Whether the live instance runs plugin mode, proxy mode, or both - the
   `proxy-stderr.log` bind failure proves a proxy on :9797 is active somewhere,
   but I did not probe the socket.
3. `catalog_mode` and `classifier_poll` are parsed-but-unwired today
   (`aphrodite.toml.example:102-103`) - relevant when porting knobs.
4. The vendored Headroom crate carries the CCR backend and classifier; a port
   must decide whether to vendor it likewise or reimplement
   (`compute_key`, `content_detector`).
