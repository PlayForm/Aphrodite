# MERGE-SUBMODULE.md - Ceremony Phase B sync-back: Current → Development (plugin submodule)

**Date:** 2026-09-18
**Scope:** selective cherry-pick of Current-only commits into Development on the plugin submodule (PlayForm/Aphrodite-Hermes). Submodule sync first (bottom-up) so the parent gitlink can reference the picked plugin. No push, no rebase; stayed on `Development` throughout (Current never checked out).

## Commit disposition

| Current commit                                                     | Date (+0300)     | Decision    | Reason                                                                                                                                                            | Result                                                                        |
| ------------------------------------------------------------------ | ---------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `175c104` release: sync v2.1.3 (test-free)                         | 2026-09-16 19:58 | **SKIPPED** | Test-free release snapshot; Development keeps tests, and Development is already at 2.1.4 (`plugin.yaml`)                                                          | -                                                                             |
| `f8f4cdf` style: Simplify logging statements for clarity           | 2026-09-16 21:16 | **PICKED**  | Small low-risk style change, NOT superseded - all 9 statements still exist on Development in identical multi-line form (verified by grep) despite later refactors | `5fde801`                                                                     |
| `a340e06` chore: remove phantom self-referential gitlink from tree | 2026-09-16 23:35 | **SKIPPED** | `git ls-files -s                                                                                                                                                  | grep 160000` on Development is empty → already clean, removal is a no-op here | -   |

Chronological order (175c104 < f8f4cdf < a340e06) respected; only one pick executed so pick ordering was moot.

## Picked commit (`5fde801`)

- Parent `d4b426b` (Development tip); message carries the `-x` trailer `(cherry picked from commit f8f4cdf93f435ac69d3b28196645d9be1d9dec15)`.
- Diff vs parent: `__init__.py` only - 9 insertions / 25 deletions, exactly f8f4cdf's 9 hunks applied to Development's own context and formatting (line positions differ because Development evolved; semantics = picked side, formatting = repo's own).
- The phantom-gitlink hunk inside f8f4cdf (`plugins/aphrodite`, new file mode 160000 - the auto-committer artifact that a340e06 later removed on Current) was **not** carried over: Development never had it, and applying it would have re-created the phantom. Dropped from the staged set before committing.

## Conflict resolutions

- Cherry-pick auto-merged cleanly; no conflict markers at any stage, no `--continue` required. Post-pick grep of the staged set for `<<<<<<<`: 0 hits.
- The initial `git commit` produced a spurious 4-hunk formatting delta beyond the pick (mechanism unexplained; no local/global hooks, no `core.hooksPath`, no filters beyond LFS). The commit object was rebuilt from the verified staged tree via `git commit-tree` (bypasses all commit machinery), then the branch was reset onto it. Final committed diff verified hunk-for-hunk = the 9 pick hunks only.

## Verification results

| Check                                                                              | Result                                                                                                                                                                 |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Branch stayed `Development`, Current never checked out                             | ✓                                                                                                                                                                      |
| No push, no rebase                                                                 | ✓                                                                                                                                                                      |
| No phantom gitlinks: `git ls-files -s \| grep 160000`                              | empty                                                                                                                                                                  |
| No `.gitmodules` in submodule (any 160000 would be self-referential by definition) | absent                                                                                                                                                                 |
| `python3 -m py_compile __init__.py`                                                | OK                                                                                                                                                                     |
| `npx prettier --check plugins/aphrodite/__init__.py` (parent config)               | exit 0                                                                                                                                                                 |
| Conflict markers in committed diff                                                 | 0                                                                                                                                                                      |
| `-x` trailer on the pick                                                           | present                                                                                                                                                                |
| `tests/` tree intact (release snapshot not picked)                                 | ✓                                                                                                                                                                      |
| `git submodule status` from parent                                                 | `+5fde801… plugins/aphrodite` - parent index gitlink still at pre-pick sha; gitlink bump is the parent lane (branch-owned, floats independently), not coordinated here |

## Files

- `sigserve/MERGE-SUBMODULE.md` (this file)
- `.hermes/notes/MERGE-SUBMODULE.md` (copy)
- Anonymized: zero local absolute paths.
