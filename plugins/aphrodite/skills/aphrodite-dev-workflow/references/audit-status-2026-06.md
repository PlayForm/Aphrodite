# Aphrodite Code Audit — June 2026

Deep audit of PlayForm/Aphrodite by remote engineer. 16 bugs identified.
Last reviewed: 2026-06-15.

## Bug Status Summary

| # | Bug | Location | Status |
|---|-----|----------|--------|
| 1 | Typo in env-var name: `APHRODITEINLINE_THRESHOLD` (missing `_`) | `plugins/aphrodite/__init__.py` | ✅ FIXED |
| 2 | Duplicate `_inline_store`/`INLINE_THRESHOLD` re-declarations shadowing configured values | `plugins/aphrodite/__init__.py` | ✅ FIXED |
| 3 | `_alive()` fragile health-check — `"ok"` vs JSON mismatch | `plugins/aphrodite/__init__.py` | ✅ FIXED (now uses `json.loads`) |
| 4 | `_rebuild_handler` hardcoded absolute path | `plugins/aphrodite/__init__.py` | ✅ FIXED (uses `__file__` relative) |
| 5 | `_download_binary()` ignores platform tag | `plugins/aphrodite/__init__.py` | ✅ FIXED |
| 6 | Tool description has wrong Unicode marker glyphs | `crates/aphrodite/src/proxy.rs` | ✅ FIXED (ASCII `<<<CCR:...>>>`) |
| 7 | `tokens_saved` counter never incremented | `crates/aphrodite/src/proxy.rs` | ✅ FIXED |
| 8 | `/health` endpoint calls upstream API on every check | `crates/aphrodite/src/proxy.rs` | ✅ FIXED (local-only, `/health/upstream` separate) |
| 9 | Proxy launch has 0.5s fixed sleep, no retry | `plugins/aphrodite/__init__.py` | ✅ FIXED (`_wait_alive` retry loop) |
| 10 | `_pre_llm_hook` calls `_alive()` twice per message pair | `plugins/aphrodite/__init__.py` | ✅ FIXED (5s TTL cache) |
| 11 | Context engine `should_compress()` always returns `True` | `plugins/aphrodite/__init__.py` | ✅ FIXED (threshold-based) |
| 12 | `_resolve_one()` hard-codes only token port | `plugins/aphrodite/__init__.py` | ✅ FIXED (tries both ports) |
| 13 | `compress()` truncates messages to 2000 chars | `plugins/aphrodite/__init__.py` | ✅ FIXED (full content, no truncation) |
| 14 | `marker_for()` vs `compute_key()` hash inconsistency | Both Python + Rust | ✅ VERIFIED (hash extraction clean) |
| 15 | `BIN_VERSION` stale (v0.4.0 vs v0.5.0 tag) | `plugins/aphrodite/__init__.py` | ✅ FIXED (2026-06-15) |
| 16 | Binary download 404 on fresh machines | — | Mitigated (BIN_VERSION synced, fallback to cargo build) |

## Post-Audit Fixes (2026-06-15)

| # | Fix | Location | Status |
|---|-----|----------|--------|
| A1 | `tokens_saved` missing from `handle_ccr_create` path — Python engine compressions uncounted | `proxy.rs` :621 | ✅ FIXED (added `fetch_add`) |
| A2 | Bug #34: ContextTracker over-expansion — `relevance_threshold=0.3` too generous, common coding keywords not filtered | `vendor/../context_tracker.py` | ✅ FIXED (0.3→0.5, 23 coding stop words added) |

## Verification Checklist (per bug type)

### CCR Marker Consistency
- [ ] Python `_ccr_marker()` uses `<<<CCR:hash|type|size|mode>>>`
- [ ] Rust `smart_marker()` uses `<<<CCR:hash|type|size>>>`
- [ ] Rust tool injection description uses same format
- [ ] Python `_transform_terminal_hook` output uses same format
- [ ] No Unicode brackets anywhere (`⫷`, `⭷`, etc.)

### Version Sync
- [ ] `BIN_VERSION` in `__init__.py` matches Cargo.toml `version`
- [ ] `git tag` matches both
- [ ] Binary rebuilt after Cargo.toml bump (version embedded at compile time)

### Health Check
- [ ] `/health` is local-only (no upstream API call)
- [ ] `/health/upstream` probes upstream separately
- [ ] Python `_alive()` uses JSON parse, not substring match
