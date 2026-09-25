---
name: submodule-fleet-management
description: "Use when resetting, syncing, or keeping Aphrodite's submodule fleet (plugins/aphrodite with remote 'Source', vendor/headroom, vendor/rtk) on-branch with auto-synced gitlinks (hooks)."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: git
category_taxonomy: git/submodule-fleet-management
date: 2026-09-25
scope:
    repositories: [PlayForm/Aphrodite]
owns: "fleet submodule working trees, local branch names, and remote URL configs; gitlink bumps are staged, never committed (the pointer commit is the user's)"
depends_on: [git-operations]
supersedes: []
verification:
    source_of_truth: "scripts/verify-fleet-sync.sh; per-submodule `git submodule status` (no `+` prefix)"
mutation_level: high
metadata:
    hermes:
        tags: [git, submodules, monorepo, fleet, reset, automation]
        related_skills: [git-operations]
status: active
---

# Submodule Fleet Management

**Class:** operating on _many_ git repositories at once - a directory of submodules,
vendored dependencies, or documentation-reference clones treated as one fleet. Covers
pristine resets, keeping local branch names identical to upstream defaults, remote URL
normalization, roster/coverage auditing, and safe destructive-operation discipline.

This skill covers the read-only mirror side of the PlayForm/Aphrodite monorepo:
`plugins/aphrodite` (git remote `Source`), `vendor/headroom`, `vendor/rtk`. Distinct from
`git-operations` (single-repo history cleanup) and from `git-feat-dev-workflow`
(authoring contributions): here the repos are **read-only mirrors** - nothing is authored
locally, so "make it exactly match upstream" is the goal.

References:

- `references/submodule-fleet-reset/reset-and-branch-sync.md` - full reset recipe: default-branch resolution function, branch-name sync, origin-only sweep, audit-before-reset probes, driver parallelism, verification.
- `references/submodule-fleet-reset/shallow-depth-truncation.md` - truncating fleet repos to shallow depth N.
- `references/gitlink-hooks.md` - the auto-synced gitlink hook machinery (submodule-side and parent-side designs), phantom-gitlink catalog, CRLF/GIT_DIR pitfalls, repair procedures.
- `references/cleanup-discipline.md` - repair-in-place recovery, scrub of distributed branches, backup before scrub.
- `references/remote-hygiene.md` - fleet remote repair: the empty-Parent bug, the `null`/`null` sentinel, the URL sweep.
- `references/formatter-parity.md` - byte-identical mirrors between editor and CI.
- `references/user-repo-conventions.md` - the user's remote/branch conventions (`Source`, `Parent`, `Current`).
- Re-runnable verifier: `scripts/verify-fleet-sync.sh` (plus `scripts/verify-fleet-depth.sh` for nested-submodule depth coverage).

## Core rules

1. **Never assume `main`. Resolve each default branch from the remote.**
   `git ls-remote --symref origin HEAD` is authoritative; fall back to cached
   `refs/remotes/origin/HEAD`, then a candidate list (`main master trunk production
   develop`). Hardcoding `main` silently mangles repos - one 33-repo fleet contained
   `production`, `master`, and `v2` defaults. Cache the answer with
   `git remote set-head origin <default>`. Worked function: reset-and-branch-sync.md §1.
2. **Sync the branch NAME, not just the commit.** Tracking the right commit on a
   differently-named local branch is still out of sync. Force the local branch to carry the
   upstream name: `git checkout --force -B "$default" "refs/remotes/origin/$default"`, then
   `branch --set-upstream-to`. Custom local nomenclature (a `Current` branch, etc.) gets
   replaced by the upstream name.
3. **Local-only branches are deleted by default.** That is what actually keeps names
   mirrored. Provide `--keep-local-branches` as the escape hatch - do not make deletion
   opt-in, or drift silently persists.
4. **`origin` is the only remote.** Fleets accumulate strays (`Source`, `Parent`, malformed
   URLs). Sweep every remote that is not `origin`. Also sweep for submodules with NO
   remote at all: a missing `origin` makes `git fetch origin` fail with
   _"does not appear to be a git repository / access rights and the repository exists"_
   (a nonexistent remote NAME falls back to path resolution) - that error does NOT mean
   the fork is unreachable. Probe `git -C <sub> config --get remote.origin.url` first;
   wire origins from the `.gitmodules` URLs (`git remote add origin <url>`) then fetch.
5. **Honor the user's URL scheme uniformly.** If the user standardizes on
   `ssh://git@github.com/...`, convert **every** GitHub remote and assert the negative
   afterward (zero `https://github.com` survivors). Non-GitHub hosts (codeberg, gitlab) keep
   their own scheme - do not rewrite a host the rule does not name.
6. **Audit before any `--hard` reset. Classify, do not count.** Raw dirty/unpushed counts
   mislead: thousands of "modifications" may be staged _deletions of upstream files_ (a
   reset restores them); "unpushed commits" may be entirely upstream authors on a stale
   tracking ref. Probe with `status --porcelain --untracked-files=no`,
   `log --branches --not --remotes --format='%an'`, and `merge-base --is-ancestor`. Save ref
   tips to `~/.hermes/tmp/` as evidence first. Only reset once you can state that nothing was
   authored locally. Worked probes: reset-and-branch-sync.md §Audit.

Stop if: `status --porcelain --untracked-files=no` shows tracked modifications you have not
classified as staged deletions of upstream files. Recovery: classify with
`log --branches --not --remotes --format='%an'` and `merge-base --is-ancestor`; only reset
when nothing was authored locally.

## Detached HEAD from the superproject gitlink

A superproject tracks each submodule by **gitlink**. A gitlink is a pinned commit SHA
recorded in the superproject's index. It is not a branch. `git submodule update` checks the
submodule out **detached** at that SHA, even when its branch has moved ahead. That detached
HEAD is a hanging-head hazard: a commit made while detached dangles off no branch and is
easy to lose.

Keep a submodule on its branch and make it durable:

```sh
git -C <submodule-path> checkout <branch>   # e.g. Current
git add <submodule-path>                     # stage the gitlink bump, no commit
```

Staging the gitlink is what makes it stick. Staging records the new pointer in the index;
it is not a commit. Without the stage, the next `git submodule update` snaps the submodule
back to the stale detached SHA. The pointer commit itself stays the user's to make (see
Reporting).

Read state with `git submodule status`: a leading `+` means the submodule is ahead of the
recorded gitlink; no prefix plus `(heads/<branch>)` means it is pinned on the branch tip.

**`ignore = all` in `.gitmodules` (and a gitignored submodule path like `.config/`) makes
`git add <sub>` silently SKIP the gitlink update** - git prints "Skipping submodule due to
ignore=all". Stage with `git add -f <sub>` instead. Read the RECORDED gitlink with
`git ls-tree HEAD <sub>` and compare to the checkout: equal means only the checkout drifted
(a reset fixes it); recorded older than fork tip means a real pointer advance - confirm
fast-forward with `git -C <sub> merge-base --is-ancestor <old> <new>` before recording it.

### Rescuing a dangling auto-commit before any reset

Auto-committers / background watchers commit on the detached HEAD (submodule checkouts are
detached by design), producing a commit referenced by NO branch - it dangles the moment the
next `submodule update` / superproject checkout moves HEAD back to the gitlink SHA. Before
ANY reset, check for a rescue:

```sh
git -C <sub> symbolic-ref -q HEAD       # fails => detached
git -C <sub> branch --contains HEAD     # empty => the commit is unreferenced
```

Both true means a swept commit is at risk. Rescue it by fast-forwarding the branch to it
(pure fast-forward when it descends from the branch tip - check `git merge-base
--is-ancestor <old> <new>` first), then check out the branch:

```sh
git -C <sub> branch -f Current <sha> && git -C <sub> checkout Current
```

Afterward `git branch --contains <sha>` must list the branch. The reflog (~90 days) and a
superproject gitlink committed at the new SHA are secondary safety nets, not guarantees.
Enforce the invariant at the source with a pre-commit branch guard
(`git symbolic-ref -q HEAD || exit 1`) wired into the submodule's OWN hooksPath - the
superproject's hooks do not run for submodule commits.

## Always-on-branch + auto-synced gitlink hooks

The manual rescue above is the fallback; the durable fix is a hook set in ONE shared
`.githooks/` dir that BOTH repos point at (hooksPath is per-repo - the submodule needs
`git config core.hooksPath ../../.githooks` relative to ITS top-level, and
`git rev-parse --git-path hooks` should resolve it). A single hook file then serves
both repos: gate every hook on `git rev-parse --show-superproject-working-tree` being
non-empty so it no-ops in the plain repo.

Invariant: BOTH repos stay on their CONFIGURED tracking branch, and
`git submodule status` / `git status` never show `+` / `M <submodule>`.
The configured branch is `submodule.<name>.branch` in the superproject's
`.gitmodules` - historically `Current` for every submodule, but the parent
may point a submodule at a different branch (`Development` for a
bump/test track while `Current` stays the stable download branch). The
hooks must read that per-submodule branch, never hardcode `Current` -
hardcoding yanks a Development-track submodule back to Current on every
checkout, silently defeating the split track.

- `pre-commit`: refuse detached-HEAD commits (`git symbolic-ref -q HEAD || exit 1`).
- `post-commit`: bump the parent's gitlink to the new HEAD.
- `post-checkout`: if detached and the submodule's CONFIGURED branch
  (from `.gitmodules`) exists, check it out (terminates - the re-entrant
  post-checkout sees an attached HEAD and skips), then bump the gitlink.
  Resolve the configured branch by matching the submodule's relative path against
  `.gitmodules` `submodule.*.path` entries, read `submodule.<name>.branch`, fall back to
  `Current` when unset.
- `lib/bump-submodule-gitlink.sh` (shared): detect the superproject, compute the
  relative submodule path with python3 `os.path.relpath` (GNU `realpath
  --relative-to` does not exist on macOS), skip while the parent holds
  `MERGE_HEAD`/`CHERRY_PICK_HEAD`, and commit `chore: bump <sub> to <short>` ONLY
  when `git -C <super> ls-files -s -- <sub>` differs from the submodule HEAD.

Why this works, as a chain:

1. Input: `git submodule update` checks out the recorded gitlink DETACHED. Output: the
   submodule's post-checkout hook fires - that is the heal trigger point.
2. Input: detached HEAD + the configured branch present. Output: the hook checks out the
   branch; the re-entrant post-checkout sees an attached HEAD and skips.
3. Input: branch checkout. Output: the bump re-records the pointer in the same operation,
   so the detached state never survives a single command.

Verify the set end-to-end before declaring it done (a battery, not one smoke test):

1. Input: submodule commit. Output: parent auto-bump, no `+`.
2. Input: rewind the parent's recorded pointer (`git update-index --cacheinfo
   160000,<old>,<sub>` + commit) then `git submodule update`. Output: heals to
   `Current` + re-bumps.
3. Input: parent-side commit. Output: submodule untouched.
4. Repeat cycles: each bump is a fresh commit, no accumulation.

Stop if: any battery step fails. Recovery: fix the failing hook (executable bit, CRLF
shebang, `GIT_DIR`/`GIT_WORK_TREE` override - see `references/gitlink-hooks.md`), then
re-run the battery from step 1.

Pitfall catalog - CRLF-ified hooks, `GIT_DIR`/`GIT_WORK_TREE` env override, phantom
self-referential gitlinks, cherry-pick import, standalone clones, sibling submodules,
bottom-up vendor fixes: `references/gitlink-hooks.md`.

## Parent-side-only variant (current design in Aphrodite / Aphrodite-Release) [CLAIM]

The submodule-side design above fails when the submodule's own hooks are
unreliable (auto-committers committing with plumbing/`--no-verify` skip them;
phantom self-referential gitlinks leak into submodule indexes) and when the
user wants the trigger to be parent actions. The repos now run the hooks in
the PARENT only: `core.hooksPath = .githooks` set in the SUPERPROJECTS; a
shared `lib/bump-submodule-gitlinks.sh` plus hooks (`post-commit`,
`post-merge`, `post-checkout` branch-only via `$3 == 1`, `pre-push`) live in
the parent's `.githooks/` and commit the parent's OWN gitlinks on parent-side
actions. The parent needs no submodule-side wiring at all. [CLAIM: "now run"
is live state; probe `git -C <super> config --get core.hooksPath` and
`git rev-parse --git-path hooks` in the parent.]

Submodule repos in this fleet must NOT carry their own `.githooks/`
(standing user rule: "only updates to the gitlink are necessary from the
parent repository") - enforcement and gitlink updates flow parent-side only,
and a child's `.githooks` is dead weight that misleads standalone clones
(they can never fire it: hooksPath is local config, the dir is untracked in
the child). Removing one: `git rm -r .githooks` in the child +
`git config --unset core.hooksPath` (left dangling at a removed dir,
hooksPath silently disables ALL hooks in that repo), commit + push the
child, then bump the parent gitlink.

Trigger chain:

1. Input: a parent-side action (commit / merge / checkout / push). Output: the parent hook
   commits the parent's own gitlink for any dirty submodule path - the pointer is never
   orphaned.

Guards in the shared lib (ALL must pass before any commit):

1. Superproject guard: only run when the current repo IS the parent (its own
   `.gitmodules` declares submodules) - no-ops in vendor/submodule checkouts.
2. Real-gitlink-only: bump candidates are only paths recorded as mode `160000`
   in the parent's index (`git ls-files -s -- <path>`) - a plain directory or
   file that merely shares a submodule's name is never staged. This prevents
   phantom gitlinks by construction.
3. Recursion guard: `GITLINK_BUMP_ACTIVE` env var - the hook's own `git commit`
   re-fires `post-commit`; without the guard it loops forever.
4. Mid-operation skip: no bump while the parent holds `MERGE_HEAD` /
   `CHERRY_PICK_HEAD` / is mid-rebase - never commit inside a merge or pick.
5. No-op when unchanged: only commit when `git ls-files -s -- <sub>` differs
   from the submodule's current HEAD.

Commit shape: surgical `git commit -o -- "${DIRTY[@]}" --no-verify -s` (only
the dirty gitlink pathspecs; `-o` keeps everything else out of the commit;
`--no-verify` prevents hook re-entry; `-s` signs like the auto-committer),
wrapped in `|| true` so a transient index lock (auto-committer holding the
index) converges on the next sweep instead of failing the user's real commit.

Verify with a battery, not one smoke: A) submodule advances → parent commit
bumps the pointer, `git submodule status` shows no `+`; B) pointer unchanged
→ hook no-ops (no commit); C) a plain file named like the submodule is never
touched (stays `100644`); D) parent commit with no submodule change → no
bump commit; E) pre-push bumps a stale pointer before push. Full pitfalls
(`IFS=` read split, `GIT_TRACE=1` hook-firing probe, `GIT_DIR` override):
`references/gitlink-hooks.md`.

## Resolving a submodule merge conflict (parallel implementations)

Auto-committers on both sides often implement the SAME feature independently, so the
merge shows "both modified" on the same files. Resolution rules:

- Scope both sides first - one is usually a SUPERSET (extra commands/tests). Take the
  superset's implementation; keep the other side's text only where it documents
  something the superset misses (e.g. accurate legacy-behavior notes).
- Dedupe everything both sides added twice: guards, tests, env-var entries, identical
  gitlink bumps. Check `plugin.yaml`/manifest-style files too - both sides append the
  same line.
- Scripted marker-block replacement (`<<<<<<<` ... `>>>>>>>`) must end the resolution
  text with the FILE's line terminator: a missing trailing CRLF glues the next code
  line onto the resolution, often INTO a comment (silently swallowing a definition or
  kwarg - py_compile still passes, the NameError surfaces only at import/test time).
  Use a script that asserts exactly one occurrence per block and re-reads file bytes
  (mixed CRLF/LF lines are normal after a merge).
- The fuzzy patch tool re-indents multi-line blocks on CRLF/tab-indented files; after
  two mangled attempts on the same region, switch to an exact-byte replacement script.

## Cleanup discipline (standing user rules)

- **NEVER run `git rebase` - any form, any repo, ever** (hard prohibition).
  History cleanup is `git reset`/merge/commit-forward only; if removing a run of
  commits requires rebase, accept the stale commits or reset to a kept ancestor
  and re-commit the pointer forward.
- **`git push --force` is authorized WHEN NEEDED, but verify first that the remote
  holds nothing real:** `git fetch Source` then `git log Source/Current..Current`
  must show the remote-only commits are pure descendants of local (junk sweeps,
  test artifacts) before force-pushing. Never force over unverified remote work.
- **Rollback verification:** after `git reset --hard` cleanup of a mess, re-check
  the final state AFTER the auto-committer has had time to sweep - it can
  re-commit an identical state (same tree+message = same hash) and resurrect the
  exact mess, and a later `git submodule update` can re-detach the submodule at
  the stale recorded pointer. One clean `git status` immediately after the reset
  proves nothing.
- **Never `git reset`/`git checkout` a file to "restore" it - repair in
  place (hard user rule).** `git checkout -- <file>` restores from the INDEX, not
  HEAD, so with an auto-committer that stages working-tree edits the checkout is a
  NO-OP (it returns the staged version and the file still "contains your edits";
  grep finds the symbol, `git status` shows nothing). When a file's CONTENT is wrong,
  git-restoring throws the intended work away - a corrupted write still contains that
  content (e.g. escaped newlines are recoverable data). Repair in place instead:
  `read_file` the file, find the improper section, fix exactly that with
  `patch`/`write_file`, preserving every intended row. If the working tree was ALREADY
  reset, recover the original blob from the object store first: `git fsck --lost-found`,
  match by size (`git cat-file -s <hash>`), extract (`git cat-file blob <hash> > file`),
  repair, re-write. When `git status` shows `MM` (index holds stale text, working tree
  holds the fix), align the index with `git add` - a forward action, never
  reset/checkout. This rule governs files carrying intended content; the read-only mirror
  fleet resets below (nothing authored locally) are a different operation.
- **Before dropping a commit as "test junk", locate the content you want to keep**
  with `git log --oneline -- <path>` - the auto-committer bundles hook
  refactors/real fixes into unrelated-looking "bump"/"chore" commits, and a
  commit that looks droppable may be the ONLY carrier of keep-able content.
  Verify what each commit in the drop range actually touches before removing it.
- **Distributed/release branches carry NO dev scaffolding.** Agent dirs
  (`.hermes/`, `skills/`, agent-feedback docs, test markers like
  `.hook-battery-test`) belong to the WORK branch only; the distributed
  branch must be scrubbed before the first release and never re-imported by
  a transplant. Scrub = `git rm -r .hermes skills <agent-docs>`, strip
  `.gitignore` negations that re-include dev dirs (`!.hermes/`), and commit
  the scrub ON the distributed branch. Also pin LF for hook files in
  `.gitattributes` (`.githooks/* text eol=lf`) while there. Verify with
  `git ls-files | grep -E '^(\\.hermes/|skills/)'` empty on the release branch.
- **Back up before you scrub, and PRESERVE what `git rm` cannot touch.**
  The user wants removed dev material saved to scratch
  (`~/.hermes/tmp/`), never deleted outright. Tracked
  files: `git archive HEAD <paths...> | tar -x -C <scratch-dir>` preserves
  the exact committed content pre-delete (tar's `-x` extraction, not the
  archive itself). Untracked dev dirs (`.plans/`, `.bench/`, build/results
  leftovers inside a removed tree) SURVIVE `git rm` and stay in the repo -
  `git rm` only removes tracked files. Physically `mv` them out:
  `mv .plans .bench bench <scratch>/moved-from-current/`. Then re-verify
  with `git status --porcelain` that nothing untracked remains and the
  tree is clean (or holds only the intended scrub commit).

Worked recipes - heartbeat/auto-commit retargeting, dead-reference removal,
experimental-crate policy, object-store recovery: `references/cleanup-discipline.md`.

## Enumerating the fleet correctly

**`git -C <dir> rev-parse --git-dir` is NOT a repo test.** It succeeds for a plain
subdirectory because git walks _up_ to the enclosing repo, so ordinary content directories
get reported as repos showing the _parent's_ branch and remote. Use
`test -e "$dir/.git"` (a file for submodules, a directory for clones). Audit the roster in
**both** directions with `-maxdepth 3` so nested submodules one level deeper are not
missed; both lists empty is the only acceptable result for a "covers everything" claim.
Recipe: reset-and-branch-sync.md §Pitfalls.

## Driver script shape

- `set -uo pipefail`, **never `set -e`** - one repo failing must not abort the other 32.
  Explicit `|| return 1` per step.
- Parallel jobs write to `$LOGDIR/<slug>.log` + `<slug>.status`; replay logs in roster order
  afterward and derive pass/fail from status files. Slugify nested paths (`a/b` → `a_b`) or
  the log path breaks.
- Exit non-zero with the failed list so callers branch on the **exit code**, not on scraped
  text. The failing step's stderr usually went to a per-module log.
- Flags worth having: `--list`, `--dry-run`, `--only a,b`, `--jobs N`,
  `--keep-local-branches`. `--dry-run` is the cheap way to surface branch-name drift before
  anything destructive runs.
- Clean with `clean -dffx` (double `-f`) - a single `-f` refuses to remove nested
  directories that are themselves git repos.

## Stale `index.lock` self-healing

An interrupted git run leaves `index.lock` and blocks every later checkout/reset with
_"Another git process seems to be running"_. In a submodule the lock lives under the
superproject, at `<super>/.git/modules/<path>/index.lock`. Find it via
`git rev-parse --absolute-git-dir`, and clear it only when no live git process holds it
(`pgrep -qf "[g]it .*$dir"` - the bracket stops pgrep self-matching). Build this into the
driver so the fleet self-heals instead of failing one repo. Full recipe:
reset-and-branch-sync.md §4.

## Tag-heavy repos: batch-delete with SHORT names

A shallow truncation reclaims nothing while tags pin old commits, so a fleet
driver deletes tags - but a `git tag -l | while read` loop spawns one
`git tag -d` process PER TAG. A release repo with thousands of tags (e.g. an
npm-style monorepo tagging every package version) grinds for minutes and looks
hung. Batch-delete first:

```sh
git tag -l | xargs -n 100 git tag -d
```

`git tag -d` accepts ONLY short names. It rejects the full `refs/tags/<name>`
refnames that `git for-each-ref --format='%(refname)'` emits, and a batch run
with stderr swallowed (`>/dev/null 2>&1`) then fails SILENTLY with zero tags
deleted. Short names from `git tag -l` are the only accepted form.

## Interrupted parallel runs: the missing status file names the in-flight module

Per-module `<slug>.status` files make interruption recovery exact: the module
whose status file is absent (or whose log is truncated at its header) is the
one that was mid-flight when the run died. Re-run `--only <that-module>` - the
reset is idempotent and repairs whatever partial truncation the interrupt left
(wiped `refs/remotes/*`, stale shallow boundary, kept tags). Do NOT re-run the
whole fleet or re-verify around the wreckage: an orphaned git child from the
dead run can still be mutating refs, so `pgrep` for live git processes first
and let a still-running orphan finish before judging any state you observed.

## Hand-registered submodules are uninitialized until `git submodule init`

Registering a submodule by hand is not initializing it. A manual registration
(manual clone, patch `.gitmodules`, `git add <path>`) leaves
`submodule.<name>.url` out of repo config; `git submodule status` then shows a
leading `-` (not initialized) even though the checkout is present and correct.
`git submodule init <path>` (or `git submodule add` when a normal clone suffices)
clears it. `git submodule add` accepts neither `--filter` nor `--sparse`, so
sparse/partial submodules MUST be cloned by hand (`git clone --depth N
--filter=blob:none --sparse`, then `sparse-checkout set <cone>`) and registered
manually. The driver needs a sparse table (`name|cone`) that clones sparse on
fresh clones AND re-applies the cone after every reset - cone config survives
resets, but re-asserting it keeps future re-clones sparse too. `git gc
--aggressive` and `git maintenance run` work unchanged on `blob:none` promisor
clones (the filter persists in `remote.<name>.promisor`/`partialclonefilter`).

## Converting existing plain directories into submodules (dirs → own repos → gitlinks)

When repo content that lived as plain dirs must become submodule-crates (each
dir = own repo, superproject tracks gitlinks), do it in this order and gate
every push on integrity:

1. Per crate: `git init -q -b Current`, commit the EXISTING content first (the
   gitlink replaces the tree - content must be committed before conversion),
   add the remote, push.
2. Superproject: `git rm -r --cached <path>` per crate (working trees stay),
   write `.gitmodules` with `git config -f .gitmodules
   submodule.<path>.<key> <value>` (never hand-edit the file - reason
   UNKNOWN), `git add <path>` (gitlink - use `-f` when the entry has
   `ignore = all`), then `git submodule init` + `git submodule sync`.
3. Hydrate with `git submodule update --init --recursive`. Freshly-hydrated
   submodules come out DETACHED at the recorded gitlink - `git submodule
   status` shows `(sha)` instead of `(heads/Current)`; re-attach with
   `git -C <sub> checkout <branch>`.
4. Integrity gate before any push: for EVERY sub, recorded gitlink
   (`git ls-tree HEAD <sub>` / `git ls-files -s -- <sub>`) ==
   `git -C <sub> rev-parse HEAD`; per-crate tracked-file counts match the
   pre-conversion listing; the key source file non-empty with the expected
   header (`#![allow(non_snake_case)]`); the crate manifest contains the crate
   name; root history still holds the pre-conversion files.
5. A submodule branch that advances AFTER registration (e.g. a cleanup push)
   shows `+` in `git submodule status` - that is a gitlink bump in the
   superproject (`git add -f <sub>` + commit per the repo's commit cadence),
   never a revert of the sub.
6. The wipe-and-restore path (`mv <live> <Backup>`, fresh `git clone
   --recurse-submodules`) works ONLY after the root's gitlinks AND every
   submodule branch are pushed: verify `git ls-remote <root-url>` and per-sub
   `git ls-remote <sub-url> <branch>` reach the intended heads BEFORE
   recommending the wipe, then run the fresh clone's `git submodule status`
   (zero `+`/`-`, every entry `(heads/<branch>)`) as the restore gate.

## Verify independently - never trust the script's own summary

A driver reporting "33/33 ok" is a self-report. Re-derive end state from git: per repo
assert `@{upstream}` equals `origin/<current-branch>`, dirty count 0, `remote` list exactly
`origin`, origin URL equals the roster URL, and exactly one local branch. Then assert the
negatives (no `https://github.com`, no stray `Current`/`Previous`/`Source` branch anywhere).
Run `scripts/verify-fleet-sync.sh <fleet-dir>` for this.

**Always print a count from a verification loop.** A loop whose input parse yielded zero
rows "passes" having checked nothing - a silent false green. E.g. filtering
space-separated `--list` output with `grep '|'` matches nothing and the check reports
clean. Iterate the driver's own roster for verification (`--list | awk '!/^total:/{print
$1}'`, filtering the summary trailer) - never a directory glob. A glob drags in non-module
dirs, and a gate like `[ -d "$d/.git" ]` silently skips every submodule-style dir (its
`.git` is a FILE; use `test -e`), so the loop "passes" on a fraction of the fleet
(44-module fleet -> 25 checked).

## Remote-sync verification (parent + submodules)

"Are the repos and submodules synced from remote?" is answered with behind-counts, not
prose. Per parent: `git rev-list --count HEAD..<remote>/<branch>` (behind) and the
reverse (ahead) - 0/0 means the parent needs nothing. Then each submodule:

```sh
rec=$(git ls-tree HEAD "$sm" | awk '{print $3}')           # recorded gitlink
br=$(git config -f .gitmodules --get submodule.$sm.branch) # CONFIGURED branch
#   read from .gitmodules, never from the checkout - the checkout may be detached
#   (no branch) or on a different track (plugins/aphrodite: Development in the main
#   repo, Current in the release repo)
git -C "$sm" fetch -q <remote> --prune
behind=$(git -C "$sm" rev-list --count "$rec..$(git -C "$sm" rev-parse <remote>/$br)")
#   behind=0 for every submodule => all gitlinks are current, nothing to commit
```

- **Never hardcode the submodule's remote name - read it per checkout
  (`git -C "$sm" remote -v`).** The SAME submodule repo can be wired as `Source` in
  one parent and `origin` in another checkout of the same superproject (Aphrodite vs
  Aphrodite-Release). Guessing `origin` against a `Source`-wired submodule yields a
  silent wrong-tip comparison.
- **Detached checkout with a current gitlink is a no-op for the parent.** If the
  recorded gitlink == remote tip and only the checkout is detached (empty
  `git branch --show-current`), re-attach with
  `git -C "$sm" checkout -B <branch> <remote>/<branch>` at the same commit: the
  gitlink stays identical, so NO gitlink bump is staged or committed - a bump commit
  here is an unrequested commit. Reserve the bump (the user's commit to make) for the
  case where the recorded gitlink is genuinely older than the tip.
- Verify the pass with `git branch --show-current` per submodule + `git submodule
  status` (no `+`) + parent `git status` (no `M <submodule>`).

## Repairing remotes fleet-wide (fleet remote hygiene)

The repo-fleet remote setup is scripted: machine-local fleet wrapper scripts
(original path omitted - machine-local invocation) call
`Fn/Configure/{Remote,Fetch,Branch}.sh` plus `Fn/Cache.sh`. Work in the canonical
originals (the call target); `Fn/Cache.sh` exists as a DUPLICATE in both trees -
patch BOTH copies or one keeps the bug. User remote/branch conventions:
`references/user-repo-conventions.md`.

**The empty-Parent-remote bug (always-on rule):** when parent/upstream
resolution fails, scripts must store the `null`/`null` SENTINEL - never empty
strings - and every guard must check BOTH parts non-empty AND non-null.
Mechanism: `gh repo view --json parent` emits JSON `null` for non-forks; an
empty failure branch makes `Parent="$OwnerParent/$NameParent"` = `"/"`, which
passes `[ "$Parent" != "null/null" ]`, and git records a host-only remote
(`ssh://git@github.com/.git`). Full guard set, sweep recipe, macOS bash 3.2
loop pattern, functional test: `references/remote-hygiene.md`.

## Reporting to the user

State the decisive facts: exit code, counts, and the _specific_ drift found (which repo was
on the wrong branch name, which URLs changed). Flag anything destructive you cleared (stale
locks) and what you verified was safe to discard before resetting. Do not commit moved
submodule pointers in the superproject unless asked - a fleet reset legitimately moves many
gitlinks, and that is the user's commit to make.

## Claim-to-test table

| Claim | Test |
|---|---|
| A gitlink is a pinned commit SHA, not a branch | `git ls-tree HEAD <sub>` shows mode `160000` + SHA; `git submodule status` shows `(heads/<branch>)` only after a branch checkout |
| Staging the gitlink makes the pointer stick | `git add <submodule-path>` then `git submodule update` keeps the submodule on the branch; without the stage it snaps back detached |
| `ignore = all` makes `git add <sub>` skip the gitlink | `git add <sub>` prints "Skipping submodule due to ignore=all"; `git add -f <sub>` stages it |
| A dangling auto-commit is at risk before a reset | `git -C <sub> symbolic-ref -q HEAD` fails AND `git -C <sub> branch --contains HEAD` is empty |
| The hook set keeps both repos on their configured branch | `git submodule status` shows no `+`; `git status` shows no `M <submodule>` |
| Hooks actually fire | `GIT_TRACE=1 git commit --amend --no-edit 2>&1 \| grep -iE 'hook\|post-commit\|bump'` |
| No phantom gitlink in the committed tree | `git ls-tree HEAD -- <path>` empty; `git ls-files -s \| grep 160000` empty |
| `origin` is the only remote, fleet-wide | `git -C <dir> remote` prints only `origin`; negative assert: zero `https://github.com` remotes |
| Each repo's upstream matches its current branch | `scripts/verify-fleet-sync.sh <fleet-dir>`; per repo `@{upstream}` equals `origin/<current-branch>` |
| Every gitlink is current with its remote tip | per submodule `git -C "$sm" rev-list --count "$rec..<remote>/<branch>"` = 0 |
| Conversion integrity before any push | per sub recorded gitlink (`git ls-tree HEAD <sub>`) == `git -C <sub> rev-parse HEAD` |
| Local branch names mirror upstream defaults | `git ls-remote --symref origin HEAD` default matches the local branch name per repo |