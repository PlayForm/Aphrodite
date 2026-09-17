# SESSION-DISPATCH - 3×2 agent plan (post-restart)

Launch order: **FFI CI fixes first (Pair A), then Issue #11 (Pairs B+C), then the
rest** (release ceremony, 1.5.0 backlog) in a later wave. Read
`.hermes/notes/HANDOFF-2026-09-17.md` first for full context.

All agents: do NOT commit (auto-committer sweeps); no githooks exist (branch
discipline manual); write deliverables to both sigserve scratch
(`~/Developer/.playform/Temporary/sigserve/`) AND `.hermes/notes/` (prettier
--write + --check against repo .prettierrc: tabs, width 100, proseWrap
preserve); deliverables anonymized (repo-relative paths, ZERO local absolute
paths). Pairs of 2 per task, disjoint file ownership.

---

## Pair A - FFI CI fixes (from `.hermes/notes/CI-FFI-CHECK.md`)

### A1 - ruff on generated artifact + N812 (file owner: ruff config + plugin)

- `.ruff.toml`: add per-file-ignores for the GENERATED
  `plugins/aphrodite/_bindings.py` (N802/N805/N801/E402/F405/B009 - it is
  machine-generated, never hand-edited; document why in a comment).
- Fix the one real lint: `plugins/aphrodite/__init__.py:78` N812 (lowercase
  import) - rework to an alias import.
- Verify: `ruff check` clean on plugins/ + Maintain/ + codegen/ (no new
  errors; pre-existing perf-probe errors in plugins/ are known - list them,
  do not fix out of scope).
- Sync template byte-identity if **init**.py changed:
  `cp plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`
    - `diff -q` + drift-guard test green.

### A2 - stale checker test + ffi-check.yml trigger gap (file owner: tests + workflow)

- Fix `test_cli_exit_codes_and_missing_files` (fails 11/12): the checker now
  prefers the committed generated `_bindings.py` as restype ground truth, so
  removing the inline restype line is masked. Update the fixture to the new
  ground-truth behavior (delete or neutralize the artifact in the fixture dir,
  or assert the new semantics).
- `.github/workflows/ffi-check.yml`: extend `paths` to include `build.rs`,
  `cbindgen.toml`, `codegen/**`, `plugins/aphrodite/**` so generator changes
  re-run the check.
- Verify: `python3 Maintain/tests/test_check_ffi_contract.py` 12/12 green.

---

## Pair B - Issue #11 preview fix WS1+WS2 (from `.hermes/notes/ISSUE-11-FIXDESIGN.md`)

### B1 - tools.rs (file owner: `crates/aphrodite-hermes/src/tools.rs`)

- WS1: delete `ok` collapse (tools.rs:123-126); guard success-string arms
  (127-131/167-172); caller-hint-wins (193-198).
- WS2-tools: honest `total_count` (tools.rs:135-156).
- Verify: `cargo test -p aphrodite-hermes` green; existing tool tests pass.

### B2 - preview.rs + config (file owner: `crates/aphrodite-hermes/src/preview.rs`,

`crates/aphrodite/src/config.rs`, `crates/aphrodite/templates/aphrodite.toml`)

- WS2-preview: honest build arm (preview.rs:184-185); error/linter/log arms
  (294-304).
- WS4: wire the `preview_max_chars` cap end-to-end (config field exists,
  declared but unread - now read it and cap previews).
- Verify: cargo build + `cargo test -p aphrodite` green; config template
  updated.

---

## Pair C - Issue #11 regression battery + sweep (files: tests/, notes)

### C1 - regression tests (file owner: `tests/` in the hermes crate)

- Implement the 5 regression tests from `.hermes/notes/ISSUE-11-FIXDESIGN.md`
  (search-collapse honesty, build-arm honesty, error/linter/log arms,
  total_count, preview cap).
- Verify: new tests pass against B1/B2 changes; run the issue-11 battery
  (95-row preview battery if reproducible) showing the defect-rate drop.

### C2 - end-to-end sweep + records (file owner: `.hermes/notes/`)

- Full verification: repro SURVIVED, `Maintain/check_ffi_contract.py` PASS,
  cargo tests all green, drift-guard, ruff clean, prettier clean.
- Write `.hermes/notes/ISSUE-11-LANDED.md`: what landed (WS1/2/4), battery
  before/after, remaining deferrals (WS3 -> 1.5.0), CI status after A1/A2.
- Update HANDOFF-2026-09-17.md: mark 2.1/2.2 done, refresh pending list.

---

## Later wave (after C2 lands, per user)

- Release ceremony: `Aphrodite/v1.4.6` tag + Finalize 12 assets +
  BINARY_VERSION last (RELEASE-METHODOLOGY.md PART 4/5; submodule-first,
  cherry-pick -x, no rebase).
- 1.5.0 backlog: WS3 proxy-pipeline parity, structured hook payloads,
  teardown export, rust-bindgen corpus stress tests.
- Housekeeping: Documentation repo `Module/Astro` phantom gitlink cleanup.

## Resume sanity (plugin re-install if needed)

- `cp target/release/libaphrodite_hermes.dylib ~/.hermes/aphrodite/binaries/`
    - `cp target/release/aphrodite ~/.hermes/aphrodite/binaries/` (binaries/
      was emptied by a session reset on 2026-09-17 ~23:21; release artifacts
      survive in target/release/).
- Verify: python3 repro.py SURVIVED; version round-trip returns 1.4.6;
  get_hooks returns 6 hooks.
