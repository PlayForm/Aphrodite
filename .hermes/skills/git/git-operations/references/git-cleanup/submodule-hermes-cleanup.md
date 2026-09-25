# Submodule `.hermes` State Directory Cleanup

## Session Trigger

A stray Hermes agent working directory (runtime state: memories, skills, cron,
`state.db`) ended up inside a submodule working tree - e.g. an agent session
was started from inside `plugins/aphrodite` or `vendor/headroom` instead of the
Aphrodite root, and Hermes initialized its runtime home there.

Goal: remove the stray `.hermes` state dir from the submodule working tree
**without touching the root repository's `.hermes`** (which is intentionally
kept) and without nuking legitimate untracked files.

## Scope & Boundaries

- **Target:** the affected submodule only (`plugins/aphrodite`, `vendor/headroom`,
  `vendor/rtk` - each its own independent Git repository)
- **NOT touching:** the Aphrodite root `.hermes/` (Hermes agent's own working
  notes, intentionally kept)
- **NOT touching:** any other submodule's working tree

## Detection

1. Find every `.hermes` directory across the monorepo:

    ```bash
    find . -type d -name .hermes -not -path './.git/*' 2> /dev/null
    # Example hit: ./plugins/aphrodite/.hermes
    ```

2. Gather git status evidence inside the submodule:

    ```bash
    git -C plugins/aphrodite status --porcelain
    # "?? .hermes/" means the state dir is UNTRACKED - safe to delete from disk
    ```

## Tracked vs Untracked Decision

```bash
git -C plugins/aphrodite ls-files .hermes
# empty        -> untracked: remove from disk only (rm -rf)
# lists files  -> TRACKED: it was committed - do NOT rm -rf; use git rm -r
#                 and commit, or purge from history with BFG (below)
```

**Rule:** only ever remove an _untracked_ `.hermes` with plain `rm -rf`. If it
is tracked, removing the working-tree copy without removing the index entries
leaves a dirty state that the external auto-committer immediately re-commits.

## Removal (untracked only)

```bash
rm -rf plugins/aphrodite/.hermes
git -C plugins/aphrodite status --porcelain # should be clean again
```

**Never** run `git clean -fdx` blindly to "fix" this. In the Aphrodite monorepo
that nukes every untracked file in the submodule - build outputs under
`crates/*/target`, the plugin's local build artifacts, and any other agent
state - and `-x` also removes `.gitignore`d files. If you must use `git clean`,
scope it: `git clean -fd plugins/aphrodite/.hermes` (still only after the
untracked check above).

## If It Was Committed (history purge)

If `git ls-files .hermes` lists files, the state dir is in history. Purge it
from the submodule's history without touching the root repo:

```bash
cd plugins/aphrodite
git branch backup-hermes-clean-$(date +%s) # safety branch
bfg --delete-folders .hermes --no-blob-protection
git reflog expire --expire=now --all
git gc --prune=now --aggressive
git log --oneline -- .hermes # should be empty
git ls-files | grep hermes   # should be empty
```

Then update the parent: the Aphrodite root's gitlink still points at the
pre-BFG commit. Push the cleaned submodule (`git push origin --force --all`)
and commit the pointer bump in the parent (`git add plugins/aphrodite`). The
parent's `.githooks/post-commit` auto-bumps mode-160000 gitlinks, so verify
the recorded pointer with `git ls-tree HEAD plugins/aphrodite` after
committing.

## Verification

```bash
git -C plugins/aphrodite status --porcelain               # clean
git -C plugins/aphrodite log --oneline -- .hermes         # no output (if purged)
test -d plugins/aphrodite/.hermes && echo "still on disk" # absent
git ls-tree HEAD plugins/aphrodite                        # pointer matches pushed SHA
```

## Aphrodite-Specific Notes

- **Auto-committer:** an external auto-committer sweeps working-tree changes;
  if it is active, capture `git rev-parse HEAD` + `git ls-remote origin
<branch>` before starting, and re-check at every boundary.
- **Submodule remotes:** `plugins/aphrodite` uses git remote `Source`;
  `vendor/headroom` and `vendor/rtk` are vendored upstreams - their histories
  are not force-pushed casually.
- **Never `git clean -fdx` blindly** - see above.
- The root repo's `.hermes` is intentional: the recursive `find` evidence must
  show the stray dir is inside a submodule before touching anything.

## Related

- `git-operations` - umbrella skill for Git repository cleanup operations
- Aphrodite `AGENTS.md` - repo facts (submodule inventory, hook conventions)
