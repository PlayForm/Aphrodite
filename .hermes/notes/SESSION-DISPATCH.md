# SESSION-DISPATCH - 3x2 agent plan (post-restart)

Launch order: **FFI CI fixes first (Pair A), then Issue #11 (Pairs B+C), then the
rest** (release ceremony, 1.5.0 backlog) in a later wave. Read
`.hermes/notes/HANDOFF-2026-09-17.md` first for full context.

**STATUS (2026-09-18): Pairs A, B, C are DONE. Issue #11 WS1+WS2+WS4 landed and
the full verification sweep is green (see `.hermes/notes/ISSUE-11-LANDED.md`).
Next wave: release ceremony + 1.5.0 backlog (section "Next wave" below).**

All agents: do NOT commit (auto-committer sweeps); no githooks exist (branch
discipline manual); write deliverables to both sigserve scratch
(`~/Developer/.playform/Temporary/sigserve/`) AND `.hermes/notes/` (prettier
--write + --check against repo .prettierrc: tabs, width 100, proseWrap
preserve); deliverables anonymized (repo-relative paths, ZERO local absolute
paths). Pairs of 2 per task, disjoint file ownership.

---

## Pair A - FFI CI fixes (from `.hermes/notes/CI-FFI-CHECK.md`) - DONE

### A1 - ruff on generated artifact + N812 (file owner: ruff config + plugin) - DONE

- `.ruff.toml` (repo file is `ruff.toml`): `[lint.per-file-ignores]` for the
  GENERATED `plugins/aphrodite/_bindings.py` (N801/N802/N805/N811/N812/E402/
  F405/B009/SIM/UP/C408 - machine-generated, never hand-edited; documented in a
  comment).
- `plugins/aphrodite/__init__.py:78` N812 fixed via `# noqa: N812` on the
  deliberate non-lowercase alias import; template synced byte-identical.
- Top-level `extend-exclude = ["crates/aphrodite/templates"]` (byte-identity
  shim copy, not lintable source).
- Result: ruff clean on plugins/ + codegen/ except the 2 known pre-existing
  perf-probe errors (UP031 :119, W292 :191 in
  `plugins/aphrodite/tests/test_perf_probe.py`) - left untouched per scope.
- Record: `.hermes/notes/PAIR-A1.md`.

### A2 - stale checker test + ffi-check.yml trigger gap (file owner: tests + workflow) - DONE

- `test_cli_exit_codes_and_missing_files` rewritten 13/13 (both ground-truth
  halves pinned: explicit `--bindings` fixture + neutralized default, plus the
  masked-behavior case documented as a toolchain property).
- Checker hardened: `[wrong-argcount]` + `[unknown-export-argtypes]` violation
  classes (argtypes-count gap closed), `argtypes : N` summary line.
- `.github/workflows/ffi-check.yml`: paths extended with `build.rs`,
  `cbindgen.toml`, `codegen/**`, `crates/aphrodite-hermes/**`,
  `plugins/aphrodite/**` (push + PR).
- Result: self-tests 13/13 (50 asserts), real repo PASS 0 violations.
- Record: `.hermes/notes/PAIR-A2.md`.

---

## Pair B - Issue #11 preview fix WS1+WS2 (from `.hermes/notes/ISSUE-11-FIXDESIGN.md`) - DONE

### B1 - tools.rs (file owner: `crates/aphrodite-hermes/src/tools.rs`) - DONE

- WS1: deleted `ok` collapse (tools.rs:123-126); guarded success-string arms
  (127-131/167-172, single-key only); caller-hint-wins (193-198).
- WS2-tools: honest `total_count` (tools.rs:135-156) - `matches_text`
  normalization, `take(20)` cap removed, zero/count-only totals surface,
  no `total_count`-only hijack.
- 6 new regression tests; `cargo test -p aphrodite-hermes` 46 -> 52.
- Record: `.hermes/notes/PAIR-B1.md`.

### B2 - preview.rs + config (file owner: `crates/aphrodite/src/preview.rs`,

`crates/aphrodite/src/config.rs`, `crates/aphrodite/templates/aphrodite.toml`) - DONE

- WS2-preview: honest build arm (line-based tallies + failure honesty),
  error/linter/log arms (first real error/lint line or tail).
- WS4: `preview_max_chars` wired end-to-end (config field was declared but
  unread; now enforced at the `build_preview` choke point, env > TOML > default
  120, char-boundary-safe with closing `]` + `…` preserved; startup + `/reload`
    - dylib init).
- 12 new tests; `cargo test -p aphrodite` 348 -> 389.
- Record: `.hermes/notes/PAIR-B2.md`; test-race fix in
  `.hermes/notes/PAIR-B-FIX.md` (20 preview tests hardened with `cap_guard()`).

---

## Pair C - Issue #11 regression battery + sweep (files: tests/, notes) - DONE

### C1 - regression tests (file owner: `tests/` in the hermes crate) - DONE (coverage landed via B1)

- The FIXDESIGN regression tests landed as B1's 6 new tools.rs tests (hint-
  keeps-type-and-full-preview, bare-success-object honesty, real-match-count
  search, matches-array counting, zero/count-only totals, total_count-only-not-
  search); no separate `tests/` dir exists in the hermes crate.
- **Battery-after record PENDING**: `ISSUE-11-BATTERY-AFTER.md` was not found
  at sweep time (not in `.hermes/notes/` or sigserve scratch). Re-run the
  95-row preview battery on the rebuilt dylib and record the defect-rate drop
  vs the 55% hook-path / 65% direct-path baselines.

### C2 - end-to-end sweep + records (file owner: `.hermes/notes/`) - DONE

- Full verification (2026-09-18, all gates actually run): checker PASS 0
  violations; repro SURVIVED; `cargo test -p aphrodite` 389 passed (0 failed,
  1 ignored); `cargo test -p aphrodite-hermes` 52 passed; codegen self-tests
  23/23; checker self-tests 13/13 (50 asserts); drift-guard `diff -q` identical;
  ruff only the 2 known pre-existing perf-probe errors; prettier clean.
- Wrote `.hermes/notes/ISSUE-11-LANDED.md` (what landed WS1/2/4, test counts
  before/after, battery before + after=PENDING, deferrals WS3 -> 1.5.0, CI
  status after A1/A2, sweep table).
- Updated `.hermes/notes/HANDOFF-2026-09-17.md` (2.1/2.2 done, pending list
  refreshed) + this dispatch.

---

## Next wave (current queue, after C2 lands)

- **Release ceremony**: `Aphrodite/v1.4.6` tag + Finalize 12 assets +
  BINARY_VERSION last (RELEASE-METHODOLOGY.md PART 4/5; submodule-first,
  cherry-pick -x, no rebase). NEW: branch-identity audit step before tagging
  (parent + submodule both on Development; no phantom gitlink entry).
- **Battery-after**: run C1's 95-row preview battery on the rebuilt dylib ->
  `ISSUE-11-BATTERY-AFTER.md`.
- **1.5.0 backlog**: WS3 proxy-pipeline parity, structured hook payloads,
  teardown export, rust-bindgen corpus stress tests, real-codegen-path CI
  (ctypesgen byte-identity), `proxy_health` export decision.
- **Housekeeping**: Documentation repo `Module/Astro` phantom gitlink cleanup.

## Resume sanity (plugin re-install if needed)

- `cp target/release/libaphrodite_hermes.dylib ~/.hermes/aphrodite/binaries/`
    - `cp target/release/aphrodite ~/.hermes/aphrodite/binaries/` (binaries/
      was emptied by a session reset on 2026-09-17 ~23:21; release artifacts
      survive in target/release/).
- Verify: python3 repro.py SURVIVED; version round-trip returns 1.4.6;
  get_hooks returns 6 hooks.
