# Headroom Fork - Complete Divergence Analysis 🔬

> PlayForm Aphrodite embeds **Headroom** as a git submodule at `vendor/headroom/`.
> This is a **custom fork** (`github.com/PlayForm/Headroom`) of the original upstream
> (`github.com/chopratejas/headroom`). This document catalogues every change -
> what was ripped out, rewritten, hardened, and added.

**Baseline**: merged at upstream `bcabc5cb` (2026-06-19, "fix(providers): update DeepSeek V3 context limit from 128K to 1M").
**Divergence**: 47 commits, 313 files changed, **+5,359 / −29,715 lines** (net: −24,356).

---

## Commit Timeline 📅

All commits are on the `Current` branch. Oldest first.

### Phase 1: Rip-Out (2026-06-15)

| Commit     | Date   | Description                                                                    | Impact                       |
| ---------- | ------ | ------------------------------------------------------------------------------ | ---------------------------- |
| `7ba6e30b` | Jun 15 | **refactor: rip out non-coding integrations** - langchain, agno, litellm, asgi | −5,354 lines, 16 files       |
| `126543f5` | Jun 15 | chore: remove liteLLM dependency - unused by aphrodite                         | −1 line (pyproject)          |
| `6cebd79b` | Jun 15 | docs: add Hermes-agent integration recommendations                             | docs only                    |
| `bf133795` | Jun 15 | feat: add `hermes_demo/` - full Hermes‑3 + Aphrodite proxy integration suite   | +1,653 lines (9 files added) |

### Phase 2: Hardening (2026-06-16)

| Commit     | Date   | Description                                                                                                                    | Impact              |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------------------------ | ------------------- |
| `9f9a3253` | Jun 15 | fix: add DeepSeek chat/r1/v4/v4‑pro tokenizer mappings                                                                         | Rust                |
| `a15e6c3b` | Jun 15 | fix: preserve `x-headroom-workspace` header for CCR cross‑project scoping                                                      | proxy               |
| `76e448b0` | Jun 15 | fix: raise relevance threshold 0.3→0.5 + add coding stop words                                                                 | Rust                |
| `57157225` | Jun 15 | perf: replace `list.pop(0)` with `deque.popleft()` in context_tracker LRU                                                      | Python              |
| `a1396ff3` | Jun 16 | fix: CCR regex, loopback exempt, threshold invert, headers passthrough, savings accumulate, rate limit exempt, image auto mode | +235/−75 (9 files)  |
| `e1afeca2` | Jun 16 | fix: CCR backends - in_memory LRU, sqlite spawn_blocking, redis pipeline                                                       | +37/−25 (5 files)   |
| `d4a832cb` | Jun 16 | hardening: poison-tolerant locks, debounced purge, queue compaction                                                            | +213/−36 (2 files)  |
| `e2cca08d` | Jun 16 | test: 8 regression tests for auth classification drift                                                                         | +145/−0             |
| `6f284bb1` | Jun 16 | fix(build): sha2 0.11 hex format - LowerHex removed from Array output                                                          | +15/−13 (5 files)   |
| `b2512027` | Jun 16 | tune: coding‑tuned policy defaults - looser lossy caps, higher volatile thresholds                                             | +174/−192 (2 files) |

### Phase 3: Production Polish (2026-06-17)

| Commit     | Date   | Description                                                                                  | Impact          |
| ---------- | ------ | -------------------------------------------------------------------------------------------- | --------------- |
| `1dc3dae0` | Jun 17 | fix: SQLite overflow clamps + in‑memory evict iteration cap                                  | +6/−7 (2 files) |
| `536ce886` | Jun 17 | **fix: compute_key 24→40 hex chars** - safe for persistent backends with millions of entries | +7/−9           |
| `8823704a` | Jun 16 | ci: bump dtolnay/rust-toolchain                                                              | CI              |
| `483a7681` | Jun 16 | ci: bump codecov/codecov-action 4→7                                                          | CI              |
| `8399a567` | Jun 16 | ci: bump actions/github-script 7→9                                                           | CI              |
| `21fa2554` | Jun 16 | ci: bump aiohttp 3.14.0→3.14.1                                                               | deps            |
| `7df6af0d` | Jun 16 | ci: bump cryptography 46.0.7→48.0.1                                                          | deps            |
| `cd305bfe` | Jun 16 | ci: bump starlette 1.0.1→1.3.1                                                               | deps            |
| `f3a92136` | Jun 16 | ci: bump python-multipart 0.0.27→0.0.31                                                      | deps            |
| `6feb5ead` | Jun 17 | ci: bump pyjwt 2.11.0→2.13.0                                                                 | deps            |
| `8c46f5cd` | Jun 17 | ci: bump vite 8.0.10→8.0.16 (sdk/typescript)                                                 | deps            |
| `d8c91220` | Jun 17 | ci: bump form-data 4.0.5→4.0.6 (sdk/typescript)                                              | deps            |
| `bb1847e9` | Jun 17 | ci: bump vite 8.0.10→8.0.16 (plugins/openclaw)                                               | deps            |
| `fa50e5b6` | Jun 17 | ci: bump js-yaml 4.1.1→4.2.0 (docs)                                                          | deps            |

### Phase 4: Merge / Finalize (2026-06-17–18)

| Commit     | Date   | Description                               |
| ---------- | ------ | ----------------------------------------- |
| `f5d9fb04` | Jun 17 | Merge dependabot PR #7 (python-multipart) |
| `a0fc3a36` | Jun 17 | Merge dependabot PR #6 (starlette)        |
| `1e5c5f31` | Jun 17 | Merge dependabot PR #5 (cryptography)     |
| `378b1bf2` | Jun 17 | Merge dependabot PR #4 (aiohttp)          |
| `b6f2dfd6` | Jun 17 | Merge dependabot PR #3 (github-script)    |
| `474bcd7b` | Jun 17 | Merge dependabot PR #2 (codecov-action)   |
| `00e373ab` | Jun 17 | Merge dependabot PR #1 (rust-toolchain)   |
| `1aea5227` | Jun 17 | Merge dependabot PR #8 (pyjwt)            |
| `aa920a42` | Jun 17 | Merge dependabot PR #12 (js-yaml)         |
| `9613c1d7` | Jun 17 | Merge dependabot PR #10 (form-data)       |
| `1b713c55` | Jun 17 | Merge dependabot PR #11 (vite/openclaw)   |
| `5d607787` | Jun 17 | Merge dependabot PR #9 (vite/sdk)         |
| `eebf943f` | Jun 18 | style: Prettier format pass               |
| `e4767570` | Jun 18 | style: Prettier format pass               |

Three interstitial "save" commits (`702bdfa5`, `6ee1747f`, `c97f135d`, `50644506`) capture work-in-progress states.

---

## What We Deleted 🗑️

77 files removed entirely (~24,000 lines).

### Integration Rip-Outs

| Module                                      | Files   | Lines  | Reason                                                |
| ------------------------------------------- | ------- | ------ | ----------------------------------------------------- |
| `headroom/integrations/langchain/`          | 7 files | ~3,500 | Aphrodite is Hermes‑only - no langchain/orchestration |
| `headroom/integrations/agno/`               | 4 files | ~1,400 | Same                                                  |
| `headroom/integrations/asgi.py`             | 1 file  | 239    | No ASGI/FastAPI wrapping needed                       |
| `headroom/integrations/litellm_callback.py` | 1 file  | 187    | LiteLLM removed entirely                              |

### Removed Subsystems

| File                                     | Lines   | Role                                                                           |
| ---------------------------------------- | ------- | ------------------------------------------------------------------------------ | ---------------------------------------- |
| `headroom/proxy/output_shaper.py`        | 360     | Intelligent output reformatting - removed in favor of Aphrodite's own pipeline |
| `headroom/proxy/output_savings.py`       | 501     | Cost/savings tracking - Aphrodite has its own Prometheus metrics               |
| `headroom/proxy/verbosity_controller.py` | 108     | Verbosity learning - removed                                                   |
| `headroom/proxy/runtime_env.py`          | 151     | Runtime environment detection                                                  |
| `headroom/proxy/cc_switch_reconciler.py` | 192     | Claude Code switch reconciler                                                  |
| `headroom/proxy/handlers/bedrock.py`     | 300     | Python Bedrock handler - moved to Rust (`crates/headroom-proxy/src/bedrock/`)  |
| `headroom/learn/verbosity.py`            | 473     | Verbosity learning models                                                      |
| `headroom/cli/doctor.py`                 | 412     | Diagnostic CLI                                                                 |
| `headroom/cli/update.py`                 | 341     | Auto‑update checker                                                            |
| `headroom/cli/audit.py`                  | 107     | Audit CLI                                                                      |
| `headroom/cli/output_savings.py`         | 61      | Output savings CLI                                                             |
| `headroom/audit/`                        | 4 files | 762                                                                            | Audit modules (reads, codex, maturation) |
| `headroom/cache/backends/sqlite.py`      | 275     | Python SQLite backend - Rust version used instead                              |
| `headroom/update_check.py`               | 303     | Update notification                                                            |
| `headroom/providers/codex/threads.py`    | 97      | Codex thread management                                                        |
| `headroom/providers/mistral_vibe/`       | 2 files | 59                                                                             | Mistral Vibe runtime                     |
| `headroom/perf/analyzer.py`              | 189     | Performance analyzer                                                           |

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

## What We Added ➕

10 files, 2,021 lines.

### Hermes Demo Suite (`examples/hermes_demo/`)

| File                                       | Lines | Purpose                                     |
| ------------------------------------------ | ----- | ------------------------------------------- |
| `hermes_via_proxy_demo.py`                 | 460   | End‑to‑end Aphrodite proxy integration demo |
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

## What We Modified 🔧

~226 files modified across the entire codebase. Key areas:

### Rust Core (`crates/headroom-core/`) - 28 files

| Area                   | Change Summary                                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **CCR hash**           | Length 24→40 hex chars (collision safety at scale). Algorithm remains BLAKE3. Python uses SHA-256 independently. |
| **CCR backends**       | Poison‑tolerant locks, debounced purge, queue compaction; in‑memory LRU, SQLite spawn_blocking, redis pipeline   |
| **SQLite**             | Overflow clamps, evict iteration caps                                                                            |
| **Compression policy** | Coding‑tuned defaults: looser lossy caps, higher volatile thresholds                                             |
| **Relevance**          | Threshold 0.3→0.5, coding stop words added                                                                       |
| **Live zone**          | Major rewrite (149 lines changed) - anthropic + openai + responses compression hooks                             |
| **Smart crusher**      | Compaction classifier, formatter, walker, config, crusher, hashing - all tuned                                   |
| **Search compressor**  | 33 lines changed                                                                                                 |
| **Tokenizers**         | DeepSeek chat/r1/v4/v4‑pro mappings added                                                                        |
| **Build**              | sha2 0.11 hex format fix (LowerHex removed from Array)                                                           |

### Rust Proxy (`crates/headroom-proxy/`) - 13 files

| Area                    | Change Summary                                                                                                                    |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **Bedrock**             | invoke.rs (186 lines changed), eventstream (20), streaming (98) - rewrote Python→Rust                                             |
| **Cache stabilization** | Anthropic cache control, drift detector, openai cache key, volatile detector - all tuned                                          |
| **SSE**                 | Anthropic + OpenAI responses + framing - 38 lines changed                                                                         |
| **Vertex**              | Envelope, raw_predict - 38 lines changed                                                                                          |
| **Headers**             | `x-headroom-workspace` preservation for cross‑project CCR scoping                                                                 |
| **Proxy core**          | 72 lines changed - loopback exempt, threshold invert, headers passthrough, savings accumulate, rate limit exempt, image auto mode |

### Python (`headroom/`) - ~40 files

| Area                    | Change Summary                                                              |
| ----------------------- | --------------------------------------------------------------------------- |
| **`wrap.py`**           | −498 lines: simplified agent wrapping (removed agno, langchain, asgi paths) |
| **`content_router.py`** | −664 lines: simplified routing                                              |
| **`server.py`**         | −198 lines: stripped integration hooks                                      |
| **`proxy_routes.py`**   | −181 lines: simplified                                                      |
| **`install.sh`**        | −98 lines: macOS LaunchAgent rewritten                                      |
| **`dashboard.html`**    | −344 lines: PlayForm branding                                               |
| **`code_handler.py`**   | −109 lines: simplified                                                      |
| **Context tracker**     | `list.pop(0)→deque.popleft()` perf fix                                      |

### Configuration & Build

| File             | Change                                                     |
| ---------------- | ---------------------------------------------------------- |
| `pyproject.toml` | −368 lines: removed liteLLM, langchain, agno deps          |
| `Cargo.toml`     | −54 lines: removed unused features                         |
| `Cargo.lock`     | Deleted entirely (−5,557 lines) - regenerated from scratch |
| `Dockerfile`     | −15 lines                                                  |
| `deny.toml`      | −24 lines                                                  |
| `.gitignore`     | Updated                                                    |
| `uv.lock`        | Removed                                                    |

### CI / Workflows

| File                                       | Change    |
| ------------------------------------------ | --------- |
| `.github/workflows/ci.yml`                 | −16 lines |
| `.github/workflows/docs.yml`               | −37 lines |
| `.github/workflows/install-native-e2e.yml` | −37 lines |
| `scripts/build_rust_extension.sh`          | −12 lines |
| `scripts/install-git-hooks.sh`             | −28 lines |
| `scripts/validate-workflows.sh`            | −32 lines |

### Docs & Release

| File                                  | Change     |
| ------------------------------------- | ---------- |
| `docs/content/docs/releases.mdx`      | −448 lines |
| `docs/content/docs/configuration.mdx` | −18 lines  |
| `docs/content/docs/proxy.mdx`         | −22 lines  |
| `CHANGELOG.md`                        | −66 lines  |
| `README.md`                           | −75 lines  |
| `PR.md`                               | −340 lines |
| `.release-please-manifest.json`       | Updated    |
| `wiki/configuration.md`               | −18 lines  |
| `wiki/troubleshooting.md`             | −17 lines  |

### Tests

| Area                                        | Files | Change                         |
| ------------------------------------------- | ----- | ------------------------------ |
| `tests/test_proxy_dashboard_stats_cache.py` | 1     | −93 lines                      |
| `tests/test_proxy_scalability.py`           | 1     | −30 lines                      |
| `tests/test_cli/test_wrap_copilot.py`       | 1     | −39 lines                      |
| `tests/test_provider_proxy_routes.py`       | 1     | −68 lines                      |
| `tests/test_auth_mode.rs`                   | 1     | +145/−0 (new regression tests) |
| Various                                     | ~20   | Minor adjustments              |

---

## Directory‑Level Impact 📊

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

## Summary 📋

The PlayForm fork transforms Headroom from a **general-purpose LLM proxy** (supporting
langchain, agno, asgi, liteLLM, Claude Code, Copilot, Cursor, aider, etc.) into a
**focused Hermes Agent compression engine** - the core that Aphrodite extends.

| Axis                | Stock Headroom | PlayForm Fork                                                                                          |
| ------------------- | -------------- | ------------------------------------------------------------------------------------------------------ |
| Integration surface | 7+ frameworks  | Hermes only                                                                                            |
| Python lines        | ~35,000        | ~11,000 (net −24,000)                                                                                  |
| Hash algorithm      | BLAKE3         | BLAKE3 (length 24→40 hex). Python side uses SHA-256 — consistent internally, differs across languages. |
| Hash length         | 24 hex chars   | 40 hex chars                                                                                           |
| CCR backends        | Basic          | Poison‑tolerant, debounced, pipelined                                                                  |
| Proxy modes         | Single         | Dual (cache :9797 + token :9798 via TOML)                                                              |
| Compression policy  | Generic        | Coding‑tuned (looser lossy, higher volatile)                                                           |
| Relevance           | 0.3 threshold  | 0.5 + coding stop words                                                                                |
| Tokenizers          | Standard       | + DeepSeek v4 family                                                                                   |
| Bedrock             | Python handler | Rust rewrite                                                                                           |
| Branding            | Stock          | PlayForm identity                                                                                      |
| Test suite          | Full upstream  | Stripped dead code, +8 auth regression tests                                                           |
| CI                  | Full matrix    | Simplified (no langchain/agno/litellm builds)                                                          |

---

## Relationship Diagram 🗺️

```
chopratejas/headroom (upstream)
    │
    └── forked ──→ PlayForm/Headroom (our fork)
                       │
                       ├── main branch (= Source/main) - all fork changes live here
                       │
                       └── Current branch - main + 2 prettier commits
                              │
                              └── tracked as submodule at vendor/headroom/
                                     │
                                     └── Aphrodite crate depends on headroom-core
```

The `Current` branch in this submodule is 2 commits ahead of `Source/main` (two Prettier
format passes). All substantive fork changes are in the 45 commits between the upstream
merge base and the tip of `Source/main`.

---

## Windows Compilation Status 🪟

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
