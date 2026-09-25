---
name: fork-pr-branch-sync
description: "Use when syncing fork PR branches to base for clean merges."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos, linux]
category: git
category_taxonomy: git/fork-pr-branch-sync
date: 2026-09-25
metadata:
    hermes:
        tags: [github, pull-requests, forks, merge, credit]
        related_skills: [git-operations, submodule-fleet-management, git-feat-dev-workflow]
status: active
---

# Fork PR Branch Sync

Reconcile an open cross-repo PR whose head branch carries an OLDER or
unhardened version of work that the base has already adopted (hardened),
so the Merge button executes cleanly WITHOUT replacing the base's content
and WITHOUT discarding the PR. The contributor's PR stays open, gets
merged, and keeps their credit.

## The situation

- Base (`Development`/`main`) already contains the adopted+hardened version of
  the contributor's change (e.g. maintainer re-implemented it defensively).
- The PR head still has the contributor's ORIGINAL unhardened version.
- GitHub reports `mergeable: CONFLICTING` / `mergeStateStatus: DIRTY`, or
  the merge would silently revert the hardened work.

Closing the PR unmerged is NOT the answer when the user wants the PR
preserved ("don't discard the PRs") - the fix is to push a sync commit
onto the PR branch.

## Prerequisite gate (read-only)

```bash
gh pr view N --repo owner/repo --json headRefName,headRepository,maintainerCanModify,mergeable,mergeStateStatus
```

- `maintainerCanModify: true` = you may push onto the fork's PR branch.
- `false` = the PR author must enable "Allow edits from maintainers" first;
  without it the push is rejected.
- Also capture the fork remote URL from `headRepository` (the push target
  is the FORK, not the base repo).

## Procedure (per PR)

```bash
cd <repo>
git fetch Source refs/pull/N/head:refs/remotes/prN        # 1. fetch PR head
git checkout -b sync-prN refs/remotes/prN                # 2. local branch at PR head
git diff --stat refs/remotes/prN Source/<base>           # 3. enumerate divergence
git checkout Source/<base> -- <divergent files...>       #    overlay BASE's content
# 4. commit + verify the branch now equals base EXACTLY
git commit -m "chore: sync PR branch with base (adopt hardened implementation)

Co-authored-by: AuthorName <id@users.noreply.github.com>"
git diff --stat Source/<base>                            # must be EMPTY
# 5. push to the fork's PR branch (maintainerCanModify authorizes it)
git push https://github.com/<fork-owner>/<repo>.git sync-prN:<their-branch>
# 6. verify GitHub recomputed mergeability
gh pr view N --repo owner/repo --json mergeable,mergeStateStatus   # CLEAN / MERGEABLE
# 7. cleanup: git checkout <base-branch>; git branch -D sync-prN;
#    git update-ref -d refs/remotes/prN
```

GitHub re-evaluates the PR on push. Head tree == base tree means the merge
button produces a CONTENT-NEUTRAL merge commit: nothing replaced, nothing
reverted, PR closed as merged with the contributor's commits in the merged
history. That is a valid, intentional outcome when the base already
contains the work.

## Hard rules

- **Never `git reset` the PR branch to the base's commit.** That rewrites
  the contributor's history and strips their commits from the PR - the
  opposite of preserving credit. Overlay + a NEW commit keeps their commits
  in the merged history.
- **Never merge the unhardened head** just to "resolve" the PR - it reverts
  the adopted implementation (in one real case it would have removed the
  defensive exception the whole hardening effort added).
- **Overlay the FULL divergent file set**, not just the conflict hunks -
  the goal is an empty `git diff --stat` vs base, which is what GitHub's
  merge check sees.
- Always include a `Co-authored-by:` trailer on the sync commit; the
  contributor's own commits are already in the branch.

## Pitfalls

- Enumerate divergence with `git diff --stat <pr-head> <base>` BEFORE
  overlaying - every divergent file must be covered or the PR stays dirty.
- `git checkout <base> -- <files>` stages base's versions on top of the PR
  head - exactly the "commit directly as if resolving conflicts" flow the
  user expects. Do not attempt a real `git merge <base>` with conflict
  markers unless the branches genuinely diverged beyond the adopted work.
- **Right after the push, GitHub may report `mergeStateStatus: UNSTABLE`
  (status checks re-running) even though the merge is now conflict-free** -
  that is transient, not a failure. Re-query after checks settle; the
  decisive signal is `mergeable: MERGEABLE` + `mergeStateStatus: CLEAN`.
- **The final diff vs the merge-base may NOT be empty even when head ==
  base** - GitHub diffs a PR against the merge-base, which can be an older
  commit; a non-empty `gh pr diff` is not a dirty merge. Judge by
  `mergeable: MERGEABLE` + `mergeStateStatus: CLEAN`, and locally by
  `git diff <base> <sync-branch>` being empty.
- An external auto-committer may sweep mid-operation (parent repo commits
  appear under the account's git identity with generic messages). Never
  fight it; verify final state (`git status`, `git log -1`) after each step.
- The `fatal: pathspec 'plugins/aphrodite'` error seen during submodule
  checkouts is the hook-side GIT_DIR env leak (see
  `submodule-fleet-management`), not a problem with this procedure.
- When the user says "keep current" about a PR, they mean keep the PR's
  OWN content as the author wrote it (not the `Current` branch) - and a
  later "merge no changes is fine" confirms they want the content-neutral
  merge. Verify intent before pushing anything to a fork.
