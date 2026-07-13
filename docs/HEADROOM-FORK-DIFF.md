# Headroom Fork - Complete Divergence Analysis 🔬

> PlayForm Aphrodite embeds **Headroom** as a git submodule at `vendor/headroom/`.
> This is a **custom fork** of the original upstream project. This document
> catalogues every change - what was ripped out, rewritten, hardened, and
> added.

**Current baseline**: merged at upstream `5e14b8c0` (2026-07-10, "fix(memory/sync):
don't clobber memories sharing a first line (#1976)"). See
[2026-07-11 Merge](#2026-07-11-merge-upstream-sync-to-5e14b8c0) below for what changed
in the most recent sync. The sections after that describe the original June 2026
rip-out/hardening pass and are kept for historical context.

**Original baseline** (superseded): merged at upstream `bcabc5cb` (2026-06-19,
"fix(providers): update DeepSeek V3 context limit from 128K to 1M").
**Divergence at that point**: 47 commits, 313 files changed, **+5,359 / -29,715 lines** (net: -24,356).

---

## 2026-07-11 Merge - Upstream Sync to `5e14b8c0`

**Range**: `95b2333e` (2026-06-21, our previous merge point, "chore: release main (#1274)")
→ `5e14b8c0` (2026-07-10). **313 upstream commits**, 2,380 files changed,
**+89,520 / -274,067 lines** in `vendor/headroom` (the huge deletion count is upstream's
own `agent-evals` extraction into a standalone repo, not something this fork did).

This was a standard `git merge upstream/main` - real, resolved conflicts in 37 files,
plus (see below) a number of **silent regressions** that git's 3-way merge introduced
in files with _no_ conflict markers at all. All were found via systematic post-merge
symbol-diffing against upstream and fixed before this state was considered stable.

### What Came In From Upstream (highlights)

- **CCR defaults**: `CompressionStore`/`CCRConfig` TTL raised 300s → 1800s (session-scale,
  matches the 30-minute language already in the miss message); SQLite is now the
  **default** CCR backend (was in-memory) - restart-safe across proxy worker processes.
- **TLS hardening**: `NODE_EXTRA_CA_CERTS` now builds an _additive_ trust store (system
  roots + the extra cert) instead of replacing the trust store outright (upstream #998);
  new `HEADROOM_TLS_STRICT` toggle for corporate MITM/TLS-inspection environments running
  Python 3.13 + OpenSSL 3.x strict mode.
- **Output token reduction**: new `HEADROOM_OUTPUT_SHAPER` proxy-side verbosity steering +
  effort routing, with a learned-terseness mode (`headroom learn --verbosity`) and a
  measured/estimated savings report (`headroom output-savings`).
- **Search compressor**: new `group_by_file` grouped-output mode (`rg --heading` style),
  plumbed through Python, the PyO3 binding, and the Rust `SearchCompressorConfig`.
- **SmartCrusher (Rust)**: new `lossless_only` strict mode and `compaction_*` heuristic
  knobs (`core_field_fraction`, `heterogeneous_core_ratio`, `max_flatten_inner_keys`,
  `min_buckets`, `max_buckets`); `factor_out_constants` field-stripping is now fully wired
  end-to-end (was silently inert before this merge caught it, see below).
- **Net-cost gating**: P3a batch-reclaim + P3b idle-timer `P_alive` decay for the
  frozen-message-floor unlock in `content_router.py`.
- **CLI**: `headroom doctor`, `headroom update`, `headroom audit`, `headroom output-savings`
  subcommands; new `CLAUDE_CONFIG_DIR` support in the MCP registrar; `headroom learn` now
  scans subagent/workflow transcripts, not just the main session.
- **Bedrock**: server-side route handling moved back into scope as
  `BedrockHandlerMixin`/`bedrock_api_url` (Python-side passthrough; the Rust
  `crates/headroom-proxy/src/bedrock/` path is unaffected either way).
- **Integrations**: `langchain`/`agno` subpackages, previously stripped by this fork's
  original June rip-out (see below), came back in via the merge.
- Dozens of smaller fixes: Vertex AI non-default-region routing, `openai_api_url`
  passthrough for custom OpenAI-compatible endpoints, `seconds_since_activity()` on the
  prefix cache tracker, thread-local tree-sitter parsers (fixes a cross-thread panic),
  non-ASCII byte<->char offset conversion in code masking, and more.

### Decision: Restored Subsystems Kept, Not Re-Stripped

The fork's original June 2026 pass (see the sections below) deliberately deleted
`headroom/integrations/{langchain,agno}/`, `headroom/proxy/{output_shaper,output_savings,
verbosity_controller,runtime_env,cc_switch_reconciler}.py`, the Python
`handlers/bedrock.py`, `cache/backends/sqlite.py`, and `cli/{doctor,update,audit,
output_savings}.py` as "Aphrodite is Hermes-only, no langchain/orchestration" /
"moved to Rust." This merge's conflict resolution treated their absence as upstream
content missing from our side and restored all of them.

**Decision (2026-07-11): keep them restored.** They're additive, don't break
`crates/aphrodite`'s build or tests, and ripping them back out now would be a second,
separate, riskier change with no functional upside for this release. Revisit only if
someone wants to shrink the Python package's dependency footprint again.

### Silent Merge Regressions Found & Fixed

Git's 3-way merge auto-resolved a number of non-conflicting hunks _incorrectly_ -
plausible-looking code with the right function names but stale or missing bodies.
None of these had a conflict marker; all were caught by comparing the resolved file's
symbol table and, where that wasn't enough, its full body, against upstream's intended
version. Categorized:

- **Entirely missing files** a resolved file still imported: `headroom/cache/backends/sqlite.py`,
  plus ~68 other new-upstream files (langchain/agno subpackages, `headroom/audit/`,
  `headroom/cli/{doctor,update,audit,output_savings}.py`, `headroom/proxy/{output_savings,
output_shaper,verbosity_controller,cc_switch_reconciler,handlers/bedrock}.py`, live-test
  fixtures, and more) that were silently dropped by the merge and never showed up as
  conflicts at all.
- **Dropped functions/constants still referenced elsewhere**: `CCR_MISS_MESSAGE`,
  `seconds_since_activity()`, `should_stamp_codex_client()`, `CODEX_RESPONSES_PATH`,
  `SearchCompressorConfig.group_by_file` (and its PyO3 binding), `ProxyConfig.bedrock_api_url`,
  `create_proxy_backend`'s `openai_api_url` param, `/health`'s `runtime_env` field.
- **Features fully wired everywhere except the final consumption point** (silently inert):
  Rust `factor_out_constants` was threaded through config/types/analyzer/planning but
  `crusher.rs::execute_plan` never consumed it; `_get_smart_crusher()` built a fresh
  default `SmartCrusherConfig` instead of `self.config.smart_crusher or SmartCrusherConfig()`,
  discarding any full-config override; the entire "P3a/P3b net-cost idle-decay" block in
  `content_router.py` was missing even though `_net_cost_allows` (its consumer) was
  present and being called.
- **Stale test expectations left over from the fork's own earlier commits**: a dict-repr
  test asserting sorted keys (should be insertion-order - Python 3.7+ semantics); CCR
  store TTL / `lossless_min_savings_ratio` tests pinned to values the fork had drifted
  away from its own Rust source of truth (`0.30` vs upstream+Rust's `0.15`).
- **Regression reverted by our own fork's earlier commit, not the merge**: the
  `NODE_EXTRA_CA_CERTS` additive-trust-store fix (#998) had been silently reverted back
  to replacement semantics sometime before this merge; restored to upstream's version.
- **Cross-workspace feature-flag drift** (Aphrodite-side, not headroom): the Aphrodite
  super-repo's root `Cargo.toml` didn't request `serde_json/preserve_order`, so when
  `headroom-core` builds as part of Aphrodite's Cargo workspace (path dependency feature
  unification), JSON object keys came out alphabetically sorted instead of
  insertion-ordered - diverging from Python's dict-repr semantics that SmartCrusher's
  anchor matching relies on. Fixed in `Cargo.toml` to match `vendor/headroom`'s own
  workspace features.

### Verification

- `cargo test --workspace --all-features` in `vendor/headroom`: **849 passed, 0 failed**.
- `cargo build --workspace --release` + `cargo test --workspace --release` at the
  Aphrodite super-repo root (covers `crates/aphrodite` + `crates/aphrodite-hermes` +
  the vendored `headroom-core`, unified feature set): **all test binaries pass**.
- `vendor/headroom` Python suite (`pytest tests`, ~8,500 tests): all real failures
  root-caused and fixed; remaining apparent failures were environment leakage (this
  session's own `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/`CLAUDE_CONFIG_DIR` bleeding into
  the test process) and test-order-dependent module-level state in a few `content_router`
  tests that pass cleanly in isolation.
- Hermes plugin FFI smoke test (`aphrodite_compress` → `aphrodite_retrieve` roundtrip via
  ctypes against the freshly built dylib) and a live end-to-end `hermes -z` session
  (real DeepSeek-backed tool calls through the plugin + proxy) both passed. Hit and fixed
  one unrelated macOS Gatekeeper issue: a freshly `cargo build`'d, unsigned local dylib
  gets `SIGKILL`'d ("Code Signature Invalid") when `dlopen`'d from the Hermes agent's
  Python process; ad-hoc `codesign -s -` resolves it for local dev/test builds.

---

## Aphrodite Fix-Pass Deltas (2026-07-13 execution, `.plans/` report 06)

Surgical fork-side changes made while executing `.plans/06-state-concurrency-storage.md`'s
task list (uncommitted as of this writing - the executor contract requires recording any
vendor edit here regardless of commit status):

- **T3 (F3, `ccr/backends/sqlite.rs::open`)**: added `conn.busy_timeout(Duration::from_secs(5))`
  after the WAL/synchronous pragmas. Default busy timeout is 0 (fail-fast); with multiple
  aphrodite processes sharing one `ccr.db` (two token proxies both defaulting to the same
  path), a write colliding with another process's write/checkpoint returned `SQLITE_BUSY`
  immediately, and the aphrodite proxy layer's `ccr_put` (now returning `bool`, see report 02's
  F4 fix) would have nothing to retry against - it would just observe the failure. Blocking up
  to 5s under normal cross-process contention turns a spurious permanent failure into a brief
  stall.
- **T12 (F10, `ccr/backends/sqlite.rs`)**: `put` now also runs the same debounced lazy-purge
  sweep `get` already ran - a compress-heavy, retrieve-light workload (the common case: most
  markers are never expanded) never purged anything on deployments whose only traffic is
  `put`, so `ccr.db` only ever grew. Also added a `PRAGMA user_version` schema-version check
  in `open()` (current schema is version 1) as the hook a future column change can branch on,
  instead of relying solely on `CREATE TABLE IF NOT EXISTS` silently keeping whatever schema
  an older binary left on disk.

## Commit Timeline 📅

All commits are on the `Current` branch. Oldest first.

### Phase 1: Rip-Out (2026-06-15)

| Commit     | Date   | Description                                                                    | Impact                       |
| ---------- | ------ | ------------------------------------------------------------------------------ | ---------------------------- |
| `7ba6e30b` | Jun 15 | **refactor: rip out non-coding integrations** - langchain, agno, litellm, asgi | -5,354 lines, 16 files       |
| `126543f5` | Jun 15 | chore: remove liteLLM dependency - unused by aphrodite                         | -1 line (pyproject)          |
| `6cebd79b` | Jun 15 | docs: add Hermes-agent integration recommendations                             | docs only                    |
| `bf133795` | Jun 15 | feat: add `hermes_demo/` - full Hermes-3 + Aphrodite proxy integration suite   | +1,653 lines (9 files added) |

### Phase 2: Hardening (2026-06-16)

| Commit     | Date   | Description                                                                                                                    | Impact              |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------------------------ | ------------------- |
| `9f9a3253` | Jun 15 | fix: add DeepSeek chat/r1/v4/v4-pro tokenizer mappings                                                                         | Rust                |
| `a15e6c3b` | Jun 15 | fix: preserve `x-headroom-workspace` header for CCR cross-project scoping                                                      | proxy               |
| `76e448b0` | Jun 15 | fix: raise relevance threshold 0.3->0.5 + add coding stop words                                                                | Rust                |
| `57157225` | Jun 15 | perf: replace `list.pop(0)` with `deque.popleft()` in context_tracker LRU                                                      | Python              |
| `a1396ff3` | Jun 16 | fix: CCR regex, loopback exempt, threshold invert, headers passthrough, savings accumulate, rate limit exempt, image auto mode | +235/-75 (9 files)  |
| `e1afeca2` | Jun 16 | fix: CCR backends - in_memory LRU, sqlite spawn_blocking, redis pipeline                                                       | +37/-25 (5 files)   |
| `d4a832cb` | Jun 16 | hardening: poison-tolerant locks, debounced purge, queue compaction                                                            | +213/-36 (2 files)  |
| `e2cca08d` | Jun 16 | test: 8 regression tests for auth classification drift                                                                         | +145/-0             |
| `6f284bb1` | Jun 16 | fix(build): sha2 0.11 hex format - LowerHex removed from Array output                                                          | +15/-13 (5 files)   |
| `b2512027` | Jun 16 | tune: coding-tuned policy defaults - looser lossy caps, higher volatile thresholds                                             | +174/-192 (2 files) |

### Phase 3: Production Polish (2026-06-17)

| Commit     | Date   | Description                                                                                   | Impact          |
| ---------- | ------ | --------------------------------------------------------------------------------------------- | --------------- |
| `1dc3dae0` | Jun 17 | fix: SQLite overflow clamps + in-memory evict iteration cap                                   | +6/-7 (2 files) |
| `536ce886` | Jun 17 | **fix: compute_key 24->40 hex chars** - safe for persistent backends with millions of entries | +7/-9           |
| `8823704a` | Jun 16 | ci: bump dtolnay/rust-toolchain                                                               | CI              |
| `483a7681` | Jun 16 | ci: bump codecov/codecov-action 4->7                                                          | CI              |
| `8399a567` | Jun 16 | ci: bump actions/github-script 7->9                                                           | CI              |
| `21fa2554` | Jun 16 | ci: bump aiohttp 3.14.0->3.14.1                                                               | deps            |
| `7df6af0d` | Jun 16 | ci: bump cryptography 46.0.7->48.0.1                                                          | deps            |
| `cd305bfe` | Jun 16 | ci: bump starlette 1.0.1->1.3.1                                                               | deps            |
| `f3a92136` | Jun 16 | ci: bump python-multipart 0.0.27->0.0.31                                                      | deps            |
| `6feb5ead` | Jun 17 | ci: bump pyjwt 2.11.0->2.13.0                                                                 | deps            |
| `8c46f5cd` | Jun 17 | ci: bump vite 8.0.10->8.0.16 (sdk/typescript)                                                 | deps            |
| `d8c91220` | Jun 17 | ci: bump form-data 4.0.5->4.0.6 (sdk/typescript)                                              | deps            |
| `bb1847e9` | Jun 17 | ci: bump vite 8.0.10->8.0.16 (plugins/openclaw)                                               | deps            |
| `fa50e5b6` | Jun 17 | ci: bump js-yaml 4.1.1->4.2.0 (docs)                                                          | deps            |

### Phase 4: Merge / Finalize (2026-06-17-18)

| Commit     | Date   | Description                               |
| ---------- | ------ | ----------------------------------------- |
| `f5d9fb04` | Jun 17 | Merge dependabot PR #7 (python-multipart) |
| `a0fc3a36` | Jun 17 | Merge dependabot PR #6 (starlette)        |
| `1e5c5f31` | Jun 17 | Merge dependabot PR #5 (cryptography)     |
| `378b1bf2` | Jun 17 | Merge dependabot PR #4 (aiohttp)          |
| `b6f2dfd6` | Jun 17 | Merge dependabot PR #3 (github-script)    |
| `474bcd7b` | Jun 17 | Merge dependabot PR #2 (codecov-action)   |
| `00e373ab` | Jun 17 | Merge dependabot PR #1 (rust-toolchain)   |
| `1aea5227` | Jun 17 | Merge dependabot PR #8 (pyjwt)            |
| `aa920a42` | Jun 17 | Merge dependabot PR #12 (js-yaml)         |
| `9613c1d7` | Jun 17 | Merge dependabot PR #10 (form-data)       |
| `1b713c55` | Jun 17 | Merge dependabot PR #11 (vite/openclaw)   |
| `5d607787` | Jun 17 | Merge dependabot PR #9 (vite/sdk)         |
| `eebf943f` | Jun 18 | style: Prettier format pass               |
| `e4767570` | Jun 18 | style: Prettier format pass               |

Three interstitial "save" commits (`702bdfa5`, `6ee1747f`, `c97f135d`, `50644506`) capture work-in-progress states.

---

## What We Deleted 🗑️

77 files removed entirely (~24,000 lines).

### Integration Rip-Outs

| Module                                      | Files   | Lines  | Reason                                                |
| ------------------------------------------- | ------- | ------ | ----------------------------------------------------- |
| `headroom/integrations/langchain/`          | 7 files | ~3,500 | Aphrodite is Hermes-only - no langchain/orchestration |
| `headroom/integrations/agno/`               | 4 files | ~1,400 | Same                                                  |
| `headroom/integrations/asgi.py`             | 1 file  | 239    | No ASGI/FastAPI wrapping needed                       |
| `headroom/integrations/litellm_callback.py` | 1 file  | 187    | LiteLLM removed entirely                              |

> **Note**: the 2026-07-11 merge (above) restored these subsystems from upstream.
> Current decision is to keep them restored - see the "Decision" callout above.

### Removed Subsystems

| File                                     | Lines   | Role                                                                           |
| ---------------------------------------- | ------- | ------------------------------------------------------------------------------ |
| `headroom/proxy/output_shaper.py`        | 360     | Intelligent output reformatting - removed in favor of Aphrodite's own pipeline |
| `headroom/proxy/output_savings.py`       | 501     | Cost/savings tracking - Aphrodite has its own Prometheus metrics               |
| `headroom/proxy/verbosity_controller.py` | 108     | Verbosity learning - removed                                                   |
| `headroom/proxy/runtime_env.py`          | 151     | Runtime environment detection                                                  |
| `headroom/proxy/cc_switch_reconciler.py` | 192     | Claude Code switch reconciler                                                  |
| `headroom/proxy/handlers/bedrock.py`     | 300     | Python Bedrock handler - moved to Rust (`crates/headroom-proxy/src/bedrock/`)  |
| `headroom/learn/verbosity.py`            | 473     | Verbosity learning models                                                      |
| `headroom/cli/doctor.py`                 | 412     | Diagnostic CLI                                                                 |
| `headroom/cli/update.py`                 | 341     | Auto-update checker                                                            |
| `headroom/cli/audit.py`                  | 107     | Audit CLI                                                                      |
| `headroom/cli/output_savings.py`         | 61      | Output savings CLI                                                             |
| `headroom/audit/`                        | 4 files | 762                                                                            | Audit modules (reads, codex, maturation) |
| `headroom/cache/backends/sqlite.py`      | 275     | Python SQLite backend - Rust version used instead                              |
| `headroom/update_check.py`               | 303     | Update notification                                                            |
| `headroom/providers/codex/threads.py`    | 97      | Codex thread management                                                        |
| `headroom/providers/mistral_vibe/`       | 2 files | 59                                                                             | Mistral Vibe runtime                     |
| `headroom/perf/analyzer.py`              | 189     | Performance analyzer                                                           |

> **Note**: every file in this table was restored by the 2026-07-11 merge (above),
> except `headroom/learn/verbosity.py`, `headroom/update_check.py`,
> `headroom/providers/codex/threads.py` (restored under a different path -
> `headroom/providers/codex/threads.py` did come back), `headroom/providers/mistral_vibe/`,
> and `headroom/perf/analyzer.py`, which remain absent. Check the working tree if in doubt;
> this table describes June 2026 state only.

### Removed Docs

| File                                                 | Lines |
| ---------------------------------------------------- | ----- |
| `docs/content/docs/ci-cd-flows.mdx`                  | 242   |
| `docs/content/docs/claude-code-vertex.mdx`           | 117   |
| `docs/lib/cn.ts`                                     | 6     |
| `docs/lib/layout.shared.ts`                          | 10    |
| `docs/lib/shared.ts`                                 | 8     |
| `docs/lib/source.ts`                                 | 35    |
| `docs/lib/telemetry.ts`                              | 46    |
| `docs/output-token-reduction-guide.md`               | 145   |
| `docs/proposals/output-token-reduction.md`           | 461   |
| `docs/proposals/vertex-claude-compression-review.md` | 255   |

### Removed Tests (~50 files)

All tests for removed subsystems: `test_audit_*`, `test_output_savings*`, `test_output_shaper*`,
`test_verbosity_*`, `test_update_*`, `test_runtime_env*`, `test_litellm_*`, `test_cc_switch_*`,
`test_vertex_claude_*`, `test_bedrock_passthrough*`, `test_serena_migrate*`, `test_wrap_claude*`,
`test_wrap_vibe*`, `test_codex_client*`, `test_compressor_config*`, `test_learn/subagent*`,
`test_live/*`, `test_ccr_sqlite_backend*`, `test_proxy_*`, `scripts/tests/*`.

---

## What We Added ➕

10 files, 2,021 lines.

### Hermes Demo Suite (`examples/hermes_demo/`)

| File                                       | Lines | Purpose                                     |
| ------------------------------------------ | ----- | ------------------------------------------- |
| `hermes_via_proxy_demo.py`                 | 460   | End-to-end Aphrodite proxy integration demo |
| `hermes_agent_eval.py`                     | 396   | Agent evaluation harness                    |
| `test_hermes_ccr.py`                       | 298   | CCR unit tests                              |
| `hermes_mcp_client.py`                     | 226   | MCP client integration                      |
| `hermes_bundle_demo.py`                    | 201   | Bundle/packaging demo                       |
| `README.md`                                | 100   | Demo documentation                          |
| `fixtures/hermes_tool_call.json`           | 23    | Test fixtures                               |
| `fixtures/hermes_tool_response_large.json` | 5     | Large response fixture                      |
| `__init__.py`                              | 17    | Package init                                |

### Other Additions

| File                          | Lines | Purpose                                  |
| ----------------------------- | ----- | ---------------------------------------- |
| `examples/recommendations.md` | 369   | Hermes agent integration recommendations |

---

## What We Modified 🔧

~226 files modified across the entire codebase. Key areas:

### Rust Core (`crates/headroom-core/`) - 28 files

| Area                   | Change Summary                                                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **CCR hash**           | Length 24->40 hex chars (collision safety at scale). Algorithm remains BLAKE3. Python uses SHA-256 independently. |
| **CCR backends**       | Poison-tolerant locks, debounced purge, queue compaction; in-memory LRU, SQLite spawn_blocking, redis pipeline    |
| **SQLite**             | Overflow clamps, evict iteration caps                                                                             |
| **Compression policy** | Coding-tuned defaults: looser lossy caps, higher volatile thresholds                                              |
| **Relevance**          | Threshold 0.3->0.5, coding stop words added                                                                       |
| **Live zone**          | Major rewrite (149 lines changed) - anthropic + openai + responses compression hooks                              |
| **Smart crusher**      | Compaction classifier, formatter, walker, config, crusher, hashing - all tuned                                    |
| **Search compressor**  | 33 lines changed                                                                                                  |
| **Tokenizers**         | DeepSeek chat/r1/v4/v4-pro mappings added                                                                         |
| **Build**              | sha2 0.11 hex format fix (LowerHex removed from Array)                                                            |

### Rust Proxy (`crates/headroom-proxy/`) - 13 files

| Area                    | Change Summary                                                                                                                    |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **Bedrock**             | invoke.rs (186 lines changed), eventstream (20), streaming (98) - rewrote Python->Rust                                            |
| **Cache stabilization** | Anthropic cache control, drift detector, openai cache key, volatile detector - all tuned                                          |
| **SSE**                 | Anthropic + OpenAI responses + framing - 38 lines changed                                                                         |
| **Vertex**              | Envelope, raw_predict - 38 lines changed                                                                                          |
| **Headers**             | `x-headroom-workspace` preservation for cross-project CCR scoping                                                                 |
| **Proxy core**          | 72 lines changed - loopback exempt, threshold invert, headers passthrough, savings accumulate, rate limit exempt, image auto mode |

### Python (`headroom/`) - ~40 files

| Area                    | Change Summary                                                              |
| ----------------------- | --------------------------------------------------------------------------- |
| **`wrap.py`**           | -498 lines: simplified agent wrapping (removed agno, langchain, asgi paths) |
| **`content_router.py`** | -664 lines: simplified routing                                              |
| **`server.py`**         | -198 lines: stripped integration hooks                                      |
| **`proxy_routes.py`**   | -181 lines: simplified                                                      |
| **`install.sh`**        | -98 lines: macOS LaunchAgent rewritten                                      |
| **`dashboard.html`**    | -344 lines: PlayForm branding                                               |
| **`code_handler.py`**   | -109 lines: simplified                                                      |
| **Context tracker**     | `list.pop(0)->deque.popleft()` perf fix                                     |

### Configuration & Build

| File             | Change                                                     |
| ---------------- | ---------------------------------------------------------- |
| `pyproject.toml` | -368 lines: removed liteLLM, langchain, agno deps          |
| `Cargo.toml`     | -54 lines: removed unused features                         |
| `Cargo.lock`     | Deleted entirely (-5,557 lines) - regenerated from scratch |
| `Dockerfile`     | -15 lines                                                  |
| `deny.toml`      | -24 lines                                                  |
| `.gitignore`     | Updated                                                    |
| `uv.lock`        | Removed                                                    |

### CI / Workflows

| File                                       | Change    |
| ------------------------------------------ | --------- |
| `.github/workflows/ci.yml`                 | -16 lines |
| `.github/workflows/docs.yml`               | -37 lines |
| `.github/workflows/install-native-e2e.yml` | -37 lines |
| `scripts/build_rust_extension.sh`          | -12 lines |
| `scripts/install-git-hooks.sh`             | -28 lines |
| `scripts/validate-workflows.sh`            | -32 lines |

### Docs & Release

| File                                  | Change     |
| ------------------------------------- | ---------- |
| `docs/content/docs/releases.mdx`      | -448 lines |
| `docs/content/docs/configuration.mdx` | -18 lines  |
| `docs/content/docs/proxy.mdx`         | -22 lines  |
| `CHANGELOG.md`                        | -66 lines  |
| `README.md`                           | -75 lines  |
| `PR.md`                               | -340 lines |
| `.release-please-manifest.json`       | Updated    |
| `wiki/configuration.md`               | -18 lines  |
| `wiki/troubleshooting.md`             | -17 lines  |

### Tests

| Area                                        | Files | Change                         |
| ------------------------------------------- | ----- | ------------------------------ |
| `tests/test_proxy_dashboard_stats_cache.py` | 1     | -93 lines                      |
| `tests/test_proxy_scalability.py`           | 1     | -30 lines                      |
| `tests/test_cli/test_wrap_copilot.py`       | 1     | -39 lines                      |
| `tests/test_provider_proxy_routes.py`       | 1     | -68 lines                      |
| `tests/test_auth_mode.rs`                   | 1     | +145/-0 (new regression tests) |
| Various                                     | ~20   | Minor adjustments              |

---

## Directory-Level Impact 📊

Where the changes concentrate (by `git diff --dirstat`):

| Directory                                            | % of Changes |
| ---------------------------------------------------- | ------------ |
| `tests/`                                             | 13.4%        |
| `headroom/proxy/`                                    | 5.4%         |
| `crates/headroom-core/src/transforms/`               | 2.8%         |
| `headroom/cli/`                                      | 2.8%         |
| `headroom/integrations/langchain/`                   | 2.8%         |
| `crates/headroom-core/src/transforms/smart_crusher/` | 2.5%         |
| `crates/headroom-proxy/src/`                         | 2.2%         |
| `.github/workflows/`                                 | 2.2%         |
| `examples/hermes_demo/`                              | 2.2%         |
| `crates/headroom-proxy/src/bedrock/`                 | 1.9%         |
| `crates/headroom-core/tests/`                        | 1.9%         |
| `docs/content/docs/`                                 | 1.9%         |

---

## Summary 📋

The PlayForm fork transforms Headroom from a **general-purpose LLM proxy** (supporting
langchain, agno, asgi, liteLLM, Claude Code, Copilot, Cursor, aider, etc.) into a
**focused Hermes Agent compression engine** - the core that Aphrodite extends.

| Axis                | Stock Headroom | PlayForm Fork                                                                                           |
| ------------------- | -------------- | ------------------------------------------------------------------------------------------------------- |
| Integration surface | 7+ frameworks  | Hermes-focused (langchain/agno present again after 2026-07-11 merge, unused by Aphrodite itself)        |
| Python lines        | ~35,000        | ~11,000 pre-merge; grew back toward upstream after the 2026-07-11 sync (see above)                      |
| Hash algorithm      | BLAKE3         | BLAKE3 (length 24->40 hex). Python side uses SHA-256 - consistent internally, differs across languages. |
| Hash length         | 24 hex chars   | 40 hex chars                                                                                            |
| CCR backends        | Basic          | Poison-tolerant, debounced, pipelined; SQLite is now the default (upstream, 2026-07-11)                 |
| Proxy modes         | Single         | Dual (cache :9797 + token :9798 via TOML)                                                               |
| Compression policy  | Generic        | Coding-tuned (looser lossy, higher volatile)                                                            |
| Relevance           | 0.3 threshold  | 0.5 + coding stop words                                                                                 |
| Tokenizers          | Standard       | + DeepSeek v4 family                                                                                    |
| Bedrock             | Python handler | Rust rewrite in `headroom-proxy`; Python `handlers/bedrock.py` also present again (upstream)            |
| Branding            | Stock          | PlayForm identity                                                                                       |
| Test suite          | Full upstream  | ~8,500 tests as of 2026-07-11 (grew back toward upstream's full suite after the sync)                   |
| CI                  | Full matrix    | Simplified (no langchain/agno/litellm builds)                                                           |

---

## Relationship Diagram 🗺️

```
chopratejas/headroom (upstream)
    │
    └── forked ──→ PlayForm/Headroom (our fork)
                       │
                       ├── Current branch - all fork changes live here
                       │
                       └── tracked as submodule at vendor/headroom/
                              │
                              └── Aphrodite crate depends on headroom-core
```

---

## Windows Compilation Status 🪟

Headroom (both original upstream and our fork) has conditional compilation for
Windows, but has never been end-to-end tested there.

| Area                                                               | Status                                                                                                             |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Core Rust** (tokenizers, regex, sha2, rusqlite, dashmap, flate2) | Pure Rust, cross-platform ✓                                                                                        |
| **Signal handling**                                                | Correctly gated: `#[cfg(unix)]` / `#[cfg(not(unix))]` ✓                                                            |
| **`build.rs`**                                                     | Skips glibc shim on non-Linux-gnu ✓                                                                                |
| **ONNX Runtime** (`fastembed` → `ort`)                             | Windows uses `ort-load-dynamic` (avoids DirectML SDK link-libs), but DLL search paths and CRT linkage are untested |
| **CI**                                                             | `windows-native-wrapper` job only runs Python installer tests - zero `cargo build` or `cargo test`                 |
| **`magika`** (content classifier)                                  | Depends on ONNX Runtime, same Windows caveat                                                                       |

**Bottom line**: it _should_ compile with `cargo build --no-default-features`
(skipping `fastembed`/`magika` → no ONNX Runtime). Full build with ONNX needs
someone to actually attempt `cargo build` on Windows and fix whatever linker
errors surface. The code has the right guards - the gap is CI coverage, not
architecture.
