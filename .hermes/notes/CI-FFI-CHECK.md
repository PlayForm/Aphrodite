# CI-FFI-CHECK - GitHub Actions inspection for the FFI codegen pipeline

Inspected 2026-09-17 (EEST) via `gh api` (authed as NikolaRHristov), repo `PlayForm/Aphrodite`, branch `Development`.
Local checkout: the Aphrodite monorepo at `24c8258` (contains FFI commits `d1fc9ce`→`11e07d0` + one later chore commit).

---

## (a) Run 35265794844 - summary

**This run is the `Check` workflow (Check.yml), NOT ffi-check.yml.** The ffi-check run for the same push is 35265794725 (run #2, see below).

| Field             | Value                                                                                                 |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| Workflow          | Check (`.github/workflows/Check.yml`, id 297334145)                                                   |
| Run number        | 525                                                                                                   |
| Branch            | Development                                                                                           |
| Head SHA          | `11e07d0af71e4b61b6673b653d0abe75f70b63ac` ("refactor(ffi): simplify the bindings codegen pipeline…") |
| Event / actor     | push / NikolaRHristov                                                                                 |
| Created / updated | 2026-09-17T19:35:34Z / 19:39:27Z                                                                      |
| **Conclusion**    | **FAILURE**                                                                                           |

### Per-job results (all 4 jobs)

| Job            | Conclusion | Cause (key log excerpts)                                                                        |
| -------------- | ---------- | ----------------------------------------------------------------------------------------------- |
| cargo deny     | ✅ success | -                                                                                               |
| ruff + pyright | ❌ failure | ruff 0.16.8 (src `plugins/aphrodite/`), ~30+ errors; pyright step never ran (ruff failed first) |
| cargo check    | ❌ failure | clippy `doc-lazy-continuation`, `-D warnings`, rust 1.96.0                                      |
| cargo test     | ❌ failure | `error[E0432]: unresolved import headroom_core::transforms::kompress` (test `kompress_parity`)  |

### ruff + pyright (job 105352525079) - failures attributable to the FFI pipeline

```
##[error]plugins/aphrodite/__init__.py:78:19: N812 Lowercase `_bindings` imported as non-lowercase `_GENERATED_BINDINGS`
##[error]plugins/aphrodite/_bindings.py:347:20: N805 First argument of a method should be named `self`
##[error]plugins/aphrodite/_bindings.py:365:9: SIM114 Combine `if` branches using logical `or` operator
##[error]plugins/aphrodite/_bindings.py:387:5: N802 Function name `ReturnString` should be lowercase
##[error]plugins/aphrodite/_bindings.py:398:5: N802 Function name `UNCHECKED` should be lowercase
##[error]plugins/aphrodite/_bindings.py:407:7: N801 Class name `_variadic_function` should use CapWords convention
##[error]plugins/aphrodite/_bindings.py:407:26: UP004 Class `_variadic_function` inherits from `object`
##[error]plugins/aphrodite/_bindings.py:437:27: SIM101 …merge into a single call
##[error]plugins/aphrodite/_bindings.py:483..489: E402 Module level import not at top of file (×7)
##[error]plugins/aphrodite/_bindings.py:514..966: UP008/C408/UP032/UP031/B009/UP024/UP028 (×~20)
##[error]plugins/aphrodite/_bindings.py:901..975: B009 + F405 `c_void_p` may be undefined, or defined from star imports
##[error]plugins/aphrodite/tests/test_perf_probe.py:119:13: UP031 / 191:6: W292 (pre-existing)
##[error]The process '/opt/hostedtoolcache/ruff/0.16.8/x86_64/ruff' failed with exit code 1
```

`plugins/aphrodite/_bindings.py` is the **generated** ctypesgen output (committed in the submodule) and the generated code trips N802/N805/N801/E402/F405/etc. `__init__.py:78` N812 is the new `from _bindings import ... as _GENERATED_BINDINGS` import added by the FFI commits. The two `test_perf_probe.py` errors pre-date FFI (present in run 523, pre-FFI).

### cargo check (job 105352525716) - pre-existing, NOT FFI-attributable

```
warning: aphrodite-hermes@1.4.6: ctypesgen not found (tried `python3 -m ctypesgen` and `ctypesgen`);
         skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect   ← graceful degradation works
error: doc list item without indentation
   --> crates/aphrodite-hermes/src/lib.rs:539:5
539 | /// with `aphrodite_hermes_free_string`.
    = help: … rust-clippy/rust-1.96.0/index.html#doc_lazy_continuation
    = note: `-D clippy::doc-lazy-continuation` implied by `-D warnings`
error: could not compile `aphrodite-hermes` (lib) due to 1 previous error
```

The offending doc line was last touched by commit `fdc1f837` (2026-09-17 16:02), which **is an ancestor of the FFI range** - same failure existed in pre-FFI run 523 (`00b765f`). Not caused by the FFI commits.

### cargo test (job 105352525325) - pre-existing, NOT FFI-attributable

```
error[E0432]: unresolved import `headroom_core::transforms::kompress`
  --> vendor/headroom/crates/headroom-core/tests/kompress_parity.rs:17:32
note: found an item that was configured out
  --> vendor/headroom/crates/headroom-core/src/transforms/mod.rs:25:9
24 | #[cfg(feature = "ml")]
25 | pub mod kompress;                     ← gated behind `ml` feature
error: could not compile `aphrodite-headroom-core` (test "kompress_parity")
```

Test compiles `kompress` unconditionally while the module is `ml`-feature-gated. `vendor/headroom` pointer is unchanged across the FFI commits; identical failure in pre-FFI run 523. **Consequence: `cargo test` dies at compile time, so the later "Build aphrodite-hermes dylib" and "Python FFI tests" (pytest `tests/`) steps in the Test job never ran - the real end-to-end FFI tests have zero CI coverage right now.**

---

## (b) ffi-check.yml workflow history

| Run | ID          | Head SHA      | Branch      | Event | Created              | Conclusion |
| --- | ----------- | ------------- | ----------- | ----- | -------------------- | ---------- |
| 2   | 35265794725 | `11e07d0af7…` | Development | push  | 2026-09-17T19:35:34Z | ❌ failure |
| 1   | 35263203992 | `0d9e3ecbb8…` | Development | push  | 2026-09-17T19:09:35Z | ❌ failure |

### Step-level results (identical in both runs, job "Static FFI contract check")

| Step                                      | Run 1 (0d9e3ec) | Run 2 (11e07d0) | Output                                                                                                                 |
| ----------------------------------------- | --------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1 Set up job                              | ✅              | ✅              |                                                                                                                        |
| 2 checkout (v7.0.1, submodules recursive) | ✅              | ✅              | submodules at `43e93e7` (plugins/aphrodite), `84c8d117` (headroom), `5b6056a` (rtk)                                    |
| 3 FFI contract check (inline setup block) | ✅ PASS         | ✅ PASS         | `FFI contract check: 0 violation(s), 0 warning(s) -> PASS`                                                             |
| 4 FFI contract check (generated bindings) | ✅ PASS         | ✅ PASS         | `--bindings plugins/aphrodite/_bindings.py` → `0 violation(s), 0 warning(s) -> PASS`                                   |
| 5 **FFI checker self-tests**              | ❌ **FAIL**     | ❌ **FAIL**     | `FAIL test_cli_exit_codes_and_missing_files: regressed fixture CLI exit != 1` - 11/12 cases passed, 41 asserts, exit 1 |

Contract checks (the actual checker) **pass in CI on both commits**. The only ffi-check failure is the self-test step.

### Self-test failure root cause (verified locally)

`test_cli_exit_codes_and_missing_files` removes the inline line
`dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_void_p` from a fixture and expects CLI exit 1.
But the checker now auto-detects the **committed generated bindings** as the restype ground truth
(`restype source : generated bindings`, `bindings file: …/plugins/aphrodite/_bindings.py`) - the generated file still
declares the restype, so the removed inline line is masked: `0 violation(s), 0 warning(s) -> PASS`, exit 0.
The test's regression-detection premise predates the "prefer generated bindings" design; it no longer exercises
the path it intends to guard.

### Trigger-path assessment

Workflow triggers on push/PR to `Development` for paths:
`crates/aphrodite-hermes/src/lib.rs`, `plugins/aphrodite/__init__.py`, `plugins/aphrodite/_bindings.py`, `plugins/**`, `Maintain/check_ffi_contract.py`, `Maintain/tests/test_check_ffi_contract.py`.

✅ Correctly fired for both FFI commits (lib.rs + submodule gitlink changes).
⚠️ **Gap:** the pipeline's own generator files are NOT in the path filter:
`crates/aphrodite-hermes/build.rs`, `crates/aphrodite-hermes/cbindgen.toml`, `crates/aphrodite-hermes/codegen/**`
(`finalize_bindings.py`). Edits to the generator itself will not re-run FFI-Check.

---

## (c) CI-vs-local discrepancies

**No behavioral CI-vs-local discrepancy found - the CI failures reproduce locally where they are FFI-related.**

- **Self-test failure reproduces on macOS**: `python3 Maintain/tests/test_check_ffi_contract.py` locally →
  `FAIL test_cli_exit_codes_and_missing_files` (11/12, exit 1) - byte-for-byte the same failure as CI. It is a test-suite bug,
  not an environment/OS issue. (Local "checker PASS" refers to the contract checks, which _do_ pass in CI steps 3-4.)
- **ctypesgen availability differs but both paths are healthy**: CI ubuntu has no ctypesgen →
  `warning: … skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect`
  (graceful degradation confirmed working in cargo check AND cargo test). Locally ctypesgen is installed and generation actually runs.
  **Implication: CI never exercises the real cbindgen→ctypesgen→finalize codegen path - it always uses the committed artifact.**
- **"Cargo tests green" (local) vs cargo test FAIL (CI)**: the CI failure is `kompress_parity` in `vendor/headroom` (pre-existing,
  unrelated to FFI - fails identically in pre-FFI run 523). Local verification was evidently scoped to FFI-relevant crates, not
  `cargo test --workspace --release` on the full tree. Not an FFI regression.
- Nothing else diverges: cargo check failure (doc-lazy-continuation at lib.rs:539) is pre-existing (commit `fdc1f837`,
  ancestor of FFI range) and identical on both the FFI commits and pre-FFI `00b765f`.

---

## (d) Check.yml / Build.yml status for the FFI commit range

FFI parent commits: `d1fc9ce` → `0d9e3ec` → `11e07d0`; submodule `plugins/aphrodite`: `0f92a8e` → `a4623b3` (adds `_bindings.py`) → `43e93e7` (drops `_LiveLookup`).

| Commit                     | Check.yml run      | Result | Job breakdown                                      |
| -------------------------- | ------------------ | ------ | -------------------------------------------------- |
| `d1fc9ce`                  | none               | -      | no CI run recorded for this commit                 |
| `0d9e3ec`                  | #524 (35263203819) | ❌     | ruff ❌ · cargo check ❌ · cargo test ❌ · deny ✅ |
| `11e07d0`                  | #525 (35265794844) | ❌     | ruff ❌ · cargo check ❌ · cargo test ❌ · deny ✅ |
| `24c8258` (post-FFI chore) | #526 (35268222361) | ❌     | same 3 jobs red (identical causes)                 |

Attribution:

- **FFI-attributable (NEW):** ruff failures on `_bindings.py` (generated code) + `__init__.py:78` N812.
- **Pre-existing (not FFI):** cargo check clippy doc-lazy-continuation (lib.rs:539, from `fdc1f837`); cargo test
  kompress `ml`-feature E0432 (vendor/headroom); ruff `test_perf_probe.py` UP031/W292.
- **Build.yml:** tag-only trigger (`Aphrodite/v*`) - **no Build runs for the FFI range** (last Build run was 09-16, all green).
- The FFI codegen itself did not break any cargo compile: with ctypesgen absent, build.rs skipped generation and the
  committed `_bindings.py` compiled/loaded fine (only the pre-existing clippy lint stopped `cargo check`).

---

## (e) Recommendations

1. **Fix the stale self-test** `test_cli_exit_codes_and_missing_files`: the regressed fixture must remove the restype
   declaration from the _ground-truth_ source the checker uses (pass a `--bindings` fixture that lacks the line, or assert
   exit 0 with a comment, or re-point the test at the inline-setup path). Currently the inline-setup regression path has zero
   working coverage, on every platform.
2. **Exclude generated code from ruff** in the plugin's lint config (`per-file-ignores` for `_bindings.py`: N802/N805/N801/
   N811/N812/E402/F405/UP*/B009/SIM*…), and fix the two remaining `test_perf_probe.py` lints (or exclude the test file too).
   Also fix `__init__.py:78` N812 (use `import _bindings as _GENERATED_BINDINGS` under a `# noqa: N812` or alias the module).
   This unblocks the ruff+pyright job (pyright never even runs today).
3. **Add the codegen sources to the ffi-check trigger paths**: `crates/aphrodite-hermes/build.rs`,
   `crates/aphrodite-hermes/cbindgen.toml`, `crates/aphrodite-hermes/codegen/**` - otherwise generator changes ship unchecked.
4. **Exercise the real codegen path in CI**: install `ctypesgen` (and use cbindgen) in one job and verify regeneration is
   byte-identical to the committed `_bindings.py` (copy-on-change contract), so CI stops testing only the fallback path.
5. **Unblock cargo test** (fix `kompress_parity` to gate on the `ml` feature or enable the feature in the test job) - today the
   compile failure skips the entire "Build dylib + Python FFI tests (pytest)" coverage, the most valuable FFI end-to-end check.
6. **Fix the clippy doc-lazy-continuation** at `crates/aphrodite-hermes/src/lib.rs:539` (indent the continuation line) so
   `cargo check`/clippy go green on the committed code.
7. Non-blocking: give `d1fc9ce` a CI run (it never triggered - no Check run recorded), or re-push after fixes so every
   FFI commit has CI evidence.

---

_Report generated 2026-09-17; all statements backed by `gh api` job logs (raw log endpoints) and local execution.
No repo files were modified; this document is a copy delivered per task._
