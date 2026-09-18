# PAIR-A2 - stale checker test fix + ffi-check trigger gap (FFI CI)

Date: 2026-09-17 (EEST). Repo: Aphrodite monorepo, branch `Development`
(binary 1.4.6 / plugin 2.1.4). Pair A2 scope per `SESSION-DISPATCH.md`:
fix `test_cli_exit_codes_and_missing_files`, extend `ffi-check.yml` paths to
cover generator inputs, audit the checker for argtypes-count gaps, verify the
self-test suite, deliver this record. No commits made (auto-committer sweeps).

Files touched: `Maintain/check_ffi_contract.py`,
`Maintain/tests/test_check_ffi_contract.py`, `.github/workflows/ffi-check.yml`.

---

## 1. Stale test fix (old assertion vs new)

### Old behavior (broken, 11/12 in CI and locally)

`test_cli_exit_codes_and_missing_files` removed the inline line
`dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_void_p` from
a fixture and asserted CLI exit 1. The checker now auto-detects the committed
generated `plugins/aphrodite/_bindings.py` as the restype ground truth
(`restype source : generated bindings`); the artifact still declares the
restype, so the removed inline line was masked -> `0 violation(s) -> PASS`,
exit 0, assertion failed. The test's regression-detection premise predated the
"prefer generated bindings" design.

### New behavior (13 asserts, both halves pinned)

The masking is a toolchain property, NOT a relaxation of the contract: a
missing restype on a pointer-returning export must fail validation whenever it
is absent from the ground-truth source the checker actually uses. The rewrite
pins both halves:

1. **Real contract, bindings ground truth**: explicit `--bindings` fixture
   whose `materialize_directives.restype` line is removed -> exit 1.
2. **Real contract, inline ground truth**: `ffi.DEFAULT_BINDINGS` temporarily
   neutralized (pointed at a nonexistent path, restored in `finally`) so the
   historical inline-regression class is visible again -> exit 1.
3. **Masked behavior, pinned explicitly**: inline line removed while the
   artifact covers the symbol -> exit 0 (documented as the toolchain
   property).
4. Clean fixture with explicit `--bindings` -> exit 0 (also exercises the
   explicit `--bindings` flag path).
5. Usage errors -> exit 2 for missing `--bindings` file, missing plugin, and
   (new) missing lib.rs.

The test is hermetic: every case is driven by temp-dir fixtures, so it no
longer depends on the real repo's artifact being present or absent.

## 2. ffi-check.yml trigger-path fix

Old paths (push + pull_request): `crates/aphrodite-hermes/src/lib.rs`,
`plugins/aphrodite/__init__.py`, `plugins/aphrodite/_bindings.py`,
`plugins/**`, `Maintain/check_ffi_contract.py`,
`Maintain/tests/test_check_ffi_contract.py`.

Added to BOTH `push.paths` and `pull_request.paths`:

```yaml
- crates/aphrodite-hermes/build.rs
- crates/aphrodite-hermes/cbindgen.toml
- crates/aphrodite-hermes/codegen/**
- crates/aphrodite-hermes/**
- plugins/aphrodite/**
```

Rationale: `build.rs` / `cbindgen.toml` / `codegen/**` (finalize_bindings.py)
drive the committed artifact - a generator change must re-run the contract
check even when lib.rs is untouched. `plugins/aphrodite/**` covers the
submodule gitlink (the submodule is `plugins/aphrodite`, per `git submodule
status`), so artifact revisions trigger the check; the existing `plugins/**`
umbrella is kept. Comment in the workflow corrected: the gitlink is
`plugins/aphrodite`, not `plugins`.

## 3. Checker audit (hardening)

Audited `Maintain/check_ffi_contract.py` for correctness gaps:

- **Argtypes-count gap (CLOSED)**: the checker verified restype only. The
  codegen finalize step
  (`crates/aphrodite-hermes/codegen/finalize_bindings.py`) validates
  "argtypes must match the header's parameter counts" at build time, but a
  hand-edited committed artifact bypasses that gate and the static checker
  never looked at argtypes. ctypes marshals arguments positionally and never
  verifies the count, so a mismatch silently misaligns the ABI frame.
  Added:
    - `parse_export_arg_counts()` - Rust parameter counts from the same
      `EXPORT_RE` (regex extended with an args group; `parse_exports` behavior
      unchanged).
    - `parse_argtypes_assignments()` - AST-based, same discipline as the
      restype parser (export-prefix filter, string-literal and k32/wt noise
      structurally excluded), keeps the list length + element expressions.
    - New violation class `[wrong-argcount]` for every bound export whose
      argtypes list length != Rust parameter count.
    - New violation class `[unknown-export-argtypes]` for argtypes configured
      on a symbol that is not an export (typo protection, mirroring
      `unknown-export-configured`).
    - `argtypes : N` summary line in the CLI output.
    - No existing behavior changed: same `check()` signature and return,
      same exit-code contract (0/1/2), same violation strings for the old
      classes. Real repo (inline + generated bindings modes): all 10 exports'
      argtypes counts match lib.rs, 0 violations.
- **`--bindings` flag path (CONFIRMED WORKING)**: explicit
  `--bindings plugins/aphrodite/_bindings.py` -> PASS; explicit missing file
  -> exit 2. Also covered by the rewritten CLI test.
- **Other gaps noted, not changed (out of scope)**: nested-paren argument
  lists (function-pointer args) are not counted by the regex (lib.rs exports
  are plain pointer/scalar args - documented); the inline-fallback restype
  path in `__init__.py` (`_manual_ffi_setup`) is still parsed, but the
  checker's ground-truth preference for the artifact is the intended design.

## 4. Verification

- Self-tests: `python3 Maintain/tests/test_check_ffi_contract.py` ->
  **13/13 cases passed, 50 asserts, exit 0** (12 pre-existing + 1 new
  `test_argtypes_count_mismatch_flagged`).
- Real repo: `python3 Maintain/check_ffi_contract.py` -> PASS,
  `0 violation(s), 0 warning(s)`, exit 0 (auto-detect and explicit
  `--bindings` modes).
- Ruff: `ruff check Maintain/check_ffi_contract.py
Maintain/tests/test_check_ffi_contract.py` -> "All checks passed!".
- YAML: `yaml.safe_load` OK; `npx prettier --check
.github/workflows/ffi-check.yml` -> clean.

## 5. Follow-ups (unchanged, from CI-FFI-CHECK.md)

- A1's ruff-exclusion for the generated artifact + N812 fix (separate pair).
- Exercise the real codegen path in CI (install ctypesgen; byte-identity
  check) - CI still only tests the committed-artifact fallback.
- Pre-existing CI blockers: `kompress_parity` `ml`-feature E0432 (vendor),
  clippy doc-lazy-continuation at lib.rs.
