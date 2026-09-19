# PAIR-B-FIX - preview test race fix (post-Pair-B verification catch)

Date: 2026-09-18 (EEST) · Branch: Development · Binary 1.4.6 / plugin 2.1.4 · macOS

## The bug

Pair B2's WS4 change introduced a process-global `PREVIEW_MAX_CHARS` cap
(`crates/aphrodite/src/preview.rs`, AtomicU32). Its tests serialize among
themselves via `preview_cap_test_guard()` (a module-static Mutex), but the
OTHER preview tests (which assert full preview content) did NOT take the guard:

| Behavior                                                                            | Before (bug)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | After (fix)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| preview tests asserting full preview content (e.g. `test_preview_diff_names_files`) | cargo's parallel test runner could execute them while a cap test had `cap=30` set mid-window; the diff preview got truncated and `assert!(p.contains("src/main.rs"))` failed. Surfaced only in the full-group run (the child's background `cargo test` output that it never read); the test passed in isolation. Failure signature (deterministic in group, passed alone): `test_preview_diff_names_files` panicked at preview.rs:1081; then, after partial fixes, `test_preview_test_names_first_failure` panicked at preview.rs:1026 - confirming the whole class, not one test | every preview test that asserts on preview content now acquires `cap_guard()` as its first statement (Rust item order makes the helper defined at :1245 visible to all tests in the module). 20 tests patched: detect/preview git_log, git_status, git_status_rename, cargo_test, ls_long, ripgrep; build_surfaces_first_error, diff_names_files, fallback_shows_first_line_hint, semantic_detector, json array/object/fallback, search counts/ignores, html, build counts/failing/passing/capitalized, error arm, lint arm, log arm, test_names_first_failure, code_rust char boundary, multibyte every type, literal marker, interior nul x2, detect_type multibyte, interior nul. The 4 cap tests already had the guard |

## Verification

- `cargo test -p aphrodite --lib preview` x3: 48 passed / 0 failed each
- `cargo test -p aphrodite`: 360 lib + 16+5+2+4+2 bins, 0 failed
- `cargo test -p aphrodite-hermes`: 52 passed / 0 failed
- `Maintain/check_ffi_contract.py`: PASS 0 violations
- repro.py: SURVIVED (no crash)
- prettier --check on preview.rs: clean
- release dylib rebuilt + copied to the runtime binaries dir (plugin
  hot-reloads on next mtime check)

## Notes

- This was NOT an FFI/segfault issue - it is a pure test-race from the new
  process-global cap. The two macOS crash reports earlier in the window
  (23:57 / 00:04) were subagent probe scripts calling raw ctypes without
  setting restype (c_int truncation) - same bug class the pipeline
  enforces against; both probes were fixed/deleted, and the plugin path
  itself never crashed.
- Follow-up (1.5.0): the cap tests would be more robust as a serial
  `--test-threads` module or an RAII guard that restores on Drop; the
  current explicit restore is fine but easy to forget in future tests.
