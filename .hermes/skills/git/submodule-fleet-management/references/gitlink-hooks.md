# Gitlink hook machinery: pitfalls, phantoms, and repair (worked catalog)

Reference for the auto-synced gitlink hook set described in SKILL.md
("Always-on-branch + auto-synced gitlink hooks" and "Parent-side-only
variant"). Each item is a worked instance of a rule; the rule itself lives
in SKILL.md. All commands are byte-stable.

## CRLF-ified hook files silently never run

A formatter/prettier pass can CRLF-ify `.githooks/*`. The CRLF shebang fails
exec (`env: bash\r: No such file or directory` in git stderr) and the hook
silently never runs - the auto-bump stops firing and gitlinks go stale
without any visible error.

- Fix with a byte pass: `data.replace(b"\r\n", b"\n")` over `.githooks`.
- Verify with `file` ("ASCII text" without "with CRLF") and
  `head -c 20 <hook> | xxd` on the shebang line. Re-verify with `file` after
  every hook edit, not just at setup.
- DURABLE fix: pin LF in `.gitattributes` with `.githooks/* text eol=lf` and
  `.githooks/lib/* text eol=lf` so checkout keeps LF even under
  `core.autocrlf=true`.
- `git add --renormalize .githooks/` fixes the INDEX but not the working tree
  - git considers the CRLF copy "clean" under autocrlf normalization and
  never rewrites it. Force the rewrite:
  `rm .githooks/* .githooks/lib/* && git checkout -- .githooks/` (the delete
  makes git re-materialize from the index with the new attributes).
- The `.gitattributes` rule is a TRACKED FILE: commit it on EVERY branch that
  uses the hooks. A branch that predates the rule re-CRLFs its hooks on the
  next checkout (`env: bash\r` in git stderr), and the fix on one branch
  does not carry to another - apply + verify per branch.

## Hooks must be EXECUTABLE

A 0644 hook (e.g. post-merge) silently never runs. `chmod +x` the hook files.

## `GIT_DIR`/`GIT_WORK_TREE` override `-C` inside hooks

Git exports `GIT_DIR`/`GIT_WORK_TREE` to hooks, pointing at the repo the
hook runs in - in a submodule that is the SUBMODULE. Any `git -C "$SUPER"
...` call inside the hook then executes against the submodule repo (env
overrides `-C`), and `git add <submodule-path>` fails with `fatal: pathspec
'<sub>' did not match any files` - the auto-bump silently never happens when
git fires the hook, while the same script run by hand works.

`unset GIT_DIR GIT_WORK_TREE` at the top of the bump script (after resolving
`$SUPER`) so git re-discovers each repo from the `-C` directory.

Verify with a forced-stale gitlink test: `git update-index --cacheinfo
160000,<old>,<sub>` then run the script under `GIT_DIR=<sub-gitdir>` - the
index must move back to the new hash with zero fatal noise.

## The auto-committer can UNDO a rollback

After `git reset --hard` cleanup, a later sweep can re-commit the identical
state (same tree + message = same hash), resurrecting the exact mess just
removed. Verify final state AFTER cleanup, not just immediately after the
reset.

## Phantom self-referential gitlinks

An auto-committer sweep run INSIDE a submodule can stage a phantom
self-referential gitlink in the submodule's OWN index: a `160000` entry
named after the submodule path (`plugins/aphrodite`) pointing at the
submodule's own HEAD.

Symptom: submodule `git status` shows `AD <submodule-path>` (staged-add +
worktree-delete), `git diff --cached` shows `new file mode 160000` +
`Subproject commit <own-HEAD>`, and there is no such directory on disk.
Left in place, the next submodule commit records a nested gitlink to
itself.

### A cherry-picked commit can IMPORT a phantom from its source diff

The pick stages whatever the source commit contains, including a
self-referential `160000` gitlink hunk (a commit from a phantom-polluted
line carries it). Drop it with `git update-index --force-remove <path>` then
`git commit --amend --no-edit` (preserves the `-x` trailer), and verify
`git ls-files -s | grep 160000` is EMPTY in the committed tree - not just
the index.

When sync-backing commits from a distributed line that historically had
phantom sweeps, inspect `git show <sha> --stat` BEFORE picking and expect
the phantom hunk; the pick's semantic diff must match the source minus the
gitlink.

### The sweep can also COMMIT the phantom into submodule history

A parent-style "bump <sub> submodule" commit lands INSIDE the submodule repo
whose only delta vs its parent is the phantom gitlink, gets pushed to the
submodule remote, and the parent's gitlink then records that junk commit.
`git status` looks clean (index == HEAD) while the committed tree holds the
`160000` entry - check `git ls-tree HEAD -- <path>` directly, and
`git log --all --oneline` inside the submodule for parent-style messages.

### Repair without content loss

1. Prove the junk commit's only delta is the gitlink with
   `git -C <sub> diff --stat <junk>~1 <junk>` - 1+/1- on the `160000` path
   = safe to drop.
2. In the submodule, `git rm --cached <path>` then `git commit` the removal
   fresh - NEVER `git commit --amend`, which re-commits whatever the
   auto-committer re-staged in the index (the phantom returns even after
   `git rm --cached`).
3. Verify `git ls-tree HEAD -- <path>` is EMPTY (status can be clean while
   the tree still holds it).
4. Re-point the parent gitlink to the clean tip (`git add <sub>` + commit).
5. Force-push the SUBMODULE branch (remote holds only the junk commit -
   verified in step 1) and normal-push the parent.

The auto-committer re-stages the phantom in the index after every removal;
clear it again before any further submodule commit.

## The auto-bump fires from whichever side carries the hooks - pick the side up front

Submodule-side design (hooks in the submodule, gated on
`--show-superproject-working-tree`): only submodule commits/checkouts move
the gitlink, and only when the sweep commits WITH hooks (plumbing/
`--no-verify` commits skip them entirely, which is why stale `+` gitlinks
survive a sweep).

Parent-side design (hooks in the parent, bumping its own gitlinks - what
Aphrodite/Aphrodite-Release now run, see "Parent-side-only variant" in
SKILL.md): parent commits are the trigger, and submodule-side commits change
nothing until the parent records them.

Do not mix the two in one repo pair.

Prove hooks fire at all with
`GIT_TRACE=1 git commit --amend --no-edit 2>&1 | grep -iE 'hook|post-commit|bump'`
and confirm the hooksPath resolves via `git rev-parse --git-path hooks`.

## A repo with NO submodule machinery is not evidence the hook set works

A repo with no `.gitmodules`, zero submodules, and stock LFS hooks holds the
invariant vacuously - compare against a repo that actually exercises it.

## A standalone clone of a submodule repo carries ZERO hook machinery, by construction

`core.hooksPath` is local git config (not version-controlled, never travels
with the clone); the shared `.githooks/` dir lives in the parent and is not
tracked in the submodule; every shared hook script gates on
`git rev-parse --show-superproject-working-tree` being non-empty (exit 0
otherwise). A user who downloads just the plugin repo (e.g. a machine-local
checkout of `plugins/aphrodite`) gets no hooks, no auto-bump, and no errors
- the machinery is a superproject wiring concern, never a dependency of a
standalone clone.

Verify wiring per-clone with `git -C <sub> config --get core.hooksPath` and
`git rev-parse --git-path hooks`; never assume a clone inherits the parent's
hook set.

## The phantom recurs across SIBLING submodules of the same parent

A sweep that stages a self-referential gitlink in one submodule's index does
the same to every sibling (observed: `plugins/aphrodite`, then
`vendor/headroom`, minutes apart). After cleaning one, sweep the rest of the
parent's submodule list (`git submodule status` + per-sub
`git ls-files -s -- <self-path>` for a `160000` entry) before declaring the
fleet clean.

## A parent-side `git checkout <src> -- <sub>` stages a gitlink the WORKTREE still lags

Selective-file checkout from another commit (a release sync, a cherry-pick
file list) updates the index's `160000` entry but leaves the submodule
working tree on the OLD commit - the parent commit is then correct while
`git submodule status` shows `+` and the invariant looks broken. The
committed gitlink is right; fix the worktree AFTER committing, never before
(a pre-commit worktree fix changes the staged SHA and rewrites the intended
float):

```sh
git -C <sub> fetch <remote> -q && git -C <sub> merge --ff-only <remote>/<branch>
```

A `checkout` alone does not move the local branch ref, it only reports
"behind by N"; `merge --ff-only` fast-forwards it. Read the remote name per
checkout first - the same submodule is wired `Source` in one parent and
`origin` in another.

## A fix inside a vendor submodule ships BOTTOM-UP

Commit + push the submodule first, then bump the parent gitlink. CI, publish
pipelines, and consumers resolve the RECORDED gitlink, never the local
working tree - a fix sitting un-pushed in the vendor checkout never reaches
the build, and a bumped gitlink pointing at an unpushed submodule commit
breaks every downstream checkout. Order: `git -C <sub> commit && git -C <sub>
push Source <branch>`, THEN `git add <sub>` + parent commit. This is the
release flow's submodule-first rule (bottom-up topology) applied to a single
in-flight fix.

## A committed phantom breaks downstream clones

`git clone --recurse-submodules` of a superproject whose submodule gitlink
records the phantom commit dies with `fatal: No url found for submodule path
'<path>/<path>' in .gitmodules` - git recursed INTO the submodule and found
a self-referential gitlink whose .gitmodules entry does not exist.

The removal commit must land on EVERY branch consumers gitlink to, not just
the branch being worked: a parent branch still recording the pre-removal
commit (e.g. Development pointed at the sweep commit) keeps breaking clones
until its gitlink is floated to a phantom-free commit (the submodule's own
branch tip, or a recreated removal commit on that branch).

## Disable the hook set for manual gitlink surgery

The auto-bump / force-branch post-checkout hooks fight hand edits (re-bump
the parent, force-checkout the branch). Point `core.hooksPath` at an empty
dir in BOTH repos for the surgery session
(`git config core.hooksPath ~/.hermes/tmp/hermes-no-hooks`), then restore
the original values (`.githooks` / `../../.githooks`) afterwards.

## Do NOT add a parent pre-commit auto-stage of gitlinks

It breaks pathspec commits (`git commit -- <paths>` errors when the index
changes during the hook). Submodule-side post-commit/post-checkout already
cover every submodule HEAD move.

## Parent-side battery, full shape

Verify the parent-side-only variant with a battery, not one smoke test:

- A) submodule advances → parent commit bumps the pointer; `git submodule
  status` shows no `+`.
- B) pointer unchanged → hook no-ops (no commit).
- C) a plain file named like the submodule is never touched (stays
  `100644`).
- D) parent commit with no submodule change → no bump commit.
- E) pre-push bumps a stale pointer before push.

## `IFS=` read split stages EVERYTHING

`while IFS= read -r key path` - `IFS=` (empty) disables read splitting, so
the whole line lands in `key` and `path` is empty - and an empty pathspec
makes `git commit -o -- "$path"` stage EVERYTHING. Use `IFS=' '` explicitly.

## Force-configured-branch post-checkout overrides ad-hoc detaches

The force-configured-branch post-checkout deliberately overrides ad-hoc
detaches in the working copy (user preference: always on the tracking
branch); inspecting old commits needs `git show`/worktrees.