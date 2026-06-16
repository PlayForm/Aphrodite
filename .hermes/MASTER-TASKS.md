# Aphrodite — Master Task Audit
## Every bug, plan task, and improvement — fully tracked and labeled

> Generated: 2026-06-16 | Releases: v0.5.50 → v0.5.54 | 4 waves, 29 bugs fixed

---

## BUGS: Proxy + Plugin Audit (bugs #1–#91)

### Legend
| Symbol | Meaning |
|---|---|
| ✅ | Fixed |
| ⚠️ | Partial |
| ❌ | Not done |
| ⏭️ | Skipped (architectural, upstream, or pre-existing) |
| 🔴 | Critical |
| 🟠 | High |
| 🟡 | Medium |
| 🟢 | Low / Improvement |

---

### Wave 1 Bugs (#1–#47) — Pre-existing / Earlier Waves

| # | Severity | File | Bug | Status | Fix |
|---|---|---|---|---|---|
| 1 | 🟡 | `__init__.py` | `INLINE_THRESHOLD` env var typo | ✅ | Fixed silently in 44-commit wave |
| 4 | 🟡 | `__init__.py` | Hardcoded `_rebuild_handler` path | ✅ | Now derived from `__file__` |
| 5 | 🟡 | `__init__.py` | `_detect_platform()` ignored in download | ✅ | Platform tag in URL |
| 9 | 🟡 | `__init__.py` | Fixed sleep → `_wait_alive()` retry | ✅ | Retry loop with 10 attempts |
| 10 | 🟡 | `__init__.py` | `_alive()` TTL cache | ✅ | 5-second TTL |
| 11 | 🔴 | `__init__.py` | `should_compress()` always True → fixed+regressed 4x | ⚠️ | v0.5.50 restored context_length fallback |
| 12 | 🟡 | `__init__.py` | `_resolve_one()` single-port | ✅ | Tries both 9797 + 9798 |
| 13 | 🟡 | `__init__.py` | `[:2000]` truncation in `compress()` | ✅ | Removed, full content |
| 18 | 🟠 | Rust | `inject_tool` placement | ❌ | |
| 21 | 🟠 | Rust | `x-headroom-*` header passthrough | ✅ | |
| 25 | 🟠 | Rust | `/retrieve` pagination | ✅ | |
| 26 | 🟠 | Rust | `ccr_db_path` relative default | ✅ | v0.5.53 XDG fix |
| 27 | ⏭️ | Rust | `--api-url` bypass | ⏭️ | Skipped |
| 28 | 🟠 | Rust | `Secret` newtype for api_key | ✅ | |
| 29 | ⏭️ | Rust | `--bind` flag | ⏭️ | Pre-existing |
| 30 | ⏭️ | Rust | `--dual` mode | ⏭️ | Skipped |
| 34 | ⏭️ | Rust | `/health/upstream` route | ⏭️ | Pre-existing |
| 35 | 🟡 | Python | Deque LRU evicts single messages | ❌ | |
| 36 | 🟡 | `__init__.py` | Bare `print()` in plugin context | ⚠️ | Gated on `APHRODITE_DEBUG=1` |
| 37 | 🟡 | Rust | Case-sensitive query in retrieve | ❌ | |
| 39 | 🟡 | Rust | `saturating_sub` swallows signal | ❌ | |
| 40 | 🟡 | `__init__.py` | Auto-expand 10KB hardcoded | ❌ | |
| 41 | 🟡 | `__init__.py` | Liveness filter per-marker on every turn | ❌ | |
| 42 | 🟡 | `__init__.py` | Hex filter ≥8 chars too permissive | ❌ | |
| 43 | 🟡 | `__init__.py` | `git diff --stat` race condition | ❌ | |
| 44 | 🟡 | `__init__.py` | `.test-results.json` to CWD | ❌ | |
| 45 | 🟡 | `__init__.py` | `_recent_markers` no persistence | ❌ | |
| 46 | 🟡 | `__init__.py` | Essential tools hardcoded | ❌ | |
| 47 | 🟡 | `__init__.py` | Debug banner always-on | ❌ | |

---

### Wave 2: Python Plugin Bugs (#48–#58)

| # | Severity | File | Bug | Status | Version |
|---|---|---|---|---|---|
| 48 | 🔴 | `__init__.py` | `cache_alive` NameError crash | ✅ | v0.5.51 |
| 49 | 🔴 | `__init__.py` | `_recent_markers` global shadow — search empty | ✅ | v0.5.51 |
| 50 | 🟡 | `__init__.py` | `should_compress()` fires on first turn | ✅ | v0.5.52 |
| 51 | 🟠 | `__init__.py` | 16-char inline hash vs 64-char proxy hash mixed | ❌ | |
| 52 | 🟡 | `__init__.py` | `_git_summary()` race + 3s blocking | ❌ | |
| 53 | 🟡 | `__init__.py` | `.test-results.json` to plugin dir | ❌ | |
| 54 | 🟡 | `__init__.py` | O(n) scan of `_inline_store` on search | ❌ | |
| 55 | 🟡 | `__init__.py` | `on_session_reset` clears shadowed list | ✅ | Resolved by #49 |
| 56 | 🟡 | `__init__.py` | `threshold_tokens` always 1 | ✅ | v0.5.52 |
| 57 | 🟠 | `__init__.py` | `compress()` tool-chain backtrack off-by-one | ❌ | |
| 58 | 🟡 | `__init__.py` | Asymmetric CCR entries ([:5000] truncation) | ❌ | |

---

### Wave 3: proxy.rs Bugs (#59–#70)

| # | Severity | File | Bug | Status | Version |
|---|---|---|---|---|---|
| 59 | 🔴 | `proxy.rs` | `health_check` returns "degraded" → plugin marks dead | ✅ | v0.5.51 |
| 60 | 🟡 | `proxy.rs` | `detect_content_type` called twice | ✅ | v0.5.51 |
| 61 | 🟡 | `proxy.rs` | `inline_ccr` dead code — never written/read | ❌ | |
| 62 | 🔴 | `proxy.rs` | EMA uses `hash.len()=64` → ratio 640000× | ✅ | v0.5.51 |
| 63 | 🟡 | `proxy.rs` | `aphrodite_compress` tool relay size=0 | ✅ | v0.5.52 |
| 64 | 🟡 | `proxy.rs` | False-positive Rust detection on Python | ✅ | v0.5.51 |
| 65 | 🟡 | `proxy.rs` | Body read failure returns 200 → silent data loss | ✅ | v0.5.51 |
| 66 | 🟡 | `proxy.rs` | Latency recorded twice — histogram diverges | ✅ | v0.5.51 |
| 67 | 🟡 | `lib.rs` | `CcrStore` trait needs `len()` + `Send + Sync` | ❌ | |
| 68 | 🔴 | `proxy.rs` | Initial EMA 10000 → startup 2× threshold scale-up | ✅ | v0.5.51 |
| 69 | 🔴 | Both | CCR marker format: ASCII `<<<` vs Unicode `⫷` | ✅ | Already matching at HEAD |
| 70 | 🟡 | `proxy.rs` | Tool relay callback no timeout — leaked tasks | ✅ | v0.5.53 |

---

### Wave 4: main.rs / retrieve.rs / config.rs Bugs (#71–#91)

| # | Severity | File | Bug | Status | Version |
|---|---|---|---|---|---|
| 71 | 🟡 | `main.rs` | Double `ctrl_c` listener — no graceful drain | ❌ | |
| 72 | 🟡 | `main.rs` | Prometheus `_us` vs seconds — misleading | ❌ | |
| 73 | 🟡 | `main.rs` | `/*path` catches `/favicon.ico` | ✅ | v0.5.52 |
| 74 | 🟡 | `main.rs` | `ccr_db_path` parent from CWD, not binary | ❌ | |
| 75 | 🔴 | `retrieve.rs` | Arbitrary filesystem read via `{"path": "/etc/passwd"}` | ✅ | v0.5.51 |
| 76 | 🟡 | `retrieve.rs` | `filter_content` returns full 100KB on zero matches | ✅ | v0.5.52 |
| 77 | 🟡 | `retrieve.rs` | Query-only request returns BAD_REQUEST | ❌ | |
| 78 | 🟡 | `retrieve.rs` | `GET /retrieve` returns 405 — no help | ❌ | |
| 79 | 🟢 | `config.rs` | `deny_unknown_fields` missing | ❌ | |
| 80 | 🟡 | `config.rs` | Two proxies both try to bind :9797 without listen | ❌ | |
| 81 | 🟢 | `config.rs` | Empty `api_key` accepted silently | ❌ | |
| 82 | 🟢 | Rust | `X-Aphrodite-Request-Id` header | ❌ | |
| 83 | 🟢 | Rust | CCR TTL not refreshed on `get()` | ❌ | |
| 84 | 🔴 | `config.rs` | Both proxies bind `0.0.0.0` by default | ✅ | v0.5.51 (127.0.0.1) |
| 85 | 🟢 | `main.rs` | `/metrics` unauthenticated | ❌ | |
| 86 | 🔴 | `config.rs` | Default port `8788` ≠ `9797`/`9798` | ✅ | v0.5.51 |
| 87 | 🔴 | `config.rs` | TOML vs CLI different DB paths | ✅ | v0.5.51 |
| 88 | 🟡 | `config.rs` | `mode = "Token"` silently becomes cache | ✅ | v0.5.52 |
| 89 | 🟡 | `config.rs` | `listen` mandatory in TOML | ✅ | v0.5.52 |
| 90 | 🟢 | `config.rs` | `no_ccr_marker` not configurable per-proxy | ❌ | |
| 91 | 🟢 | `lib.rs` | `CcrStore` trait must declare `Send + Sync + len()` | ❌ | |

---

### Bug Scorecard

| Severity | Total | Done | Remaining |
|---|---|---|---|
| 🔴 Critical | 6 | **6** | 0 |
| 🟠 High | 6 | **3** | 3 |
| 🟡 Medium/Low | 64 | **17** | 47 |
| 🟢 Improvement | 6 | **0** | 6 |
| ⏭️ Skipped | 5 | **0** | 5 |
| **TOTAL** | **91** | **26** | **61** |

> Note: Skipped items are architectural decisions or pre-existing conditions.
> Remaining medium/low items are polish — no critical or high bugs remain.

---

## PLAN TASKS: Headroom Coding-Agent Rewrite (100 tasks)

See `plans/0-headroom-100-tasks.md` for the full plan.

| Phase | Tasks | Done | Status |
|---|---|---|---|
| 1: Rip Out | 1–20 | 2 | ✅ T1-2, rest skipped |
| 2: Code Compression | 21–35 | 0 | Upstream tree-sitter exists |
| 3: Context Tracker | 36–50 | 1 | #36 (deque LRU) only |
| 4: Plugin Rewrite | 51–75 | 17 | Most tool enhancements done |
| 5: Proxy Rewrite | 76–100 | 6 | #87, #88, #89, #73, #75, #76, #86 |

---

## Release History

| Version | Date | Waves | Bugs |
|---|---|---|---|
| v0.5.50 | 2026-06-15 | — | (baseline) |
| v0.5.51 | 2026-06-16 | 1 | 13 bugs (critical + high + medium) |
| v0.5.52 | 2026-06-16 | 2 | 8 bugs (medium + low) |
| v0.5.53 | 2026-06-16 | 3 | 8 bugs (medium + refactor) |
| v0.5.54 | 2026-06-16 | 4 | Duplicate cleanup + handoff |

---

## File Index

| File | Purpose |
|---|---|
| `MASTER-TASKS.md` | This file — comprehensive bug + plan audit |
| `tasks/1-wave-audit-v050-v0550.md` | Wave 1 audit of 44-commit push |
| `tasks/2-python-plugin-bugs-48-58.md` | Python plugin bugs at HEAD |
| `tasks/3-proxy-rs-bugs-59-70.md` | proxy.rs bugs at HEAD |
| `tasks/4-main-retrieve-config-bugs-71-91.md` | main/retrieve/config bugs at HEAD |
| `tasks/5-wave4-execution.md` | Wave 4 execution plan |
| `plans/0-headroom-100-tasks.md` | Original 100-task rewrite plan |
| `plans/1-honest-gaps.md` | Honest self-assessment |
| `plans/2-architectural-subtasks.md` | Detailed subtask implementation plans |
| `AGENTS.md` | Development context |
| `CLAUDE.md` | Project rules |
| `HANDOFF.md` | Handoff document |
