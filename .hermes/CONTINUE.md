# Aphrodite — Session Continuation Prompt
## June 16, 2026 | v1.62.1 | Proxy: headroom-cache :9799

You are resuming aphrodite development. Use the **headroom-cache** provider (127.0.0.1:9799, HEADROOM_DEEPSEEK_KEY) — the aphrodite proxy uses the wrong key. Load skills `plan-then-delegate`, `execution-blocks`, `aphrodite-dev-workflow`. Work async: write plan scripts to /tmp, fire via terminal(background=true, notify_on_complete=true), continue talking while they run.

---

## ✅ Already Fixed (this session)

| Version | Bug | What |
|---|---|---|
| v1.62.1 | #57 | tool-chain backtrack off-by-one in compress() |
| v1.62.1 | #58 | asymmetric CCR truncation [:5000]/[:200] removed |
| v1.62.1 | — | profile skills: barebone/compress-off exclude aphrodite-dev-workflow |
| v1.62.1 | — | proxy-cache: compression disabled (cache-only, matches --no-ccr-marker) |
| v1.62.1 | — | dev-dual.sh: trap 'kill 0' for child cleanup |
| v1.62.1 | — | generate-soul.py: --dry-run + --force + dirty file check |
| v1.62.1 | — | aphrodite.toml.example: comment that api_key reads from env var |
| v1.62.1 | — | gitignore: sessions/, cron/, snapshots, cache, logs in profiles/ |
| v1.62.1 | — | pyproject.toml: full dev deps (ruff, pyright, pytest, mypy, black, etc.) |
| v1.62.1 | — | pyrightconfig.json: strict mode enabled |

---

## 🔴 Critical — Fix First

| # | File | Issue |
|---|---|---|
| C1 | `scripts/run-headroom-proxy.py` + plugin | **Port mismatch**: script uses :9799/:9800, plugin expects :9797/:9798. Unify to :9799/:9800 or :9797/:9798 everywhere. |
| C2 | `_engine.py` | `on_session_reset` assigns to module-local `_turn_counter`, never resets the real one in `_core.py`. Must write through module reference. |
| C3 | `_engine.py` | Engine marker format `[CONTEXT COMPRESSED: ... CCR:{hash}]` invisible to `_CCR_RE` (`<<<CCR:...>>>`). Orphaned compressions. Use `_ccr_marker()`. |
| C4 | `_resolve.py` | `_resolve_one` returns error JSON as content on non-200. Must check `r.status != 200` and return None. |
| C5 | `_engine.py` | `update_model` sets `threshold_tokens = 1`, triggers immediate compression on first response. Should calculate from `context_length * threshold_percent / 100`. |

## 🟠 High

| # | File | Issue |
|---|---|---|
| H1 | `_hooks.py` | `_recent_markers.clear()` wipes freshly compressed tool results before `aphrodite_search` can find them. Merge instead of replace. |
| H2 | `_proxy.py` | Port numbers hardcoded in `subprocess.Popen` args (strings "9797", "9798"), bypass `PORTS` dict. Use `str(PORTS["token"])`. |
| H3 | `_proxy.py` | Proxy stdout/stderr inherited → pollutes Hermes terminal. Redirect to log file. |
| H4 | `_hooks.py` | Turn archive recompresses on every turn if LLM call fails (no sentinel in `_conv_index`). Store sentinel after successful compression. |
| H5 | `_hooks.py` | `_pre_llm_hook` return value may overwrite user_message. Should append catalog to user_message, not replace. |
| H6 | `_engine.py` | `should_compress` returns False when threshold == 0 (should be "always"). Align semantics with _core.py. |

## 🟡 Medium

| # | File | Issue |
|---|---|---|
| M1 | `_core.py` | `CATALOG_MODE` defaults to "full" — burns 800–1200 tokens/turn. Change default to "compact". |
| M2 | `_tools.py` | `RETRIEVE_SCHEMA` gives no hint about extracting hash from `<<<CCR:hash|...>>>`. Add format description. |
| M3 | `_hooks.py` | `_extract_preview` is O(messages × markers). Pre-build hash→preview cache once per hook. |
| M4 | `_hooks.py` | `_group_into_turns` discards tool content with `pass`. Store tool summaries for useful turn archives. |
| M5 | `_hooks.py` | `_search_handler` returns duplicates across `_conv_index`, `_inline_store`, `_recent_markers`. Deduplicate. |
| M6 | `_hooks.py` | `_test_handler` round-trip test broken — never compresses before retrieving, always returns "not found" but passes vacuously. |
| M7 | `_hooks.py` | Missing `__all__` — internal helpers leak as public symbols. |
| M8 | `_tools.py` | `_compress_handler` accepts empty content, stores empty CCR entries. Validate non-empty. |
| M9 | `_resolve.py` | `_resolve_recursive` uses `str.replace` (all occurrences) instead of per-match. Add cycle detection. |
| M10 | `_core.py` | `_CCR_RE` pattern permits pathological backtracking on large outputs. Add `{1,100}` length limit. |
| M11 | `scripts/setup-headroom-providers.py` | Provider key ambiguity — both use APHRODITE_API_KEY. Document explicitly. |
| M12 | `headroom` | CCR fragmentation at >1 worker has no runtime warning. Add assertion or print warning. |
| M13 | `START.md` | Missing port verification step before bootstrap. Add lsof check. |
| M14 | `aphrodite.toml.example` | Missing `cache_port` field — silent fallback may conflict. |

## 🟢 Low / Improvement

| # | File | Issue |
|---|---|---|
| L1 | `_inline.py` | 16-char inline hashes collide with proxy hashes visually. Use `"i:"` prefix. |
| L2 | `plugin.yaml` | No `min_hermes_version` constraint or `requires_hooks` declaration. |
| L3 | `_marker.py` | Preview is outside marker string — lost on truncation. Embed preview_b64 inside marker. |
| L4 | `_proxy.py` | No stale PID cleanup on startup — second instance competes with old one. |
| L5 | `_proxy.py` | No lock file prevents double-spawn from multiple Hermes windows. |
| L6 | `_proxy.py` | `_alive()` opens new TCP per call. Use HTTPConnection keep-alive. |
| L7 | `_proxy.py` | `on_start` only waits for token port (:9798), not cache (:9797). Poll both. |
| L8 | `proxy.rs` | `tokens_saved` counter never incremented — stats always show 0. |
| L9 | `generate-soul.py` | Uses `os.path.abspath(".")` — breaks on different machines. Use `pathlib.Path(__file__).parent.parent`. |
| L10 | `dev-dual.sh` | `RUST_LOG=info` floods terminal with HTTP framework noise. Use `RUST_LOG=aphrodite=info,tower_http=warn`. |

---

## Remaining from Original 91-Bug Audit

| # | Sev | Bug | Status |
|---|---|---|---|
| 51 | 🟠 | 16-char vs 64-char hash mix | ❌ |
| 52 | 🟡 | `_git_summary()` race + 3s block | ❌ |
| 53 | 🟡 | `.test-results.json` to plugin dir | ❌ |
| 54 | 🟡 | O(n) `_inline_store` scan | ❌ |
| 61 | 🟡 | `inline_ccr` dead code in proxy.rs | ❌ |
| 67 | 🟡 | `CcrStore` trait `len()` + `Send+Sync` | ❌ |
| 71 | 🟡 | Double `ctrl_c` — no drain | ❌ |
| 72 | 🟡 | Prometheus `_us` vs seconds | ❌ |
| 74 | 🟡 | `ccr_db_path` CWD not binary | ❌ |
| 77 | 🟡 | Query-only BAD_REQUEST | ❌ |
| 78 | 🟡 | `GET /retrieve` 405 | ❌ |
| 79 | 🟢 | `deny_unknown_fields` missing | ❌ |
| 80 | 🟡 | Both proxies bind same port | ❌ |
| 81 | 🟢 | Empty `api_key` silent | ❌ |
| 82 | 🟢 | `X-Aphrodite-Request-Id` | ❌ |
| 83 | 🟢 | CCR TTL not refreshed on `get()` | ❌ |
| 85 | 🟢 | `/metrics` unauthenticated | ❌ |
| 90 | 🟢 | `no_ccr_marker` per-proxy | ❌ |
| 91 | 🟢 | `CcrStore` trait bounds | ❌ |

---

## Running Infrastructure

- **headroom-cache** → 127.0.0.1:9799 (healthy, HEADROOM_DEEPSEEK_KEY) — USE THIS
- **cargo watch** → pane 6 (RUST_LOG=debug, both :9797 + :9798, but key is wrong)
- **WezTerm panes**: 0 (export), 4 (TUI), 6 (cargo watch proxy)

## Key Paths

- Crate: `crates/aphrodite/src/`
- Plugin: `plugins/aphrodite/`
- Configs: `profiles/*/config.yaml`
- Scripts: `scripts/`
- Master tasks: `.hermes/MASTER-TASKS.md`
- Handoff: `.hermes/HANDOFF.md`

## Workflow

1. Write plan script to `/tmp/aphrodite-plan-N.sh`
2. Fire: `terminal(command="bash /tmp/...", background=true, notify_on_complete=true)`
3. Continue talking while it runs
4. Result arrives as notification
5. Commit + push after each fix
