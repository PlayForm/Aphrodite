# MERGE-SUBMODULE-EXEC.md - Ceremony Phase B sync-back execution record (plugin submodule)

**Date:** 2026-09-18
**Scope:** selective cherry-pick of the single safe Current-only commit (`f8f4cdf`, logging style) into Development on the plugin submodule. Submodule sync first (bottom-up). No push, no rebase, no `merge --squash`; stayed on `Development` throughout (`Current` never checked out).

## Commit disposition

| Current commit                                                     | Decision    | Reason                                                                                                                                    | Result    |
| ------------------------------------------------------------------ | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `175c104` release: sync v2.1.3 (test-free)                         | **SKIPPED** | Deletes the 5 test files, `_bindings.py`, `layout_check.py`, `layout_schema.json`; re-adds `directives/`. Development keeps all of these. | -         |
| `f8f4cdf` style: Simplify logging statements for clarity           | **PICKED**  | Pure logging-style simplification, not present in Development after the earlier rollback. Safe low-risk pick.                             | `098134e` |
| `a340e06` chore: remove phantom self-referential gitlink from tree | **SKIPPED** | `git ls-files -s \| grep 160000` on Development is empty -> already clean, removal is a no-op here.                                       | -         |

## Picked commit (`098134e`)

- Cherry-pick command: `git cherry-pick -x f8f4cdf` (auto-merged cleanly; no conflicts).
- Message carries the `-x` trailer `(cherry picked from commit f8f4cdf...)` - verified present (1 hit).
- **Phantom-gitlink hunk dropped:** f8f4cdf's diff also contained `plugins/aphrodite | 1 +` (a `160000` self-referential gitlink entry that a340e06 later removed on Current). The raw pick staged that entry; it was removed from the staged set with `git update-index --force-remove plugins/aphrodite` and the commit rebuilt via `git commit --amend --no-edit` (keeps the `-x` trailer). Development's tree now has no phantom, in index or in the committed tree.
- Final committed diff vs parent: `__init__.py` only - 9 insertions / 25 deletions (exactly f8f4cdf's 9 hunks applied to Development's own context/formatting; semantics = picked side, formatting = repo's own).

## Conflict resolution

- Cherry-pick auto-merged; no conflict markers at any stage, no `--continue` required.
- Post-pick grep of the staged set for `<<<<<<<`: 0 hits. Committed diff grep for `<<<<<<<`: 0 hits.
- The phantom-gitlink hunk was resolved by dropping it (Development never had a phantom; carrying it would have re-created one).

## Verification results

| Check                                                                                                                                          | Result                                                                                                                                                 |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Branch stayed `Development`, `Current` never checked out                                                                                       | OK                                                                                                                                                     |
| No push, no rebase, no `merge --squash`                                                                                                        | OK                                                                                                                                                     |
| `git submodule status` (from parent): no `+` (I2)                                                                                              | OK - `098134ea... plugins/aphrodite (v2.1.2-47-g098134e)`; parent gitlink staged (not committed, not pushed) - the commit is the parent lane's B2 step |
| Phantom check: `git ls-files -s \| grep 160000`                                                                                                | empty                                                                                                                                                  |
| Phantom check in committed tree: `git ls-tree HEAD -- plugins/aphrodite`                                                                       | empty                                                                                                                                                  |
| `git diff HEAD~1 HEAD --stat` shows ONLY `__init__.py` (nothing else touched)                                                                  | OK - 1 file, 9 insertions / 25 deletions                                                                                                               |
| 5 test files present (test_dylib_candidates, test_hotreload_cleanup, test_perf_probe, test_reaper_prefix_contract, test_windows_process_state) | OK                                                                                                                                                     |
| `_bindings.py`, `layout_check.py`, `layout_schema.json` present and unchanged (not in pick diff)                                               | OK                                                                                                                                                     |
| `directives/` still absent (not resurrected)                                                                                                   | OK                                                                                                                                                     |
| `python3 -m py_compile __init__.py`                                                                                                            | OK                                                                                                                                                     |
| `npx prettier --check plugins/aphrodite/__init__.py` (parent config)                                                                           | exit 0 - all matched files use Prettier code style                                                                                                     |
| `-x` trailer on the pick                                                                                                                       | present                                                                                                                                                |

## Files

- `.hermes/notes/release/MERGE-SUBMODULE-EXEC.md` (this file)
- Anonymized: zero local absolute paths.
