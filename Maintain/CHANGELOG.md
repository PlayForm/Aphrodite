# Changelog

## v1.2.2 - crates.io Publish + Doc Sync (2026-07-13)

### Published

- **First crates.io publish** — all three crates published in dependency order:
  `aphrodite-headroom-core` (0.1.1) → `aphrodite` (1.2.2) → `aphrodite-hermes` (1.2.2)
- `aphrodite` publish required moving the `include_str!("../../../plugins/aphrodite/__init__.py")`
  into the crate's `templates/` directory so `cargo publish` could package it
- Fixed `aphrodite`'s headroom-core version pin from `0.1.0` → `0.1.1` to match the published crate
- Plugin submodule bumped from `v2.0.5` → `v2.0.6`; sync commit fixed after auto-release
  script's `git update-index --cacheinfo` failed silently

### Benchmarks (headroom-core, release profile, Apple M2 Max)

| Benchmark | Result |
|:----------|:-------|
| Auth classify (empty) | 40.7 ns |
| Auth classify (payg) | 80.4 ns |
| Auth classify (oauth_jwt) | 122.8 ns |
| Auth classify (subscription) | 50.5 ns |
| CCR put (ST, new keys) | 453 ns |
| CCR put (ST, overwrite) | 42.9 ns |
| CCR get (ST, hit) | 119.4 ns |
| CCR get (ST, miss) | 38.3 ns |
| CCR mixed MT (8t, Dashmap) | 229 µs |
| CCR mixed MT (8t, Mutex) | 1.15 ms |
| Tokenizer (small/medium/large) | 32–50 MiB/s |

### Verified

- `cargo test --workspace`: 1,089+ passed, 0 failed (849 headroom-core + 240 aphrodite crates)
- `cargo build --release -p aphrodite -p aphrodite-hermes`: clean (10.8s)
- GitHub Release `Aphrodite/v1.2.2` auto-created by CI with all 9 cross-platform assets
- All docs scanned and version badges synced to v1.2.2 / v2.0.6

## v1.2.1 - Silent Startup Failures (2026-07-11)

### Fix

- **SQLite CCR directory auto-create** (`crates/aphrodite/src/proxy.rs`) - a
  missing `~/.hermes/aphrodite/` directory previously failed the token
  proxy's SQLite CCR store silently at startup while the cache proxy (in
  memory) kept running - a partial failure invisible to the plugin, which
  piped stderr to `/dev/null`. The binary now creates the parent directory
  before opening the DB.
- **Multi-instance port override finished** (`crates/aphrodite/src/config.rs`,
  `plugins/aphrodite/__init__.py`) - `APHRODITE_CACHE_PORT`/
  `APHRODITE_TOKEN_PORT` now actually override the TOML-configured listen
  ports end-to-end, letting multiple concurrent Hermes Agent instances each
  bind their own proxy pair without editing `aphrodite.toml`. The plugin's
  health check reads the same env vars (previously hardcoded to 9797/9798),
  and proxy startup stderr now logs to `~/.hermes/aphrodite/proxy-stderr.log`
  instead of vanishing.
- **Tracing subscriber initialized before config resolution**
  (`crates/aphrodite/src/main.rs`) - found while verifying the port-override
  fix above: `MultiConfig::resolve()` emits `tracing::info!`/`tracing::warn!`
  diagnostics (mode fallback, port-override application, malformed
  port-override warnings, timeout clamping), but the tracing subscriber
  wasn't registered until after `resolve()` ran, so every one of those
  diagnostics was silently dropped. A malformed `APHRODITE_CACHE_PORT` value
  fell back to the default port with zero log output - indistinguishable
  from not setting an override at all. Reordered startup so the subscriber
  is live before any config resolution happens.

### Verified

- `cargo test --workspace`: 849+ passed, 0 failed.
- Directly exercised both fixes against the built binary: deleted
  `~/.hermes/aphrodite/` and confirmed auto-create + both proxies healthy
  (twice, to confirm idempotency); ran two concurrent instances on default
  vs. custom ports simultaneously with zero collisions; confirmed a
  malformed port override now logs a `WARN` and a valid one logs an `INFO`
  (neither did before the tracing-order fix).
- Live end-to-end via `hermes -z` calling `aphrodite_test` (mode=full):
  3/3 roundtrip checks passed, both proxies healthy, `proxy-stderr.log`
  created and empty.

## v1.2.0 - Headroom Upstream Sync (2026-07-11)

### Headroom Fork Sync

`vendor/headroom` merged forward 313 upstream commits (`95b2333e` → `5e14b8c0`,
2026-06-21 to 2026-07-10), bringing in CCR/TLS/output-shaping improvements while
preserving every Aphrodite-specific customization. Full breakdown, including every
silent merge regression found and fixed, in
[`docs/HEADROOM-FORK-DIFF.md`](../docs/HEADROOM-FORK-DIFF.md#2026-07-11-merge-upstream-sync-to-5e14b8c0).

Highlights:

- CCR store TTL raised 300s → 1800s (session-scale); SQLite is now the default CCR
  backend (was in-memory) - survives proxy worker restarts.
- Additive `NODE_EXTRA_CA_CERTS` trust store (system roots + extra cert) instead of
  outright replacement; new `HEADROOM_TLS_STRICT` toggle for corporate TLS-inspection
  environments.
- Output token reduction (`HEADROOM_OUTPUT_SHAPER`): proxy-side verbosity steering +
  effort routing, learned-terseness mode, measured/estimated savings report.
- `SearchCompressorConfig.group_by_file` grouped output (`rg --heading` style).
- SmartCrusher `lossless_only` strict mode + `compaction_*` heuristic knobs;
  `factor_out_constants` now fully wired end-to-end.
- `headroom doctor`/`update`/`audit`/`output-savings` CLI subcommands restored.

### Fix

- Aphrodite's root `Cargo.toml` didn't request `serde_json/preserve_order`, so
  `headroom-core` built through this workspace (a path dependency, unified feature
  set) produced alphabetically-sorted JSON object keys instead of insertion order -
  silently diverging from the Python dict-repr semantics SmartCrusher's anchor
  matching relies on. Now matches `vendor/headroom`'s own workspace features.
- ~15 silent merge regressions in `vendor/headroom` found via systematic post-merge
  symbol-diffing against upstream: missing files still imported elsewhere, dropped
  functions/constants still referenced, features wired everywhere except their final
  consumption point, and stale test expectations left over from the fork's own
  earlier commits. Full list in the fork-diff doc linked above.

### Chore

- `plugins/aphrodite` submodule bumped to `2.0.5` (type-hint cleanup, dead code
  removal - no functional change).
- CI hardening: pinned `github-push-action` to a specific SHA, macOS x86_64 runner
  switched off the perpetually-queued `macos-13` host, crates.io publish pipeline now
  publishes `aphrodite-headroom-core` before `aphrodite`/`aphrodite-hermes`.
- `--allow-hidden` on `APHRODITE_API_KEY`/`APHRODITE_NOTIFY_KEY` CLI args so secrets
  never leak into process listings or crash logs.
- 16MB content-size guard added to the C ABI's `aphrodite_compress`/
  `aphrodite_transform`/`aphrodite_terminal`/`aphrodite_dispatch` entry points.

## v1.1.0 - Configurable Proxy Ports (2026-07-03)

### Multi-Agent Port Configuration

Resolves [#17](https://github.com/PlayForm/Aphrodite/issues/17) - running
multiple concurrent Hermes Agents on one machine no longer requires patching
and recompiling the source to avoid `:9797`/`:9798` port collisions:

- `aphrodite setup --cache-port <port> --token-port <port>` (or
  `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` env vars) templates the
  generated `aphrodite.toml` and `plugin.yaml` with the chosen ports instead
  of hardcoding `9797`/`9798`.
- `aphrodite_hermes_proxy_health` (the C ABI health probe the Hermes plugin
  calls) now reads `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` from the
  environment at call time instead of hardcoding the default ports, so
  liveness reporting stays accurate for non-default instances.
- Verified end-to-end against a real Hermes v0.18.0 install: two independent
  `aphrodite setup` installs running concurrently on distinct port pairs with
  zero conflicts, plugin registration confirmed via `hermes plugins list`.

### Chore

- Synced `package.json` and `README.md` version references that had drifted
  behind the `crates/aphrodite` / `crates/aphrodite-hermes` Cargo.toml version.

## v0.7.0 - Atomization + Live Containers (2026-06-17)

### Plugin Atomization - 29 Nested Modules

Three monolithic files split into deeply-nested single-responsibility modules:

| Package    | Modules                                                                                                                                                | Max lines |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| `_core/`   | config, store, template, struct, state, **init**                                                                                                       | 244       |
| `_hooks/`  | catalog, classify, diff, files, git, live, prefetch, rebuild, reclassify, search, session, session_helpers, stats, terminal, test, transform, **init** | 245       |
| `_marker/` | classify, compress, marker, parse, preview, **init**                                                                                                   | 246       |

Each file exports exactly one function. No file exceeds 250 lines. Originals
preserved as `.py.bak`.

### Live Containers - Streaming Terminal Output

`aphrodite_poll_container(hash)` - LLM never blocks on terminal output:

- Process runs in background thread, output streams to container
- Marker returned instantly - `<<<LIVE:hash|terminal|streaming>>>`
- Poll anytime for partial output + status (running/done/error)
- Content accumulates as process runs

### CCR_UNRESOLVED Fix - Dual-Store Guarantee

Every proxy compress/fetch now mirrors to inline zlib store:

- `_resolve_one`: proxy fetch → `_inline_store_put`
- `_transform_terminal_hook`: proxy compress → `_inline_store_put`
- `_transform_tool_result`: proxy compress → `_inline_store_put`
- Content always in both proxy SQLite AND inline store

### Persistent Markers - Session Resume

`_recent_markers` persists across restarts:

- `atexit` saves last 100 markers to `~/.hermes/aphrodite/recent-markers.json`
- `on_start()` restores on session begin
- TOC populated from previous session immediately

### Prefetch + ETA Schedule

`aphrodite_prefetch(paths)` - background file read + compress:

- Threads read files, classify, compress via proxy
- Markers returned instantly - agent continues
- `aphrodite_prefetch_status()` - live ETA schedule per file
- Status: LOADING → READY (with elapsed time) → ERROR

### TOC - Table of Contents with Retrieve? Recommendations

`aphrodite_catalog(mode='toc')` - compact decision table:

- Shows every CCR entry with hash, type, size, preview
- Retrieve? column: NO for clean outputs, YES for content worth retrieving
- Agent checks TOC before any retrieval - eliminates blind retrieval reflex

### Classifier Expansion

From 10 types → 28 types:

- New: write_file, log, browser_snapshot, web_search, image_generate, todo,
  memory, cronjob, session_search
- New language support: code_ts (TypeScript), code_sh (Shell)
- All 28 types have TOML templates per model family

### Classifier Poll - Zero-Token Clean Outputs

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

- `[compression]` - 14 knobs (thresholds, engine, classifier poll, code
  multiplier)
- `[previews]` - 4 knobs (model_family, code_structure_map, preview_max_chars)
- `[prompts]` - 3 knobs (retrieve_guidance, ccr_marker_hint,
  catalog_intent_hints)
- `[templates.preview.{family}]` - 18 per-type format strings × 3 families
- `[templates.marker]` - CCR block format + hint string
- `[templates.prompts]` - 5 prompt templates
- `[templates.reverse]` - 25-type key map

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

`[compression].context_engine = true` - no `APHRODITE_CONTEXT_ENGINE=1` needed.
Engine registers automatically at plugin load.

### Post-Rebuild Proxy Auto-Restart

`aphrodite_rebuild()` now: kill proxies → copy binary → restart both → query
version. One call replaces the binary without manual intervention.

### CI - Multi-Platform Builds

- 4 targets: Linux x86_64, macOS arm64, macOS x86_64, Windows x86_64
- Binary naming: full Rust triple (e.g., `aphrodite-aarch64-apple-darwin`)
- Shared cache between Check and Build workflows
- Nightly toolchain everywhere
- Tag trigger: `Aphrodite/v*` - single run per release

### Release Automation

- `scripts/auto-release.sh --minor` for feature bumps
- All 4 version locations auto-bumped: Cargo.toml, \_core/config.py,
  pyproject.toml, **init**.py
- `scripts/release-notes.sh` - shell-safe template generator
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
