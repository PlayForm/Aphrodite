# Aphrodite — Consolidated Task List
## All known bugs, gaps, improvements — merged from 4 deep-scans + structural analysis
### v1.62.1 | June 16, 2026

---

## 🔴 Critical (Fix First — Blocks Correctness)

| # | File:Line | Issue | Fix |
|---|---|---|---|
| C1 | `_core.py:10` vs `Cargo.toml:3` | **BIN_VERSION mismatch**: Python says `v0.5.54`, Cargo.toml is `0.5.52`. Also: `v` prefix in Python but not in TOML. Download URL resolves to non-existent release. | Bump both to `0.5.55`, strip `v` prefix from download URL, or add `v` to Cargo.toml. |
| C2 | `run-headroom-proxy.py` + `_core.py:8` | **Port mismatch**: script uses `:9799/:9800`, plugin uses `:9797/:9798` (`PORTS` dict). Health checks fail. | Unify to `:9799/:9800` everywhere or back to `:9797/:9798`. |
| C3 | `_engine.py` `on_session_reset` | Assigns `_turn_counter = 0` as module-local, never resets real `_turn_counter` in `_core.py`. | Import and write through module reference: `import aphrodite._core as _c; _c._turn_counter = 0` |
| C4 | `_engine.py` `compress()` | Engine marker `[CONTEXT COMPRESSED: ... CCR:{hash}]` invisible to `_CCR_RE` (`<<<CCR:...>>>`). Orphaned compressions. | Use `_ccr_marker()` from `_marker.py`. |
| C5 | `_resolve.py:27-29` | `_resolve_one` returns error JSON as content on non-200. LLM sees `{"error":"not found"}` as tool output. | Check `r.status != 200` → return `None`. |
| C6 | `_engine.py` `update_model` | Sets `threshold_tokens = 1`, triggers immediate compression on first response. | Calculate: `int(context_length * threshold_percent / 100)` |

## 🟠 High (Causes Data Loss / Wrong Behavior)

| # | File:Line | Issue | Fix |
|---|---|---|---|
| H1 | `_hooks.py` `_pre_llm_hook` | `_recent_markers.clear()` wipes freshly compressed tool results before `aphrodite_search`. | Merge: keep existing markers not in new scan, cap at 200. |
| H2 | `_proxy.py:65` | Port hardcoded as `f"127.0.0.1:{port}"` — only starts token mode. Cache proxy never launched by plugin. | Launch both modes from `PORTS` dict. |
| H3 | `_proxy.py:68-73` | Proxy stdout/stderr directed to `DEVNULL` (good) but was previously inherited → noisy. | ✅ Already fixed in current code (DEVNULL). |
| H4 | `_hooks.py` turn archive | Recompresses on every turn if LLM call fails (no sentinel). | Store sentinel in `_conv_index` after successful compression. |
| H5 | `_hooks.py` `_pre_llm_hook` return | Return value may overwrite `user_message`. | Append catalog to user_message, don't replace. |
| H6 | `_engine.py` `should_compress` | Returns `False` when `threshold == 0` (should mean "always"). | Align semantics with comments. Use `-1` for always, `0` for disabled. |

## 🟡 Medium (Performance / UX / Reliability)

| # | File:Line | Issue | Fix |
|---|---|---|---|
| M1 | `_core.py:36` | `CATALOG_MODE` default `"compact"` — ✅ already fixed. Was `"full"`. | — |
| M2 | `_tools.py:95` | `RETRIEVE_SCHEMA` hash field: no hint about extracting hash from `<<<CCR:hash\|...>>>`. | Add description + defensive `hash_val.split("\|")[0]`. |
| M3 | `_hooks.py` `_extract_preview` | O(messages × markers) per turn. | Pre-build `hash→preview` cache once per hook. |
| M4 | `_hooks.py` `_group_into_turns` | Discards tool content with `pass`. Turn summaries useless. | Store first 200 chars of tool content. |
| M5 | `_hooks.py` `_search_handler` | Returns duplicates across `_conv_index`, `_inline_store`, `_recent_markers`. | Deduplicate by hash. |
| M6 | `_hooks.py` `_test_handler` | Round-trip test never compresses first, always returns "not found" but passes vacuously. | Compress before retrieve in test. |
| M7 | `_hooks.py` | Missing `__all__` — internal helpers leak as public symbols. | Add `__all__` list with only public API. |
| M8 | `_tools.py:41-43` | `_compress_handler` accepts empty content (but now validates — ✅ check added). | Already validates `if not content:`. |
| M9 | `_resolve.py:38-60` | `_resolve_recursive` uses `str.replace` (all occurrences) instead of per-match. With cycle detection present but `content.replace()` is global. | Use `re.sub` with `count=1` per match. |
| M10 | `_core.py:59` | `_CCR_RE = r"<<<CCR:([^>]+)>>>"` permits pathological backtracking on large outputs. | Add `{1,100}` length limit: `([^>]{1,100})`. |
| M11 | `run-headroom-proxy.py` | CCR fragmentation at >1 worker has no runtime warning. | Add assertion or print `[WARN]`. |
| M12 | `setup-headroom-providers.py` | Both providers use same key name — ambiguous. | Document explicitly in comments. |
| M13 | `START.md` / bootstrap | Missing port verification step. | Add `lsof` check section. |
| M14 | `aphrodite.toml.example` | Missing `cache_port` field. | Add `cache_port = 9798` explicitly. |
| M15 | `_marker.py:48` | `_parse_ccr_markers` silently drops malformed markers (`except: pass`). | Log warning on parse failure. |
| M16 | `_resolve.py` | No `_resolve_recursive` called from `_resolve_one` — the `_CCR_RE.findall` + replacement logic is in `_resolve_recursive`, not in `_resolve_one`. Content with nested markers retrieved from proxy won't be unpacked unless `_resolve_recursive` is used. This is correct behavior (caller uses recursive), but the function names are confusing. | Rename or add docstring clarity. |

## 🟢 Low / Improvement

| # | File:Line | Issue | Fix |
|---|---|---|---|
| L1 | `_inline.py:13` | 16-char inline hashes visually collide with proxy hashes. | Use `"i:"` prefix: `"i:" + hex[:14]`. |
| L2 | `plugin.yaml` | No `min_hermes_version` or `requires_hooks`. | Add version guard. |
| L3 | `_marker.py:9-17` | Preview text outside marker → lost on truncation. | Embed `\|preview_b64` inside marker. |
| L4 | `_proxy.py` | No stale PID cleanup on startup. | Check and kill old process before launch. |
| L5 | `_proxy.py` | No lock file — double-spawn from multiple windows. | Check port-in-use before launch. |
| L6 | `_proxy.py:37-55` | `_alive()` opens new TCP per call (but has 5s TTL cache). 12 `pass` statements across codebase are bare except blocks swallowing errors. | Use `http.client.HTTPConnection` keep-alive. Replace bare `pass` with logging. |
| L7 | `_proxy.py:86` | `on_start` only launches token mode: `for name in ("token",):` | Launch both: `for name in PORTS:`. |
| L8 | `proxy.rs` | `tokens_saved` counter never incremented. Stats always show 0. | Increment in `ccr_create_handler` / `compress`. |
| L9 | `generate-soul.py` | Uses `os.path.abspath(".")` — breaks on different machines. | Use `pathlib.Path(__file__).parent.parent`. |
| L10 | `dev-dual.sh` | `RUST_LOG=info` floods with framework noise. | Use `RUST_LOG=aphrodite=info,tower_http=warn`. |
| L11 | `main.rs:118-137` | `/metrics` endpoint unauthenticated. | Add auth check or document as intentional. |
| L12 | `main.rs:34` | TOML path hardcoded to `aphrodite.toml` (CWD). Breaks if run from different dir. | Use args or env var for config path. |
| L13 | `config.rs:152` | `dirs::data_dir().unwrap_or_else(\|\| PathBuf::from("/tmp"))` — unwrap on potentially None. | Handle gracefully. |
| L14 | `config.rs:146` | API key resolution chain: TOML → `APHRODITE_API_KEY` → `DEEPSEEK_API_KEY` → empty. Missing `HEADROOM_DEEPSEEK_KEY` fallback. | Add `HEADROOM_DEEPSEEK_KEY` to chain. |
| L15 | `proxy.rs` | `request_history` lock: `.unwrap_or_default()` on lock poison. | Document why unwrap is safe or handle poison. |

## 📋 Infrastructure / Non-Code

| # | Item | Status |
|---|---|---|
| I1 | `Cargo.lock` has 112KB of pinned deps — audit for outdated/vulnerable | ❌ |
| I2 | No `rust-toolchain.toml` — build breaks on wrong Rust version | ❌ |
| I3 | No CI/CD (GitHub Actions) — no automated build/test/lint | ❌ |
| I4 | No `CHANGELOG.md` — release history only in MASTER-TASKS.md | ❌ |
| I5 | Vendor `headroom` at pinned commit `126543f5` — never audited upstream | ❌ |
| I6 | `hermes_demo/` and `hermes_mcp_client.py` bugs — in different repo | ⏭️ |

## 📊 Stats

| Category | Count | Done |
|---|---|---|
| 🔴 Critical | 6 | 0 |
| 🟠 High | 6 | 1 (H3) |
| 🟡 Medium | 16 | 2 (M1, M8) |
| 🟢 Low | 15 | 0 |
| 📋 Infra | 6 | 0 |
| **Total** | **49** | **3** |
