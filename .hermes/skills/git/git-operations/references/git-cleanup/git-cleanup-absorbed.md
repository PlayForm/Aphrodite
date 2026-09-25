# Git Cleanup: History Surgery & LFS (absorbed from `git-cleanup`)

## Overview

Remove large accidentally-committed files from a Git repository (e.g. a stray
Hermes agent-state dir committed into the `plugins/aphrodite` submodule), reset
history, configure LFS appropriately, and force-push a clean state to remote.

## Decision Matrix

| Situation                                              | Choose                                   | Reason                   |
| ------------------------------------------------------ | ---------------------------------------- | ------------------------ |
| Personal/agent repo, no valuable history               | Fresh `git init`                         | Simple, guaranteed clean |
| Need to keep commit history                            | `git filter-branch` or `git filter-repo` | Preserves ancestry       |
| Many large files scattered across history              | `git filter-repo`                        | More efficient           |
| Delete specific folders from history (e.g., `.hermes`) | **BFG Repo Cleaner**                     | Targeted, fast           |

## Fresh Init Procedure

1. Backup `.git` → `.git.backup`
2. Remove `.git`
3. Create comprehensive `.gitignore` (runtime state, logs, caches, OS artifacts)
4. `git init && git remote add origin <ssh-url>`
5. Configure Git LFS if needed
6. Stage and commit
7. Force push with `git push -u origin <branch> --force`

## BFG Repo Cleaner (Targeted Deletion)

1. Create backup branch
2. `bfg --delete-folders <name> --no-blob-protection`
3. `git reflog expire --expire=now --all && git gc --prune=now --aggressive`
4. Verify cleanup: `git log --oneline -- <folder>` returns nothing
5. Force push

## Submodule Cleanup

After history rewrite on a submodule (e.g. `plugins/aphrodite`, `vendor/headroom`,
`vendor/rtk`), update the parent's gitlink (recorded in `git ls-tree`) and push
both repos. In the Aphrodite monorepo the `.githooks/post-commit` hook
auto-bumps mode-160000 gitlinks, so verify the pointer after the parent commit.
Collaborators must re-clone or run `git submodule update --init --force`.

## LFS Quota Fix

If GitHub blocks push due to exceeded LFS budget:

- **Option A:** Purchase LFS data pack or delete old LFS objects
- **Option B:** Remove LFS tracking, recommit as regular Git
- **Option C:** Delete & recreate remote (nuclear, fastest for personal repos)

## Related References

See `references/git-cleanup/` for session-specific cleanup logs.
See `scripts/git-cleanup/verify-clean.py` for automated repository verification.
