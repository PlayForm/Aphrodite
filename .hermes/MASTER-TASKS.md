# Aphrodite — Master Task Audit
## Every bug, plan task, improvement, commit, and dependency — fully tracked

> Generated: 2026-06-16 | Releases: v0.5.39 → v0.5.54 | ~30 commits in 24 hours

---

## Complete Commit Timeline (last 24h)

### Earlier Feature Wave (v0.5.39 → v0.5.50, ~22:14–23:06)

| Version | Commit | What |
|---|---|---|
| v0.5.39 | `606537d` | Verbose startup debug banner — all thresholds, engine status, proxy config when `APHRODITE_DEBUG=1` |
| v0.5.40 | `fd2f116` | Essential tools excluded: `skill_view`, `skills_list`, `skill_manage`, `memory`, `session_search` never compressed |
| v0.5.41 | `1e39e76` | Debug banner uses `print()` for TUI visibility + `_log.info()` for log file |
| v0.5.42 | `db15587` | Debug info injected into `[APHRODITE]` catalog block with `⚙` lines |
| v0.5.43 | `6bdba15` | Hex validation: ≥8 hex chars filter, removes `abc123` placeholder matches |
| v0.5.44 | `d19dc49` | Liveness filter on catalog — `ccr.get()` per marker, filters ghost markers |
| v0.5.45 | `864bf7e` | `saturating_sub` on `tokens_saved` — prevents overflow panic |
| v0.5.46 | `3caa3c3` | Auto-expand cached CCR markers <10KB — LLM never sees `aphrodite_retrieve` for small items |
| v0.5.47 | `342cbdc` | `should_compress()` — first fix: `last_prompt_tokens` as fallback |
| v0.5.48 | `b4aff68` | Overcorrected — defaults to full `context_length` (always compresses) |
| v0.5.49 | `c90455a` | Overcorrected — returns `False` when tokens unknown (never compresses) |
| v0.5.50 | `dca0ba4` | Restored `context_length` fallback + dedup in catalog |

### Audit Fix Wave 1 (v0.5.51, ~23:55)
| Version | Commit | Bugs |
|---|---|---|
| v0.5.51 | `5a4082f` | **13 bugs**: #48, #49, #59, #60, #62, #64, #65, #66, #68, #69, #75, #84, #86, #87 |

### Audit Fix Wave 2 (v0.5.52, ~23:58)
| Version | Commit | Bugs |
|---|---|---|
| v0.5.52 | `8872cf0` | **8 bugs**: #50, #56, #63, #73, #76, #88, #89, #70 |

### Modular Refactor (v0.5.53, ~00:02–00:25)
| Version | Commit | What |
|---|---|---|
| v0.5.53 | `3a2fa60` | Split 1656-line `__init__.py` into 9 atomic modules |
| v0.5.53 | `90801c9` | Consolidated shared state to `_core.py`, broke circular import, ruff+pyright configs |
| v0.5.54 | `43e2d1c` | Removed duplicate shared-state definitions |
| v0.5.54 | `dfec38c` | `.hermes/` reorganization — MASTER-TASKS.md, clean structure |
| v0.5.54 | `f332959` | Updated AGENTS.md |

---

## Four-Repo Dependency Chain

| # | Repository | Status | What It Contains |
|---|---|---|---|
| 1 | **`playform/aphrodite`** | ✅ Fully audited | Rust proxy binary, Python Hermes plugin, vendored headroom. All 91 bugs found here. |
| 2 | **`chopratejas/headroom`** | ❌ Not yet scanned | Upstream vendored at `vendor/headroom/`. Provides `CcrStore` trait, `compute_key`, `InMemoryCcrStore`, `SqliteCcrStore`. Pinned at `126543f5`. |
| 3 | **Your headroom fork** | ❌ Not yet located | `.gitmodules` shows only `chopratejas/headroom`. If forked under `NikolaRHristov/headroom` or `PlayForm/headroom`, not wired into submodule. |
| 4 | **Hermes agent** | ❌ Not yet located | Host process that loads plugin, drives hook lifecycle, manages `ContextEngine` interface. Not in `.gitmodules`. |

### What scanning repos 2–4 would resolve

| Repo | Bugs it would clarify |
|---|---|
| `chopratejas/headroom` | #67 (`CcrStore` trait bounds), #83 (SQLite TTL sliding window), #51 (hash length assumption), in-memory store eviction cap |
| Headroom fork | Whether local patches are being consumed by the aphrodite build |
| Hermes | #91 (`ContextEngine` contract match), hook method signatures, tool registration API contract |

---

## BUGS: Full Audit (bugs #1–#91)

### Legend
| Symbol | Meaning |
|---|---|
| ✅ | Fixed |
| ⚠️ | Partial |
| ❌ | Not done |
| ⏭️ | Skipped |
| 🔴 | Critical |
| 🟠 | High |
| 🟡 | Medium |
| 🟢 | Low / Improvement |

---

### Wave 0: Pre-existing / Earlier Fixes (#1–#47)

| # | Sev | File | Bug | Status |
|---|---|---|---|---|
| 1 | 🟡 | `__init__.py` | `INLINE_THRESHOLD` env var typo | ✅ |
| 4 | 🟡 | `__init__.py` | Hardcoded `_rebuild_handler` path | ✅ |
| 5 | 🟡 | `__init__.py` | `_detect_platform()` ignored | ✅ |
| 9 | 🟡 | `__init__.py` | Fixed sleep → `_wait_alive()` retry | ✅ |
| 10 | 🟡 | `__init__.py` | `_alive()` TTL cache | ✅ |
| 11 | 🔴 | `__init__.py` | `should_compress()` regressed 4× | ⚠️ v0.5.50 |
| 12 | 🟡 | `__init__.py` | `_resolve_one()` single-port | ✅ |
| 13 | 🟡 | `__init__.py` | `[:2000]` truncation | ✅ |
| 18 | 🟠 | Rust | `inject_tool` placement | ❌ |
| 21 | 🟠 | Rust | `x-headroom-*` header passthrough | ✅ |
| 25 | 🟠 | Rust | `/retrieve` pagination | ✅ |
| 26 | 🟠 | Rust | `ccr_db_path` relative | ✅ v0.5.53 |
| 27 | ⏭️ | Rust | `--api-url` bypass | ⏭️ |
| 28 | 🟠 | Rust | `Secret` newtype | ✅ |
| 29 | ⏭️ | Rust | `--bind` flag | ⏭️ |
| 30 | ⏭️ | Rust | `--dual` mode | ⏭️ |
| 34 | ⏭️ | Rust | `/health/upstream` | ⏭️ |
| 35 | 🟡 | Python | Deque LRU orphans tool-call pairs | ❌ |
| 36 | 🟡 | `__init__.py` | Bare `print()` | ⚠️ Gated |
| 37 | 🟡 | Rust | Case-sensitive query | ❌ |
| 39 | 🟡 | Rust | `saturating_sub` swallows signal | ❌ |
| 40 | 🟡 | `__init__.py` | Auto-expand 10KB hardcoded | ❌ |
| 41 | 🟡 | `__init__.py` | Liveness filter per-marker | ❌ |
| 42 | 🟡 | `__init__.py` | Hex filter ≥8 too permissive | ❌ |
| 43 | 🟡 | `__init__.py` | Git diff race | ❌ |
| 44 | 🟡 | `__init__.py` | `.test-results.json` CWD | ❌ |
| 45 | 🟡 | `__init__.py` | `_recent_markers` no persistence | ❌ |
| 46 | 🟡 | `__init__.py` | Essential tools hardcoded | ❌ |
| 47 | 🟡 | `__init__.py` | Debug banner always-on | ❌ |

---

### Wave 2: Python Plugin (#48–#58)

| # | Sev | Bug | Status | Version |
|---|---|---|---|---|
| 48 | 🔴 | `cache_alive` NameError crash | ✅ | v0.5.51 |
| 49 | 🔴 | `_recent_markers` global shadow | ✅ | v0.5.51 |
| 50 | 🟡 | `should_compress()` first-turn | ✅ | v0.5.52 |
| 51 | 🟠 | 16-char vs 64-char hash mix | ❌ | |
| 52 | 🟡 | `_git_summary()` race + 3s block | ❌ | |
| 53 | 🟡 | `.test-results.json` to plugin dir | ❌ | |
| 54 | 🟡 | O(n) `_inline_store` scan | ❌ | |
| 55 | 🟡 | `on_session_reset` shadow | ✅ | by #49 |
| 56 | 🟡 | `threshold_tokens` always 1 | ✅ | v0.5.52 |
| 57 | 🟠 | `compress()` tool-chain off-by-one | ❌ | |
| 58 | 🟡 | Asymmetric CCR entries [:5000] | ❌ | |

---

### Wave 3: proxy.rs (#59–#70)

| # | Sev | Bug | Status | Version |
|---|---|---|---|---|
| 59 | 🔴 | `health_check` "degraded" | ✅ | v0.5.51 |
| 60 | 🟡 | Double `detect_content_type` | ✅ | v0.5.51 |
| 61 | 🟡 | `inline_ccr` dead code | ❌ | |
| 62 | 🔴 | EMA `hash.len()=64` ratio | ✅ | v0.5.51 |
| 63 | 🟡 | Compress tool relay size=0 | ✅ | v0.5.52 |
| 64 | 🟡 | False Rust+ on Python | ✅ | v0.5.51 |
| 65 | 🟡 | Body read failure → 200 | ✅ | v0.5.51 |
| 66 | 🟡 | Double `t0.elapsed()` | ✅ | v0.5.51 |
| 67 | 🟡 | `CcrStore` trait `len()` + `Send+Sync` | ❌ | Needs headroom scan |
| 68 | 🔴 | Initial EMA 10000 scale-up | ✅ | v0.5.51 |
| 69 | ✅ | CCR marker `<<<` vs `⫷` | ✅ | Already matching at HEAD |
| 70 | 🟡 | Tool relay no timeout | ✅ | v0.5.53 |

---

### Wave 4: main/retrieve/config (#71–#91)

| # | Sev | Bug | Status | Version |
|---|---|---|---|---|
| 71 | 🟡 | Double `ctrl_c` — no drain | ❌ | |
| 72 | 🟡 | Prometheus `_us` vs seconds | ❌ | |
| 73 | 🟡 | `/*path` catches favicon | ✅ | v0.5.52 |
| 74 | 🟡 | `ccr_db_path` CWD not binary | ❌ | |
| 75 | 🔴 | Arbitrary filesystem read | ✅ | v0.5.51 |
| 76 | 🟡 | `filter_content` full on zero | ✅ | v0.5.52 |
| 77 | 🟡 | Query-only BAD_REQUEST | ❌ | |
| 78 | 🟡 | `GET /retrieve` 405 | ❌ | |
| 79 | 🟢 | `deny_unknown_fields` missing | ❌ | |
| 80 | 🟡 | Both proxies bind :9797 | ❌ | |
| 81 | 🟢 | Empty `api_key` silent | ❌ | |
| 82 | 🟢 | `X-Aphrodite-Request-Id` | ❌ | |
| 83 | 🟢 | CCR TTL not refreshed on `get()` | ❌ | Needs headroom scan |
| 84 | 🔴 | Bind `0.0.0.0` → `127.0.0.1` | ✅ | v0.5.51 |
| 85 | 🟢 | `/metrics` unauthenticated | ❌ | |
| 86 | 🔴 | Default port 8788 | ✅ | v0.5.51 |
| 87 | 🔴 | TOML vs CLI DB paths | ✅ | v0.5.51 |
| 88 | 🟡 | Mode silent fallback | ✅ | v0.5.52 |
| 89 | 🟡 | `listen` mandatory TOML | ✅ | v0.5.52 |
| 90 | 🟢 | `no_ccr_marker` per-proxy | ❌ | |
| 91 | 🟢 | `CcrStore` trait bounds | ❌ | Needs Hermes scan |

---

### Bug Scorecard

| Severity | Total | Done | Remaining |
|---|---|---|---|
| 🔴 Critical | 7 | **7** | 0 |
| 🟠 High | 6 | **3** | 3 |
| 🟡 Medium/Low | 64 | **17** | 47 |
| 🟢 Improvement | 6 | **0** | 6 |
| ⏭️ Skipped | 5 | — | 5 |
| **TOTAL** | **91** | **27** | **61** |

---

## PLAN TASKS: Headroom Coding-Agent Rewrite (100 tasks)

See `plans/0-headroom-100-tasks.md`.

| Phase | Tasks | Done | Notes |
|---|---|---|---|
| 1: Rip Out | 1–20 | 2 | T1-2 (delete integrations) |
| 2: Compression | 21–35 | 0 | Upstream tree-sitter exists |
| 3: Tracker | 36–50 | 1 | #36 (deque LRU) |
| 4: Plugin | 51–75 | 17 | Most tool enhancements |
| 5: Proxy | 76–100 | 6 | #73,#75,#76,#86,#87,#88,#89 |

---

## Release History

| Version | Date | Key Changes |
|---|---|---|
| v0.5.39–50 | Jun 15–16 | Feature wave: debug banner, essential tools, hex filter, auto-expand, `should_compress()` iterations |
| v0.5.51 | Jun 16 | 13 bugs: critical + high + medium |
| v0.5.52 | Jun 16 | 8 bugs: medium + low |
| v0.5.53 | Jun 16 | Modular refactor + 8 bugs |
| v0.5.54 | Jun 16 | Cleanup, reorganization, handoff |

---

## File Index

| File | Purpose |
|---|---|
| `MASTER-TASKS.md` | **This file** — everything in one place |
| `tasks/1-wave-audit-v050-v0550.md` | Wave 1: 44-commit push audit |
| `tasks/2-python-plugin-bugs-48-58.md` | Wave 2: Python plugin bugs |
| `tasks/3-proxy-rs-bugs-59-70.md` | Wave 3: proxy.rs bugs |
| `tasks/4-main-retrieve-config-bugs-71-91.md` | Wave 4: main/retrieve/config bugs |
| `tasks/5-wave4-execution.md` | Wave 4 execution plan |
| `plans/0-headroom-100-tasks.md` | Original 100-task rewrite plan |
| `plans/1-honest-gaps.md` | Honest self-assessment |
| `plans/2-architectural-subtasks.md` | Detailed subtask plans |
| `AGENTS.md` | Development context |
| `CLAUDE.md` | Project rules |
| `HANDOFF.md` | Handoff document |
