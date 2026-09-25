# Hotfix on Current - the fast path

Hotfixes are worked DIRECTLY on Current - no Development round-trip, no full
ceremony. The same state machine runs with Current as the workspace: Prepare
→ Validate on Current, Release on Current, then Integrate = sync-back to
Development. The user drives every commit/tag/push decision; never commit,
tag, or push unasked.

### Step H1 - Fix + commit on Current; check the version is free first

**Purpose:** Claim the NEXT patch number, never a burned one.

**Preconditions**

- Orientation gate on Current; HEAD + remote tip captured.

**Do**

```sh
# fix in place on Current; before committing the bump:
curl -A < ua > https://crates.io/api/v1/crates/aphrodite | grep max_version
# bump the binary patch to the next number; commit on Current
```

**Verify**

- `max_version` does not contain the proposed number.

**Stop if**

- The proposed number is already published - claim the next one.

**Recovery**

- Permitted: bump to the next free number.
- Prohibited: reusing a claimed version; retagging an older release.

### Step H2 - BINARY_VERSION bump LAST + gitlink float (same rule as R3)

**Do**

- Bump `BINARY_VERSION` only after the release assets exist (tag pushed AND
  Build completed), then float the parent gitlink to that plugin commit.
  Version bumps ride the release; the plugin submodule's `BINARY_VERSION`
  and the parent gitlink float are the LAST commits of a hotfix cycle.

**Stop if**

- Assets do not exist for the named version (`_check_version_published`
  warns).

### Step H3 - Tag on the release-sync commit only

**Do**

- Run B4 (Step I1) + Gate R7 (Step R2) + the approval pause (Step R4) before
  the tag. A post-release style/cleanup commit does NOT move the tag
  (historical example: plugin v2.1.3 sat on its `release: sync v2.1.3`
  commit, not the reformat commit that followed it).

### Step H4 - Sync-back: selective cherry-pick -x (Current → Development)

**Purpose:** Bring release-line fixes back WITHOUT merging - Development has
diverged with its own work and Current's tree is test-free with
dev-scaffolding absent.

**Preconditions**

- Submodule clean (or phantom cleared); B4 audit clean.

**Do**

```sh
# submodule FIRST (bottom-up rule), then parent:
git log --oneline Development..Source/Current     # in each repo separately
git cherry-pick -x <commit>                        # CHRONOLOGICAL order
```

- PICK: real fixes (setup.rs changes, config-template refresh, docs URL
  fixes, release-notes finalization, version bumps that ride the line).
- SKIP: gitlink-only bumps (`chore: bump plugin submodule to <sha>` - the
  gitlink is branch-owned, Development floats its own), style-only commits on
  files Development has since rewritten, release snapshots that re-add
  content Development deliberately removed (e.g. `directives/`) or delete
  test files (Development keeps tests).
- Conflict resolution: take the PICKED side's SEMANTICS but the repo's own
  formatting (nightly rustfmt: tabs, `space_after_colon = false` -
  patch-tool rustfmt warnings about unstable options are noise, not errors).
- After EVERY `--continue`, grep the staged file set for leftover markers:

```sh
git diff HEAD~1 HEAD --name-only | xargs grep -l '<<<<<<<' || true
# if one slipped in: fix, git add, git commit --amend --no-edit
```

**Stop if**

- An empty cherry-pick (`nothing to commit, working tree clean`) is treated
  as failure - it means the change is already contained; verify with
  `git diff <HEAD> <source> -- <paths>` and skip.
- A second conflict region in the same file survives the first resolution
  (it gets COMMITTED by `--continue`).

**Recovery**

- Permitted: resolve semantics + full-file marker sweep; `--amend` a slipped
  marker.
- Prohibited: merging Current wholesale; rebasing picks; disabling the hooks
  (they are already gone - see pitfalls).

### Step H5 - V1 identity re-expression guard (after any pick touching the gitlink)

**Purpose:** Re-express Development's OWN identity after hotfix picks so a
pick can never transplant Current's identity onto Development.

**Do**

```sh
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite
# (controlled restore of branch-owned identity only - the ceremony invariant,
# NOT a repair mechanism; for corrupted working content never checkout/reset)
git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite # EMPTY (I9)
git submodule status plugins/aphrodite                           # no '+'
git cat-file -e HEAD:plugins/aphrodite/__init__.py               # picked files exist
```

**Expected**

- Identity diffs empty; no `+`; no 160000 phantom in the submodule; every
  picked commit's files exist in HEAD.

**Stop if**

- Identity files differ from Development's baseline; a phantom gitlink
  reappears.

**Recovery**

- Permitted: re-run the controlled restore from Development's baseline;
  clear a phantom with `git -C plugins/aphrodite rm --cached <path>`.
- Prohibited: blanket worktree checkout/reset; ignoring a 160000 entry.

### Step H6 - Verify remote tip, not push output

**Do**

```sh
git log Source/Development # confirm the remote tip
```

**Expected**

- Pushed commits visible on the remote tip - the auto-committer may already
  have pushed as commits landed ("Everything up-to-date" is not evidence).

**Produces**

- Hotfix synced back; B4 re-run after the sync (phase exit).