# MERGE-PARENT-AUDIT.md - Independent auditor report: Phase B parent sync-back (empty pick set + B2 gitlink bump)

**Date:** 2026-09-18
**Repo:** Aphrodite (parent), branch `Development`
**Auditor role:** read-only; no commits, no cherry-picks, no pushes issued by this lane.
**Audited record:** `MERGE-PARENT-EXEC.md` (partner's execution record) + live git state, verified independently.

**Prime directive check:** NOTHING LOST. Verified below - checksums identical, zero deletions came
over, protected paths unchanged, no phantom gitlink, HEAD advanced only by the declared commits.

## 1. Pre-state capture (independent, taken while the partner was still executing)

| Item                                                  | Observed pre-state                                                                                                                                | Evidence                                                                                                                        |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| HEAD                                                  | `11c98a8` (docs(.hermes): expand SPLIT-ADAPTATION...)                                                                                             | `git rev-parse HEAD` at audit start                                                                                             |
| Parent gitlink `plugins/aphrodite`                    | `d4b426b` in committed tree; index already staged to `098134e` (no `+` shown)                                                                     | `git ls-tree 11c98a8 plugins/aphrodite` = `160000 commit d4b426b...`; first `git submodule status` showed `098134e` without `+` |
| `preview.rs` checksum                                 | `e4e011094e166be7fee317667511b7104c3a737071a9441acde0d038d57ff2ee`                                                                                | `shasum -a 256`                                                                                                                 |
| `config_loader.rs` checksum                           | `e3f271a64da0cabd152742b3f61ac4f7b63c8e38e114d5f89fa869eddcce51ba`                                                                                | `shasum -a 256`                                                                                                                 |
| `check_ffi_contract.py` checksum                      | `cae37b8e5fb4bfaa923097ee18ce2a59b581e700d0cc110e97481560a9d7e795`                                                                                | `shasum -a 256`                                                                                                                 |
| Inventory `Maintain/`, `.hermes/`, `bench/`, `tests/` | all present                                                                                                                                       | `test -e` per path                                                                                                              |
| `tests/` 5 submodule plugin tests                     | `test_aphrodite_hermes_plugin.py`, `test_aphrodite_plugin_shim.py`, `test_directives_materialize.py`, `test_hotreload.py`, `test_layout_check.py` | `ls tests/*.py` = 5                                                                                                             |
| `plugins/aphrodite/__init__.py` + `_bindings.py`      | both present                                                                                                                                      | `ls`                                                                                                                            |

## 2. Post-state verification (NOTHING LOST)

| Check                                                           | Result   | Evidence                                                                                                                                                                                                |
| --------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Checksums identical (3 files)                                   | **PASS** | post = pre, byte-for-byte (`shasum -a 256`, 3/3 match)                                                                                                                                                  |
| No deletions came over (`Maintain/.hermes/bench/tests` present) | **PASS** | all 4 paths present post; range diff `11c98a8..HEAD` = 5 files touched, all adds except `plugins/aphrodite` gitlink (1-line mode change)                                                                |
| Protected paths unchanged (`.gitmodules`, `.github/workflows`)  | **PASS** | `git diff 11c98a8 HEAD -- .gitmodules .github/workflows` = EMPTY (0 lines)                                                                                                                              |
| `git submodule status` no `+`                                   | **PASS** | `098134e` / `84c8d117` / `5b6056a` - no `+` on any of the 3 submodules                                                                                                                                  |
| No 160000 phantom                                               | **PASS** | `git -C plugins/aphrodite ls-files -s \| grep 160000` = 0 hits; parent HEAD has exactly 3 `160000` entries (the 3 real submodules)                                                                      |
| HEAD advanced only by declared commits                          | **PASS** | `git log --oneline 11c98a8..HEAD` = exactly `a613948` (gitlink bump + MERGE-SUBMODULE docs) and `22a66f7` (this lane's exec record). Matches partner's PICK table: **empty pick set** + B2 gitlink bump |
| Gitlink landed at declared target                               | **PASS** | `git ls-tree HEAD plugins/aphrodite` = `160000 commit 098134ea0d477a70eac3c8ba1c42e24752aa2e32`                                                                                                         |
| No conflict markers in range                                    | **PASS** | `git diff 11c98a8 HEAD \| grep -c '^<<<<<<<\|^>>>>>>>\|^=======$'` = 0                                                                                                                                  |
| Range diff composition                                          | **PASS** | `git diff --stat 11c98a8 HEAD`: `INDEX.md +1`, `MERGE-PARENT-EXEC.md +69`, `MERGE-SUBMODULE-AUDIT.md +77`, `MERGE-SUBMODULE-EXEC.md +47`, `plugins/aphrodite 2 +-` (gitlink). Nothing else              |

## 3. PASS/FAIL table

| #   | Check                                                               | Result |
| --- | ------------------------------------------------------------------- | ------ |
| 1   | Pre-state HEAD/gitlink captured before sync-back completed          | PASS   |
| 2   | Pre-state checksums captured                                        | PASS   |
| 3   | Post-state checksums identical to pre-state                         | PASS   |
| 4   | No deletions came over (Maintain/.hermes/bench/tests)               | PASS   |
| 5   | Protected paths unchanged                                           | PASS   |
| 6   | No `+` on any submodule, no phantom 160000                          | PASS   |
| 7   | HEAD advanced only by declared commits (empty picks + gitlink bump) | PASS   |
| 8   | Picked content present and coherent (see section 4)                 | PASS   |
| 9   | No conflict markers, no invalid content landed                      | PASS   |
| 10  | Build/contract gates green                                          | PASS   |

**Overall: PASS.** Nothing at risk; nothing for the partner to restore.

## 4. Independent content spot-verification of the picks

The parent lane's PICK table is **empty** - every Current-only candidate was content-inspected and
either already exists in Development or is a superseded/snapshot/delete-something commit. The
auditor independently re-verified the three substantive "already present" claims plus the gitlink
bump (this is the content the sync-back was supposed to carry - it must exist in Development, real
and coherent, or the claim is a lie):

### 4.1 `ee25ec2` fix(setup): core libaphrodite dylib best-effort - ALREADY IN DEVELOPMENT

- Partner cites `2768ac8` (refactor(install): move skills dev-side, drop bundled skills, make plugin
  symlink manual) as the Development carrier. Auditor confirms: `git log --oneline -S "BEST-EFFORT" -- crates/aphrodite/src/setup.rs` returns `2768ac8`; `git show 2768ac8:crates/aphrodite/src/setup.rs` contains the best-effort comment, the per-dylib `required` flag (`for &(name, required) in dylib_names`), and the warn-and-continue path (`if required` guard).
- Working tree at HEAD matches: `setup.rs` lines 254-259 (BEST-EFFORT comment), 277 (`required` flag), 337 (`if required`) - identical semantics to `ee25ec2`'s diff.
- **Compiles:** `cargo check -p aphrodite` (auditor's own run) -> `Finished dev profile`, exit 0.
  (Partner's warm run also exit 0.)
- **No regression:** change is additive (optionality), default behavior for the hard-required
  `libaphrodite_hermes` unchanged.

### 4.2 `932a86d` chore(config): refresh templates + drop provider-specific defaults - ALREADY IN DEVELOPMENT

- Partner cites `b028328` (identical message). Auditor confirms the commit exists with that exact
  subject, and the working tree at HEAD carries the change:
    - `config.rs` `api_url`/`model` CLI defaults are `""` (lines 70/74) - DeepSeek defaults gone.
    - `resolve()` API-key fallback chain = `proxy.api_key` -> `defaults.api_key` -> `APHRODITE_API_KEY` only; no `DEEPSEEK_API_KEY`/`HEADROOM_DEEPSEEK_KEY` probe (the only remaining references are `remove_var` calls in a test, lines 653-654).
    - Template keys config_loader reads all present: `poll_worker`, `chain_split`, `previews`,
      `engine_threshold_pct` found in both `crates/aphrodite/templates/aphrodite.toml` and
      `aphrodite.toml.example` (4 hits each); `config_loader.rs` reads `chain_split_*`,
      `engine_threshold_pct`, `poll_worker` - **no key dropped**.
    - Development has since gone further (`c636c74` removed the S2 navigation feature, `fe374d3`
      wired `preview_max_chars`) - picking `932a86d` now would regress those, so SKIP is correct.

### 4.3 `c50a962` / `785b96c` fix(docs): README absolute `tree/Current` URLs - ALREADY IN DEVELOPMENT

- Partner cites `be50538` (identical message to `c50a962`). Auditor confirms `be50538` exists.
- Working tree at HEAD: `crates/aphrodite/README.md` 4 x `tree/Current`, `crates/aphrodite-hermes/README.md` 5 x `tree/Current`, root `README.md` 6 x `tree/Current`; **zero** relative `../../docs` links remain in either crate README (`grep -c` = 0). URLs point at `tree/Current` as required.

### 4.4 Gitlink bump (the only parent-lane write)

- `a613948` diff = exactly `MERGE-SUBMODULE-AUDIT.md` (+77), `MERGE-SUBMODULE-EXEC.md` (+47),
  `plugins/aphrodite` (`160000 d4b426b -> 098134e`). No hidden content; name-status confirms no
  other path touched.

### 4.5 No invalid content anywhere in the range

- Conflict-marker grep over the full range diff: 0 hits.
- Phantom gitlink: 0 hits in the submodule tree and exactly the 3 legitimate `160000` entries in the
  parent HEAD tree.
- No deleted Development files: the range diff contains no deletions of any tracked path.

## 5. Gates

| Gate                                     | Result | Evidence                                                         |
| ---------------------------------------- | ------ | ---------------------------------------------------------------- |
| `cargo check -p aphrodite`               | PASS   | auditor run: `Finished dev profile`, exit 0                      |
| `python3 Maintain/check_ffi_contract.py` | PASS   | 0 violations, 0 warnings (exports 10 / required 7 / argtypes 10) |

## 6. Findings and residual notes

- **No loss, no regression, no conflict, no phantom.** The partner's empty-pick decision is
  independently corroborated: every candidate either already exists in Development (4.1-4.3, with
  Development-only supersession for the config commit), is a gitlink-only bump Development floats
  itself, or fails content inspection (deletes `.hermes`/`Maintain`/tests/bench or touches
  protected workflows).
- Residual (carried from MERGE-PARENT-EXEC.md, not a pick): embedded shim-template drift -
  `crates/aphrodite/templates/__init__.py` is 9 logging-style hunks behind the live `098134e`
  plugin. Functional impact none; correct fix is a Development-lane template sync, not a Current
  cherry-pick. Out of scope for this audit; tracked as follow-up.
- Anonymized: zero local absolute paths in this report (repo-relative only).
