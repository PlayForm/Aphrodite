# PREVIEW-RACE-FIX - preview test-suite flake (PREVIEW_MAX_CHARS race)

Date: 2026-09-18 · Branch: Development · Not committed (auto-committer sweeps)

## Symptom

`cargo test -p aphrodite --lib preview` is flaky: parent-agent runs observed a
varying **21-25 failures per run** (44 / 41 / 40 passing across 3 runs). The
variance is the fingerprint of the known race: the process-global
`PREVIEW_MAX_CHARS` `AtomicU32` (`crates/aphrodite/src/preview.rs`) is mutated
by tests via `set_preview_max_chars()` while OTHER content-asserting tests run
in parallel and read the truncated cap → previews come out shorter than the
assertions expect → assert fails. The failure set varies because which
unguarded test happens to read the cap mid-mutation depends on thread
scheduling.

## Baseline (pre-fix) failing-test list per run

Local baseline runs (3x, this session, BEFORE any change):

| Run | Result                                                |
| --- | ----------------------------------------------------- |
| 1   | 65 passed / 0 failed - race did NOT reproduce locally |
| 2   | 65 passed / 0 failed - race did NOT reproduce locally |
| 3   | 65 passed / 0 failed - race did NOT reproduce locally |

The race is timing/thread-count dependent and did not fire on this machine,
but the parent-agent's observed 21-25 failures per run is the same signature.
The statically-derived set of UNGUARDED content-asserting tests (the failure
candidates when the race fires) is:

- `ffi_tests::test_build_preview_terminal_surfaces_exit_code`
- `ffi_tests::test_build_preview_terminal_falls_back_to_first_line`
- `proxy::tests::test_build_preview_code_has_ct_prefix`
- `proxy::tests::test_proxy_and_hook_previews_are_identical_for_semantic_shapes`
- `proxy::tests::test_build_preview_error_has_ct_prefix_via_error_line`
- `proxy::tests::test_build_preview_diff_has_ct_prefix`
- `proxy::tests::test_build_preview_json_has_ct_prefix`
- `marker::tests::test_marker_preview_never_doubles_bracket_prefix`

## Root cause

The shared mutex (`cap_guard()` in `preview::tests`, backed by the
`pub(crate)` `preview_cap_test_guard()` at `preview.rs:594`) was already
applied to 51 of 52 `preview::tests` and to
`config_loader::tests::test_apply_previews_wires_preview_max_chars`
(`config_loader.rs:511`). Every content-asserting test OUTSIDE those modules
that calls `build_preview` (directly or via `proxy_build_preview`) was
unguarded, so it read the cap while a cap-mutating test held it mid-change.

The one remaining unguarded `#[test]` in `preview.rs`
(`test_new_arms_never_panic_on_pathological_input`, line 1762) makes NO content
assertions (panic-smoke only) → intentionally left unguarded.

## Fix - guard additions per test (file:line)

Added `let _g = crate::preview::preview_cap_test_guard();` as the FIRST
statement of each test (fully-qualified; `preview_cap_test_guard` is
`pub(crate)` so `ffi_tests`, `proxy::tests` and `marker::tests` can reach it).
No assertions or behavior changed - purely the missing guard lines.

| File                           | Guard line | Test                                                                    |
| ------------------------------ | ---------- | ----------------------------------------------------------------------- |
| crates/aphrodite/src/lib.rs    | 747        | `ffi_tests::test_build_preview_terminal_surfaces_exit_code`             |
| crates/aphrodite/src/lib.rs    | 758        | `ffi_tests::test_build_preview_terminal_falls_back_to_first_line`       |
| crates/aphrodite/src/marker.rs | 245        | `marker::tests::test_marker_preview_never_doubles_bracket_prefix`       |
| crates/aphrodite/src/proxy.rs  | 2972       | `tests::test_detect_content_type_json_tool_output`                      |
| crates/aphrodite/src/proxy.rs  | 2978       | `tests::test_detect_content_type_invalid_json_is_text`                  |
| crates/aphrodite/src/proxy.rs  | 2985       | `tests::test_detect_content_type_json_array`                            |
| crates/aphrodite/src/proxy.rs  | 2991       | `tests::test_detect_content_type_rust_code`                             |
| crates/aphrodite/src/proxy.rs  | 2998       | `tests::test_detect_content_type_python_code`                           |
| crates/aphrodite/src/proxy.rs  | 3005       | `tests::test_detect_content_type_go_code`                               |
| crates/aphrodite/src/proxy.rs  | 3012       | `tests::test_detect_content_type_js_code`                               |
| crates/aphrodite/src/proxy.rs  | 3019       | `tests::test_detect_content_type_error_first_line`                      |
| crates/aphrodite/src/proxy.rs  | 3028       | `tests::test_detect_content_type_diff`                                  |
| crates/aphrodite/src/proxy.rs  | 3036       | `tests::test_detect_content_type_log_lines`                             |
| crates/aphrodite/src/proxy.rs  | 3044       | `tests::test_detect_content_type_empty_is_text`                         |
| crates/aphrodite/src/proxy.rs  | 3050       | `tests::test_detect_content_type_plain_text`                            |
| crates/aphrodite/src/proxy.rs  | 3098       | `tests::test_build_preview_code_has_ct_prefix`                          |
| crates/aphrodite/src/proxy.rs  | 3110       | `tests::test_proxy_and_hook_previews_are_identical_for_semantic_shapes` |
| crates/aphrodite/src/proxy.rs  | 3127       | `tests::test_build_preview_error_has_ct_prefix_via_error_line`          |
| crates/aphrodite/src/proxy.rs  | 3135       | `tests::test_build_preview_diff_has_ct_prefix`                          |
| crates/aphrodite/src/proxy.rs  | 3143       | `tests::test_build_preview_json_has_ct_prefix`                          |

Notes:

- The 12 `test_detect_content_type_*` tests assert on detect output; per the
  fix rule ("every test that asserts on build_preview/detect output holds the
  mutex") they take the guard too. They cannot fail from the cap race
  (`detect_type` does not read the cap), but holding the mutex is the stated
  invariant and is harmless.
- The 5 proxy `test_build_preview_*` tests go through `proxy_build_preview`,
  which calls the SAME `crate::preview::build_preview` the FFI/hook path uses -
  they are the primary in-race victims alongside the 2 `ffi_tests` terminal
  tests.
- Agent B's Issue #11 rewrite tests that call `set_preview_max_chars`
  (`test_preview_cap_*` in `preview.rs`, `config_loader.rs:511`) all hold the
  guard already; the ones that don't restore the previous cap
  (`test_preview_cap_unset_is_unlimited`) are safe because every cap READER
  also holds the guard, so the value is only observed inside the mutex.

## Verification (post-fix)

- `cargo test -p aphrodite --lib preview` × 3: **65 passed / 0 failed** each
  (0.08s, 0.23s, 0.06s) → 3x green
- `cargo test -p aphrodite` (full): 7 test binaries, 406 passed / 0 failed /
  1 ignored, exit 0 → green
- `python3 Maintain/check_ffi_contract.py`: 0 violations, 0 warnings → PASS
- `python3 .hermes/tmp/issue11_repro.py`: exit 0, enriched previews intact
  (issue_json_indented → `[tool_result:4L 58…]`, type-control → `[text:3L 21B | {]`)
  → repro SURVIVED
- `prettier --check` on touched files: see run output (md files clean; .rs
  files are not prettier-parsed in this repo)

## Files touched

- `crates/aphrodite/src/lib.rs` (+2 guard lines)
- `crates/aphrodite/src/marker.rs` (+1 guard line)
- `crates/aphrodite/src/proxy.rs` (+17 guard lines)
- `sigserve/PREVIEW-RACE-FIX.md` (this file)
- `.hermes/notes/PREVIEW-RACE-FIX.md` (copy)
