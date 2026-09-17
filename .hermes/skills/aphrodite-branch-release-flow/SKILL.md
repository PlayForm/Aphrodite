---
name: aphrodite-branch-release-flow
description: "Use when syncing Development to Current or hotfixes up."
version: 1.1.0
platforms: [macos]
tags: [aphrodite, release, branch, cherry-pick, submodule]
---

# Aphrodite Branch Release Flow

One-line policy:

> Development builds the next release. Current contains the last released
> state. Tags identify published Current commits. Current hotfixes are
> cherry-picked back to Development.

The two-branch model for PlayForm/Aphrodite (parent P) and its plugin
submodule (S = `plugins/aphrodite`, repo PlayForm/Aphrodite-Hermes):

- **Development** - the workshop. All work, version bumps, and preparation
  happen here. STATIC: accumulates, never rewritten (rebase banned).
  Never carries release tags.
- **Current** - the distributed branch. People download it; it is GitHub's
  default. Tags and releases live ONLY here. Bi-directional: receives
  releases from Development; hotfixes born here get picked up to Development.

The two branches are intentionally different products, not two branches that
must merge cleanly: workflow triggers, `.gitmodules` branch fields, and the
submodule pin are branch-owned (deliberate divergence). A normal
merge-based promotion would therefore always carry conceptual friction.
Releases are **snapshot transplants**, not merges.

## Protected paths (I9) - NEVER cross Development → Current

During any sync-down, these stay Current-owned:

1. `.gitmodules` - branch field must say `Current` on Current,
   `Development` on Development.
2. `.github/workflows/*` - triggers are branch-specific
   (`branches: [Current]` on Current, `[Development]` on Development).
   Never fan out a workflow to both branches.
3. `plugins/aphrodite` (the gitlink) - must point at S-Current's tip,
   never at a Development-only commit.

`Maintain/scripts/release/auto-release.sh` is branch-aware (derives
RELEASE_BRANCH from HEAD). Run release PREPARATION on Development with the
bump/build/test half; the TAG step is deferred to Current (below).

## Invariants (verify before declaring done)

- **I1** Both repos on a named branch (hooks that used to float on detach are REMOVED 2026-09-17 - branch discipline is manual).
- **I2** `git submodule status` never shows `+`; `git status` never shows
  `M plugins/aphrodite`.
- **I3** P-Current tree == the tagged released content; S-Current == the
  released plugin.
- **I4** Development is static/accumulating: no rebase, no reset, no
  history rewrite.
- **I5** `.gitmodules` branch field matches the branch name on each branch.
- **I6** Workflow triggers match the branch: `[Development]` on
  Development, `[Current]` on Current.
- **I7** Every hotfix pick carries `-x` + a manifest entry.
- **I8** Versions monotonic: bump where the change ships; picks propagate
  the bump; never reuse a claimed number.
- **I10** The working copy lives on Development; Current is touched only
  during the sync ritual and returned-from immediately.

## Promotion mechanism - decide per need

A promotion branch (`promote/vX.Y.Z`) is OPTIONAL review scaffolding, NOT
architecture. Both `Current` branches are unprotected, so direct promotion
works; use the PR branch only when review is actually required.

| Need                  | Best mechanism                                                      |
| --------------------- | ------------------------------------------------------------------- |
| Maximum simplicity    | Promote directly on Current (clean tree + checks)                   |
| Mandatory code review | `promote/vX.Y.Z` from Current, one snapshot commit, PR, then delete |
| Urgent production fix | Commit on Current, then `git cherry-pick -x` into Development       |
| Release candidate     | Prerelease tag (`Aphrodite/v1.4.3-rc.1`), never a branch            |

Do NOT cherry-pick normal feature commits from Development down into
Current - it duplicates SHAs and makes every release a series of conflict
decisions. A release snapshot moves the completed release as one
intentional unit.

## Release ceremony (sync-down + tag, order matters)

### 1. Prepare on Development (no tags)

1. Work lands on P-Development and S-Development.
2. Bump versions: binary 1.4.x in P (`crates/aphrodite/Cargo.toml`,
   `crates/aphrodite-hermes/Cargo.toml` package+dep, `package.json`,
   `plugins/aphrodite/BINARY_VERSION`, README badges); plugin 2.1.x in S
   (`plugin.yaml` version + install_message).
3. Gates: `cargo clippy -p aphrodite -- -D warnings`,
   `ruff check plugins/aphrodite/`, `cargo deny check advisories`,
   `python3 -c "import aphrodite"` (from repo root with plugins on path).
4. Build + test: `cargo build --release -p aphrodite -p aphrodite-hermes`,
   `cargo test -p aphrodite -p aphrodite-hermes`.
5. Commit the bump on Development. Do NOT tag here.

### 2. Sync plugin first (S-Dev → S-Current)

The parent's gitlink must reference the released plugin, so the plugin
sync always precedes the parent sync:

1. `git -C plugins/aphrodite checkout Current`
2. `git -C plugins/aphrodite merge --squash Development`
3. Verify no protected paths exist in S (it has no submodules, no
   branch-triggered workflows to diverge).
4. Commit `release: sync v2.1.x` in S, push S-Current.
5. Tag `v2.1.x` on S-Current's sync commit, push the tag.

### 3. Sync parent (P-Dev → P-Current) + tag

1. `git status` MUST be clean (else the auto-committer sweeps dirt into
   the release - abort).
2. `git checkout Current && git pull --ff-only Source Current`
3. `git merge --squash Development` (stages full diff, no merge commit)
4. Restore protected paths from HEAD:
   `git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite`
5. Float submodule to S-Current tip (parent checkout does NOT auto-update
   it):
    ```
    git -C plugins/aphrodite fetch Source Current
    git -C plugins/aphrodite checkout -B Current Source/Current
    git add plugins/aphrodite
    ```
6. Verify: `git diff HEAD -- .gitmodules .github/workflows` is empty;
   `git submodule status` shows no `+`; S branch is Current.
7. Commit `release: sync v1.4.x from Development`, push P-Current.
8. Tag `Aphrodite/v1.4.x` on P-Current's sync commit, push the tag.
9. Create the GitHub release from the tag (notes via --notes-file, never
   inline backticks). Build.yml fires on `refs/tags/Aphrodite/*` -
   branch-agnostic, so tagging Current works.
10. `git checkout Development` (restore the safe working copy).

**PR variant** (when review is mandatory): skip steps 7-8; instead
`git checkout -b promote/vX.Y.Z` from Current before step 3, do steps 3-6
on the branch, push it, open `promote/vX.Y.Z -> Current`, merge, tag on
Current, delete the branch.

## Hotfix on Current (bi-directional) - pick up to Development

1. Fix on P-Current; bump the patch (claim the number - I8). Commit.
   If it ships now: tag on Current + release, exactly as step 3 above.
2. Pick up to Development:
   `git checkout Development && git cherry-pick -x <sha>` - the bump
   rides along, so Development's version matches.
3. **V1 guard**: if the pick touched a protected path (`.gitmodules`,
   `.github/workflows/*`), re-express the branch identity after the pick:
   `.gitmodules` back to `branch = Development`; workflow triggers back to
   `[Development]`. Diff the protected set and restore before committing.
4. Append to the picks manifest `.hermes/picks/` (source sha, picked sha,
   date, reason).
5. Coordinated P+S hotfix: commit + push S-Current FIRST (so the P-Current
   commit can reference the new S gitlink), then P; pick S up first, then P.

## Variant traps

- **V2** Current diverged because a pick was missed: sync-down squash
  conflicts on a file both branches changed - resolve keeping Current's
  hotfix content, then immediately pick that hotfix up. Development must
  always contain Current (superset), so sync-down is a clean copy.
- **V3** Submodule detach: hooks used to float to the configured branch +
  bump the gitlink automatically; the hooks are REMOVED 2026-09-17, so a
  detach is now handled manually (`git -C plugins/aphrodite checkout
  Development`).
- **V4** Auto-committer races: it commits on the checked-out branch.
  Working copy stays on Development (I10); Current is only touched inside
  the ritual.
- **V5** Tags live on Current only. A Current clone sees them via
  `git describe`; Development never carries release tags.
- **V6** Sync stages Development's S-gitlink; step 4 restores it, step 5
  floats to S-Current tip. Never skip either.
- **V7** Bump files live on Development; the sync carries the new version
  down. Never bump on Current except hotfix-claims.
- **V8** Abort the sync if the tree is dirty - the sweeper would commit
  unreviewed dirt into the release commit.

## Related

- `branch-flow-protocol` - the general protocol this skill is the first
  concrete instance of (axioms, state machine, verifier).
- `aphrodite-release-workflow` - version-sync locations, release notes
  standards, crates.io publishing.
- `aphrodite-development-lessons` - dev session setup, hook pitfalls.
