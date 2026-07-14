# Changelog

## v1.3.2 - Correctness Sweep: Compression Losslessness, Proxy Security, Release Channel (2026-07-14)

### Summary

The largest correctness pass since the Rust migration. A fresh full-repo
re-analysis (v2 plan corpus) turned up real regressions left by earlier
work in this same version - most notably that the Hermes JSON-unwrap fix
from v1.3.0 never actually reached the live hook path Hermes calls on every
tool result, and that compressing a Hermes wrapper destroyed the original
bytes permanently. Both are fixed. Also closes a real security gap
(unauthenticated management APIs), a correctness bug that broke every
OpenAI-tools client through the proxy, and an SSE bug that silently killed
long-running streams mid-answer.

**Why 1.3.2, not 1.3.1**: `crates.io` already had `aphrodite`/`aphrodite-hermes`
1.3.1 published (2026-07-13, before any of this entry's fixes existed) -
crate versions are immutable once published, so this work ships as 1.3.2
instead. A GitHub tag `Aphrodite/v1.3.1` was created moments before this was
discovered and does contain this entry's fixes (its binaries are real and
fine to use) - but do not treat "v1.3.1" as a single consistent artifact
across GitHub and crates.io; `v1.3.2` is the first version where both
release channels agree.

### Fixed - Compression pipeline

- **Hermes JSON-unwrap regression on the live hook path** - `8f138c1` (v1.3.0)
  moved wrapper-unwrapping into `aphrodite-hermes`'s `compress_into()`, reached
  only by the explicit `aphrodite_compress`/`aphrodite_test` tools - but the
  hook Hermes actually fires on _every_ tool result
  (`aphrodite_hermes_call_hook("transform_tool_result", ...)`) called core's
  `hooks::transform_tool_result` directly, which has zero unwrapping. The
  "previews show `[json:1items 1L]`" symptom the v1.3.0 fixes chased was only
  ever fixed for the rarely-called explicit tool. `hooks::transform_tool_result`/
  `transform_terminal_output` gained classified variants that accept an
  optional `(content, type)` hint; the bridge computes that hint via
  `unwrap_hermes_result` and passes it through, keeping the core crate
  agent-agnostic.
- **Lossy compression on Hermes wrappers** - `compress_into` stored the
  _extracted_ content under `hash(extracted)`, discarding the original. A
  failed command's `exit_code`/`error` fields were unrecoverable after
  compression, and caller JSON that merely matched a wrapper shape (e.g.
  `{"content":"hi","id":42}`) got permanently collapsed to one field. Both
  hook and tool paths now hash and store the ORIGINAL content always;
  `unwrap_hermes_result`'s output is used only for classification and preview.
- **Empty-`output` terminal wrapper aborted extraction entirely** - a failed
  command with empty stdout (`{"output":"","exit_code":1,"error":"..."}`) is
  exactly the case where the error string _is_ the payload; it now falls
  through to the error/success branches instead of returning `None`.
- Added a 15-case table-driven regression suite for `unwrap_hermes_result` -
  this ~100-line heuristic had been rewritten three times with zero tests.

### Fixed - Proxy correctness & security

- **`tool_calls[].function.arguments` mangling** - the proxy compressed large
  tool-call arguments into a CCR marker string, which a real OpenAI-tools
  client (no Aphrodite plugin) can't parse as JSON, breaking every tool call
  it makes. `function.arguments` is client-executable JSON, not model-facing
  prose - it's never compressed now; `message.content` remains the
  legitimate target.
- **Bearer-token auth on management routes** - `/stats`, `/stats/db`,
  `/history`, `/retrieve`, `/ccr/*`, `/reload`, `/tool/relay`, `/version`,
  `/health/upstream` now require `Authorization: Bearer <token>` when
  `APHRODITE_MGMT_TOKEN` is set (unset = unchanged back-compat behavior, with
  a one-time startup warning). Closes a cross-site-write gap: a hostile local
  page could previously issue a CORS "simple request" that lands as a write
  (seed CCR entries, evict markers via `/reload`) even though it can't read
  the reply. `/health` and `/metrics` stay unauthenticated (health checks,
  Prometheus scrapers); the LLM-proxying route is unaffected.
- **SSE streams cut off mid-answer past the configured timeout** - reqwest's
  client-level `.timeout()` bounds the whole request including the response
  body stream, not just headers. A `"stream": true` request now goes out on a
  separate client with no total timeout (relying on `connect_timeout` +
  `tcp_keepalive` for hang protection instead), so a legitimately slow but
  progressing stream is never severed.
- SSE responses now count bytes into `response_body_bytes` and a new
  `sse_stream_errors` counter observes mid-stream chunk errors - previously
  a stream that died mid-flight recorded a 200 with zero signal anywhere.
- Upstream transport-error bodies (which can embed the upstream URL/host via
  `reqwest::Error`'s `Display`) are now a generic `"upstream request failed"`
  to the client; the detail is still recorded server-side in
  `record_error`/`/stats.last_errors`.
- **DNS-rebinding gap**: an empty/unparseable `Host` header was waved through
  the loopback check instead of rejected - closed; every real caller here
  (curl, the Hermes plugin) already sends `Host`, so there's no legitimate
  case to exempt.
- **`/retrieve` dropped the trailing newline** on full-document retrieval
  (`str::lines()` discards it, `join("\n")` never restored it) - a stored
  file ending in `\n` (nearly every source file) came back one byte short,
  breaking the content-addressing round-trip. Fixed; also added a
  `truncated: bool` field to the response so a client can detect a
  windowed/capped result without parsing the `[lines a-b/total]` header.

### Added - Directives, wired end-to-end

- The Conversational Directives feature (shipped dark in v1.2.3 - no schema,
  no dispatch arm, no bridge wiring) is now reachable from Hermes:
  `aphrodite_directive` is a registered schema + dispatchable tool in the
  bridge, and the bridge's `pre_llm_call` arm actually injects active
  directives' text into the context Hermes reads (previously it only ever
  injected the catalog summary).
- Directive loading no longer gated on `active` being non-empty - the
  shipped template default is `active = []`, so a cold start could never
  discover directives to `add`/`swap` into. Directories now load
  unconditionally when present; `active` only seeds what starts active.
- Directive injection now carries each active directive's full body (minus
  leading `#` markers), not just its first line (a markdown title) - the
  actual behavioral bullets never reached the model before.
- Deduplicated ~90 lines of copy-pasted directive-action logic between the
  core C-ABI `aphrodite_directive` export and `aphrodite_dispatch`'s
  `"directive"` arm into one `directives::handle_action`.

### Fixed - macOS install/setup

- Consolidated every macOS artifact copy (binary, dylib, dev-build fallback)
  through one `install_macos_artifact` helper. Fixes three related bugs:
  a _failed_ (non-zero exit) `ditto` was treated as success (only a spawn
  failure was checked), so the `fs::copy` fallback never ran; the
  `target/release` dev-build fallback path skipped the Gatekeeper treatment
  entirely, reproducing the SIGKILL bug on the most common dev workflow;
  and `install_name_tool`/`xattr` failures were silently swallowed with
  `let _ = ...`, so a machine missing Xcode Command Line Tools got the
  SIGKILL bug back with zero diagnostic. All three now `eprintln!` a
  specific warning, and dylibs get an explicit ad-hoc `codesign -f -s -`
  re-sign step.

### Changed - Submodule workflow

- Local checkouts now always float `plugins/aphrodite`, `vendor/headroom`,
  and `vendor/rtk` to their `Current` branch tip on every checkout/merge,
  auto-committing the resulting pin bump (pathspec-scoped - never sweeps up
  unrelated staged work). Previously `git submodule update` reset to
  whatever SHA happened to be pinned, which could silently sit behind
  `Current` for a long time. A fresh clone's plain `git submodule update
--init --recursive` now lands close to `Current`'s tip too, since the pin
  stays continuously synced rather than only moving via an explicit release
  action.
- Safety preserved from the prior dirty-guard design: a submodule with
  uncommitted changes, or with local commits not yet pushed to its tracking
  branch (checked via `merge-base` ancestry, not just `git status`), is
  skipped and reported rather than force-reset.
- Fixed a remote-resolution bug found while building this: `vendor/rtk`'s
  `origin` remote is the upstream `rtk-ai/rtk` repo (no `Current` branch
  there at all) - the fork actually tracked is under a remote named
  `Source`. The sync now resolves the fetch remote by matching
  `.gitmodules`' configured `url`, not by assuming `origin`.

### Fixed - Documentation accuracy

- Corrected every stale count found against the real built artifacts: tool
  count (12→13, `aphrodite_directive` was missing everywhere), C-ABI export
  count (22→25), content-classifier type count (28→26, matching the
  canonical registry in `docs/ccr/content-types.md`), hook count (a
  long-stale "14 hooks" in two READMEs, actual is 5), skill count (a
  stale "14 bundled skills", actual is 9), Prometheus metric count (31→28,
  and the metrics doc's own listing was completely out of sync with the
  real `/metrics` output - wrong names, missing the `+Inf` bucket relabel).
  Also removed a "zstd decompression" doc section describing dead code
  already removed from `/retrieve` in an earlier pass.
- Fixed two schema-text bugs found in the same pass: `aphrodite_test`'s
  schema advertised a "matrix" mode that never existed (the handler only
  ever branches on `quick` vs everything-else); `aphrodite_rebuild`'s schema
  claimed it "rebuilds from source and installs" - it's a report-only
  handler that can't safely rebuild itself mid-session.

## v1.3.0 - Hermes Wrapper-Unwrap Fix, cargo install Support (2026-07-13)

_(Consolidates unreleased intermediate bumps 1.2.7-1.2.9, none separately
tagged or published.)_

### Summary

Fixes the "tool results not expanding properly" bug: every Hermes tool wraps
its result in a JSON envelope, and Aphrodite was compressing that wrapper
whole - so markers pointed at JSON scaffolding instead of the real content.
(v1.3.2 found and closed the gap where this fix didn't reach the live hook
path - see above.)

### Changed

- Extracted real content from Hermes tool-result JSON wrappers
  (`crates/aphrodite/src/hooks.rs`) - detects the envelope every Hermes tool
  wraps its result in and compresses the meaningful content inside with
  proper re-classification, instead of compressing the wrapper whole.
- Wired the extraction into the actual `aphrodite_compress` path
  (`compress_into()` in the bridge crate) - the initial fix never fired for
  tool-driven compression.
- Consolidated unwrapping into the `aphrodite-hermes` bridge exclusively,
  reverting the core-crate copy - core `aphrodite` stays agent-agnostic;
  `unwrap_hermes_result()` handles terminal output, patch diffs,
  error/success messages, search results, file reads, and generic Hermes
  result fields.
- Improved classification: terminal build output now detects
  `Compiling`/`Finished` → `build_output`, `error[E...]` → `build_error`;
  search results render as grep-style hit previews.
- `aphrodite setup` uses `ditto` on macOS instead of bare `fs::copy` -
  strips extended attributes (code signature, quarantine, provenance) by
  default, with `fs::copy` + `xattr -c` as fallback when `ditto` is
  unavailable. Completes the v1.2.5 Gatekeeper work.
- Added `cargo install aphrodite-hermes` support: a helper binary so the
  crate has an installable `[[bin]]` target (a bare library crate can't be
  `cargo install`ed). Its messaging was corrected in v1.3.2 once it became
  clear `cargo install` cannot distribute the cdylib itself, only this
  helper.
- `.gitguardian.yaml` migrated to the current config format.
- Fixed a non-functional secret-content scanner in
  `check-no-runtime-state.sh` (it only ever grepped filenames, never file
  contents), and wired `.githooks/pre-commit` to actually invoke that guard
  script (it previously only ran `ggshield`, via `exec`, silently skipping
  the repo's own check entirely).
- Fixed a dead `.gitignore` re-inclusion rule for `.hermes/` (a leading
  `./` prefix is invalid gitignore syntax and never matched).

## v1.2.6 - CHANGELOG Backfill, Docs Sync (2026-07-13)

### Summary

Documentation-and-bookkeeping release: backfills the CHANGELOG entries for
v1.2.3 and v1.2.5, and syncs version/doc references across the tree. No
runtime code changes.

### Changed

- CHANGELOG backfill for v1.2.3 (Conversational Directives System, CRLF
  fix, accumulated refactors) and v1.2.5 (macOS Gatekeeper fix).
- Docs/version sync across `README.md`, `crates/aphrodite-hermes/README.md`,
  and the `docs/` tree.

## v1.2.5 - macOS Gatekeeper Fix (2026-07-13)

### Fixed

- **`hermes --tui` SIGKILL on macOS** - two root causes fixed in `aphrodite setup`:
    1. Dylib install names pointed to `target/release/deps/` (stale build paths). macOS
       dynamic linker killed the process when loading from a copied location. Fixed by
       running `install_name_tool -id @rpath/<name>.dylib` after every dylib copy.
    2. Extended attributes (code signature, quarantine) preserved by `fs::copy`.
       macOS Gatekeeper validated these at the source path and killed the process
       when run from the install location. Fixed by running `xattr -c` after every
       binary and dylib copy.

- **Re-run install without `--force`**: `aphrodite setup` no longer blocks when
  the target binary already exists. Binaries and dylibs are always overwritten;
  config is preserved unless `--force` is passed.

- **Dylibs always overwritten on re-run**: removed `dest.exists()` skip in
  `copy_dylibs()` - stale dylibs with wrong install names were never replaced.

## v1.2.3 - Conversational Directives System (2026-07-13)

### Added

- **Conversational Directives System** - lightweight `.md`-based behavioral context
  injected into the LLM via `pre_llm_call`. Directives are short instruction files
  that live between file compression and the LLM's context - never compressed,
  never needing retrieval.
    - `directives/*.md` - 4 built-in directives: `focus`, `explore`, `cleanup`, `foresight`
    - `[directives]` TOML section with `active = [...]`
    - `aphrodite_directive(action, name)` C-ABI tool - `list`, `swap`, `add`, `remove`, `reset`
    - Configurable per user/profile; swappable mid-conversation

### Fixed

- **CRLF line endings** in `download.sh`, `plugin.yaml`, `BINARY_VERSION` -
  caused bash syntax errors on macOS/Linux

### Refactored (accumulated from pre-release commits)

- Feature-gated proxy modules: `#[cfg(feature = "proxy")]` on `config`, `proxy`,
  `retrieve`, `setup` - cdylib no longer links axum/tokio/reqwest unnecessarily
- Removed 7 unused dependencies (tower, tracing-appender, futures-util, http,
  http-body-util, hyper, zstd)
- Deleted `center.rs` (zero callers), `generate_summary` (dead code)
- Extracted `preview.rs` module from lib.rs: `detect_type`, `build_preview`
- Replaced inline catalog JSON with `catalog::build_catalog`
- Renamed proxy duplicate functions: `proxy_detect_content_type`,
  `proxy_build_preview`, `proxy_format_ccr_output`
- Converted `SetupError` to `#[derive(thiserror::Error)]`
- **SSE streaming** - `text/event-stream` responses forwarded chunk-by-chunk;
  64 MB buffer cap for non-streaming bodies
- `.githooks/pre-commit` with ggshield secret scanning
- `Maintain/scripts/ops/check-no-runtime-state.sh` repo-guard script

### Docs

- Fixed stale numeric claims (145-line, 14 hooks, 3 levels → 5)
- Status lines added to `.hermes/plans/` design documents
- Prefetch wording updated to reflect synchronous semantics
- Skill metadata cleanup: removed v0.8.43 markers, 6→28 content types

## v1.2.2 - crates.io Publish + Doc Sync (2026-07-13)

### Published

- **First crates.io publish** - all three crates published in dependency order:
  `aphrodite-headroom-core` (0.1.1) → `aphrodite` (1.2.2) → `aphrodite-hermes` (1.2.2)
- `aphrodite` publish required moving the `include_str!("../../../plugins/aphrodite/__init__.py")`
  into the crate's `templates/` directory so `cargo publish` could package it
- Fixed `aphrodite`'s headroom-core version pin from `0.1.0` → `0.1.1` to match the published crate
- Plugin submodule bumped from `v2.0.5` → `v2.0.6`; sync commit fixed after auto-release
  script's `git update-index --cacheinfo` failed silently

### Benchmarks (headroom-core, release profile, Apple M2 Max)

| Benchmark                      | Result      |
| :----------------------------- | :---------- |
| Auth classify (empty)          | 40.7 ns     |
| Auth classify (payg)           | 80.4 ns     |
| Auth classify (oauth_jwt)      | 122.8 ns    |
| Auth classify (subscription)   | 50.5 ns     |
| CCR put (ST, new keys)         | 453 ns      |
| CCR put (ST, overwrite)        | 42.9 ns     |
| CCR get (ST, hit)              | 119.4 ns    |
| CCR get (ST, miss)             | 38.3 ns     |
| CCR mixed MT (8t, Dashmap)     | 229 µs      |
| CCR mixed MT (8t, Mutex)       | 1.15 ms     |
| Tokenizer (small/medium/large) | 32-50 MiB/s |

### Verified

- `cargo test --workspace`: 1,089+ passed, 0 failed - this command runs both this project's own crates AND the vendored `vendor/headroom` submodule (part of the same Cargo workspace), so the combined figure is not solely a measure of this project's own correctness: 240 in `aphrodite`/`aphrodite-hermes` (the code this changelog actually describes) + 849 in the vendored `aphrodite-headroom-core`
- `cargo build --release -p aphrodite -p aphrodite-hermes`: clean (10.8s)
- GitHub Release `Aphrodite/v1.2.2` auto-created by CI with all 9 cross-platform assets
- All docs scanned and version badges synced to v1.2.2 / v2.0.6
- Classification claim updated: <0.1ms → 40-123 ns (benchmark-verified)
- Real-world savings example added: 216 KB → 12 markers, 54K→240 tokens (225×)

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

- `cargo test --workspace`: 849+ passed, 0 failed - includes the vendored `vendor/headroom` submodule's own test suite (part of the same Cargo workspace), not solely this project's `aphrodite`/`aphrodite-hermes` crates.
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
