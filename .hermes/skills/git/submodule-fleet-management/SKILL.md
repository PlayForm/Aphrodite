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

**Class:** many git repositories at once (submodule/vendor fleet): pristine resets, local branch names identical to upstream defaults, remote URL normalization, roster/coverage auditing, destructive-operation discipline. Read-only mirrors of PlayForm/Aphrodite: `plugins/aphrodite` (remote `Source`), `vendor/headroom`, `vendor/rtk` - "make it exactly match upstream" (distinct from `git-operations` / `git-feat-dev-workflow`).

References (worked recipes, probes, scar catalog):

- `references/submodule-fleet-reset/reset-and-branch-sync.md` - reset recipe, default-branch resolution, enumeration, driver parallelism, verification.
- `references/submodule-fleet-reset/shallow-depth-truncation.md` - shallow-depth truncation.
- `references/gitlink-hooks.md` - hook roles, batteries, pitfalls (CRLF, `GIT_DIR`/`GIT_WORK_TREE`), phantom-gitlink catalog and repair, standalone clones, bottom-up fixes.
- `references/cleanup-discipline.md` - repair-in-place, scrub, backup recipes.
- `references/remote-hygiene.md` - empty-Parent bug, `null`/`null` sentinel, URL sweep, bash 3.2 loops.
- `references/formatter-parity.md` - byte-identical mirrors between editor and CI.
- `references/dir-to-submodule-conversion.md` - plain dirs to own repos to gitlinks walkthrough.
- `references/user-repo-conventions.md` - user remote/branch conventions (`Source`, `Parent`, `Current`).
- Verifiers: `scripts/verify-fleet-sync.sh`, `scripts/verify-fleet-depth.sh`.

## Core rules

1. **Never assume `main`:** `git ls-remote --symref origin HEAD` is authoritative; fall back to cached `refs/remotes/origin/HEAD`, then a candidate list (`main master trunk production develop`); cache with `git remote set-head origin <default>`. Worked function: reset-and-branch-sync.md §1.
2. **Sync the branch NAME, not just the commit:** `git checkout --force -B "$default" "refs/remotes/origin/$default"`, then `branch --set-upstream-to`; custom local nomenclature (a `Current` branch, etc.) is replaced by the upstream name.
3. **Local-only branches are deleted by default** (that is what keeps names mirrored); `--keep-local-branches` is the escape hatch - deletion must not be opt-in, or drift silently persists.
4. **`origin` is the only remote.** Sweep strays (`Source`, `Parent`, malformed URLs) and submodules with NO remote: a missing `origin` makes `git fetch origin` fail with "does not appear to be a git repository / access rights and the repository exists" - which does NOT mean the fork is unreachable (a nonexistent remote NAME falls back to path resolution). Probe `git -C <sub> config --get remote.origin.url` first; wire origins from `.gitmodules` URLs (`git remote add origin <url>`) then fetch.
5. **Honor the user's URL scheme uniformly:** if `ssh://git@github.com/...` is standard, convert **every** GitHub remote and assert zero `https://github.com` survivors; non-GitHub hosts (codeberg, gitlab) keep their own scheme - never rewrite a host the rule does not name.
6. **Audit before any `--hard` reset. Classify, do not count:** thousands of "modifications" may be staged _deletions of upstream files_ (a reset restores them); "unpushed commits" may be upstream authors on a stale ref. Probe `status --porcelain --untracked-files=no`, `log --branches --not --remotes --format='%an'`, `merge-base --is-ancestor`; save ref tips to `~/.hermes/tmp/` as evidence first. Only reset once nothing was authored locally. Worked probes: reset-and-branch-sync.md §Audit.

Stop if: `status --porcelain --untracked-files=no` shows tracked modifications not classified as staged deletions of upstream files. Recovery: classify with `log --branches --not --remotes --format='%an'` and `merge-base --is-ancestor`; only reset when nothing was authored locally.

## Detached HEAD from the superproject gitlink

A **gitlink** is a pinned commit SHA in the superproject's index - NOT a branch. `git submodule update` checks the submodule out **detached** at that SHA even when its branch moved ahead; a commit made detached dangles off no branch. Stay on-branch, durably:

```sh
git -C <submodule-path> checkout <branch>   # e.g. Current
git add <submodule-path>                     # stage the gitlink bump, no commit
```

Staging makes it stick (index record, not a commit); without it the next `git submodule update` snaps back to the stale SHA. The pointer commit stays the user's (see Reporting). `git submodule status`: leading `+` = ahead of the recorded gitlink; no prefix + `(heads/<branch>)` = pinned on the branch tip.

**`ignore = all` in `.gitmodules` (and a gitignored path like `.config/`) makes `git add <sub>` silently SKIP the gitlink update** ("Skipping submodule due to ignore=all") - stage with `git add -f <sub>` instead. Compare the RECORDED gitlink (`git ls-tree HEAD <sub>`) to the checkout: equal = only the checkout drifted (a reset fixes it); recorded older than fork tip = real pointer advance - confirm `git -C <sub> merge-base --is-ancestor <old> <new>` before recording it.

### Rescuing a dangling auto-commit before any reset

Auto-committers commit on the detached HEAD - a commit referenced by NO branch, dangling once the next `submodule update` moves HEAD back to the gitlink SHA. Before ANY reset:

```sh
git -C <sub> symbolic-ref -q HEAD       # fails => detached
git -C <sub> branch --contains HEAD     # empty => the commit is unreferenced
```

Both true = a swept commit is at risk. Rescue by fast-forwarding the branch to it (check `git merge-base --is-ancestor <old> <new>` first), then `git branch --contains <sha>` must list the branch:

```sh
git -C <sub> branch -f Current <sha> && git -C <sub> checkout Current
```

Reflog (~90 days) + a superproject gitlink at the new SHA are secondary safety nets, not guarantees. Worked detail + submodule-side pre-commit guard: `references/gitlink-hooks.md`.

## Always-on-branch + auto-synced gitlink hooks

Durable fix: ONE shared `.githooks/` dir that BOTH repos point at (hooksPath is per-repo - the submodule needs `git config core.hooksPath ../../.githooks` relative to ITS top-level; `git rev-parse --git-path hooks` should resolve it). Gate every hook on `git rev-parse --show-superproject-working-tree` non-empty so it no-ops in the plain repo.

Invariant: BOTH repos stay on their CONFIGURED tracking branch (`submodule.<name>.branch` in `.gitmodules` - historically `Current`, but the parent may point a submodule at a different branch e.g. `Development`; hooks must read it, never hardcode `Current`), and `git submodule status` / `git status` never show `+` / `M <submodule>`.

Roles: `pre-commit` refuses detached-HEAD commits (`git symbolic-ref -q HEAD || exit 1`); `post-commit` bumps the parent gitlink; `post-checkout` re-attaches a detached checkout to the CONFIGURED branch then bumps; shared `lib/bump-submodule-gitlink.sh` computes the relative path with python3 `os.path.relpath` (GNU `realpath --relative-to` does not exist on macOS), skips while the parent holds `MERGE_HEAD`/`CHERRY_PICK_HEAD`, commits `chore: bump <sub> to <short>` ONLY when `git -C <super> ls-files -s -- <sub>` differs from the submodule HEAD. Full batteries + pitfalls (executable bit, CRLF shebang, `GIT_DIR`/`GIT_WORK_TREE`, phantom gitlinks, cherry-pick import, standalone clones, siblings, bottom-up vendor fixes, committed phantoms breaking clones): `references/gitlink-hooks.md`.

Why it holds: `git submodule update` checks out the gitlink DETACHED and fires the submodule's post-checkout hook - it re-attaches and re-records the pointer in one operation, so the detached state never survives a single command.

Stop if: any battery step fails. Recovery: fix the failing hook (executable bit, CRLF shebang, `GIT_DIR`/`GIT_WORK_TREE` override - see `references/gitlink-hooks.md`), then re-run the battery from step 1.

## Parent-side-only variant (current design in Aphrodite / Aphrodite-Release) [CLAIM]

Submodule-side hooks fail when the submodule's own hooks are unreliable (auto-committers using plumbing/`--no-verify` skip them; phantom self-referential gitlinks leak into submodule indexes). Now the hooks run in the PARENT only: `core.hooksPath = .githooks` in the SUPERPROJECTS; shared `lib/bump-submodule-gitlinks.sh` + hooks (`post-commit`, `post-merge`, `post-checkout` branch-only via `$3 == 1`, `pre-push`) commit the parent's OWN gitlinks on parent-side actions. [CLAIM: "now run" is live state; probe `git -C <super> config --get core.hooksPath` and `git rev-parse --git-path hooks` in the parent.]

Submodule repos must NOT carry their own `.githooks/` (standing rule: "only updates to the gitlink are necessary from the parent repository") - a child's `.githooks` misleads standalone clones (hooksPath is local config, dir untracked in the child). Removing one: `git rm -r .githooks` + `git config --unset core.hooksPath` (a dangling hooksPath silently disables ALL hooks), commit + push the child, then bump the parent gitlink.

Guards in the shared lib (ALL must pass before any commit):

1. Superproject guard: only run when the current repo IS the parent (own `.gitmodules` declares submodules) - no-ops in vendor/submodule checkouts.
2. Real-gitlink-only: bump candidates are only mode `160000` paths in the parent's index (`git ls-files -s -- <path>`) - prevents phantom gitlinks by construction.
3. Recursion guard: `GITLINK_BUMP_ACTIVE` env var - the hook's own `git commit` re-fires `post-commit`; without it the loop never ends.
4. Mid-operation skip: no bump while the parent holds `MERGE_HEAD` / `CHERRY_PICK_HEAD` / is mid-rebase.
5. No-op when unchanged: only commit when `git ls-files -s -- <sub>` differs from the submodule's current HEAD.

Commit shape: surgical `git commit -o -- "${DIRTY[@]}" --no-verify -s` (only the dirty gitlink pathspecs; `-o` keeps everything else out; `--no-verify` prevents hook re-entry; `-s` signs like the auto-committer), wrapped in `|| true` so a transient index lock converges on the next sweep instead of failing the user's real commit. Battery A-E + `IFS=`/`GIT_TRACE=1` pitfalls: `references/gitlink-hooks.md`.

## Resolving a submodule merge conflict (parallel implementations)

Auto-committers on both sides often implement the SAME feature independently - the merge shows "both modified" on the same files:

- Scope both sides first - one is usually a SUPERSET (extra commands/tests); take it, keep the other side's text only where it documents something the superset misses.
- Dedupe what both sides added twice: guards, tests, env-var entries, identical gitlink bumps; check `plugin.yaml`/manifest-style files too.
- Scripted marker-block replacement (`<<<<<<<` ... `>>>>>>>`) must end with the FILE's line terminator: a missing trailing CRLF glues the next code line onto the resolution, often INTO a comment (py_compile passes; the NameError surfaces only at import/test time). Assert exactly one occurrence per block; re-read file bytes (mixed CRLF/LF is normal after a merge).
- The fuzzy patch tool re-indents multi-line blocks on CRLF/tab-indented files; after two mangled attempts on the same region, switch to an exact-byte replacement script.

## Cleanup discipline (standing user rules)

- **NEVER run `git rebase` - any form, any repo, ever.** Cleanup is `git reset`/merge/commit-forward only; if removing a run of commits requires rebase, accept the stale commits or reset to a kept ancestor and re-commit the pointer forward.
- **`git push --force` is authorized WHEN NEEDED**, but verify first: `git fetch Source` then `git log Source/Current..Current` must show the remote-only commits are pure descendants of local (junk sweeps, test artifacts). Never force over unverified remote work.
- **Rollback verification:** after `git reset --hard` cleanup, re-check AFTER the auto-committer has swept - it can re-commit an identical state (same tree+message = same hash) and resurrect the exact mess; a later `git submodule update` can re-detach at the stale pointer. One clean `git status` right after the reset proves nothing.
- **Never `git reset`/`git checkout` a file to "restore" it - repair in place.** `git checkout -- <file>` restores from the INDEX, not HEAD - with an auto-committer that stages edits it is a NO-OP, and git-restoring throws corrupted content away (escaped newlines are recoverable data). Repair with `read_file` + `patch`/`write_file`; already reset? `git fsck --lost-found`, match by size (`git cat-file -s <hash>`), extract (`git cat-file blob <hash> > file`). On `MM`, align the index with `git add` - never reset/checkout. Governs files carrying intended content; mirror resets are a different operation.
- **Before dropping a commit as "test junk", locate the keep-able content** with `git log --oneline -- <path>` - the auto-committer bundles hook refactors/real fixes into unrelated-looking "bump"/"chore" commits.
- **Distributed/release branches carry NO dev scaffolding.** Agent dirs (`.hermes/`, `skills/`, agent-feedback docs, `.hook-battery-test`) belong to the WORK branch only. Scrub = `git rm -r .hermes skills <agent-docs>`, strip `.gitignore` negations (`!.hermes/`), pin LF (`.githooks/* text eol=lf`), commit the scrub ON the distributed branch; verify `git ls-files | grep -E '^(\.hermes/|skills/)'` empty on the release branch.
- **Back up before you scrub; preserve what `git rm` cannot touch.** Tracked: `git archive HEAD <paths...> | tar -x -C <scratch-dir>`. Untracked dev dirs (`.plans/`, `.bench/`, build leftovers) SURVIVE `git rm` - `mv` them out (`mv .plans .bench bench <scratch>/moved-from-current/`). Re-verify `git status --porcelain`.

Worked recipes - heartbeat/auto-commit retargeting, dead-reference removal, experimental-crate policy, object-store recovery: `references/cleanup-discipline.md`.

## Enumerating the fleet correctly

**`git -C <dir> rev-parse --git-dir` is NOT a repo test** - it walks UP to the enclosing repo. Use `test -e "$dir/.git"` (a file for submodules, a directory for clones). Audit the roster in **both** directions with `-maxdepth 3`; both lists empty is the only acceptable "covers everything". Recipe: reset-and-branch-sync.md §Pitfalls.

## Driver script shape

- `set -uo pipefail`, **never `set -e`** - one repo failing must not abort the other 32; explicit `|| return 1` per step.
- Parallel jobs write to `$LOGDIR/<slug>.log` + `<slug>.status`; replay logs in roster order, derive pass/fail from status files. Slugify nested paths (`a/b` → `a_b`).
- Exit non-zero with the failed list - callers branch on the **exit code**, not scraped text.
- Flags: `--list`, `--dry-run`, `--only a,b`, `--jobs N`, `--keep-local-branches`; `--dry-run` surfaces branch-name drift before anything destructive runs.
- Clean with `clean -dffx` (double `-f`) - a single `-f` refuses to remove nested directories that are themselves git repos.

## Stale `index.lock` self-healing

An interrupted run leaves `index.lock`, blocking checkouts/resets with "Another git process seems to be running". In a submodule it lives at `<super>/.git/modules/<path>/index.lock` - find via `git rev-parse --absolute-git-dir`; clear only when no live git process holds it (`pgrep -qf "[g]it .*$dir"` - the bracket stops pgrep self-matching). Build into the driver so the fleet self-heals. Full recipe: reset-and-branch-sync.md §4.

## Tag-heavy repos: batch-delete with SHORT names

A `git tag -l | while read` loop spawns one `git tag -d` process PER TAG (thousands of tags = minutes, looks hung). Batch-delete first:

```sh
git tag -l | xargs -n 100 git tag -d
```

`git tag -d` accepts ONLY short names - it rejects `refs/tags/<name>` refnames; a batch run with stderr swallowed (`>/dev/null 2>&1`) fails SILENTLY with zero tags deleted.

## Interrupted parallel runs: the missing status file names the in-flight module

The module whose `<slug>.status` file is absent (or log truncated at its header) was mid-flight when the run died. Re-run `--only <that-module>` - the reset is idempotent and repairs the partial truncation (wiped `refs/remotes/*`, stale shallow boundary, kept tags). Do NOT re-run the whole fleet: `pgrep` for live git processes first, let a still-running orphan finish before judging any state.

## Hand-registered submodules are uninitialized until `git submodule init`

Manual registration (manual clone, patch `.gitmodules`, `git add <path>`) leaves `submodule.<name>.url` out of repo config; `git submodule status` shows a leading `-` (not initialized) even though the checkout is correct. `git submodule init <path>` (or `git submodule add`) clears it. `git submodule add` accepts neither `--filter` nor `--sparse` - sparse/partial submodules MUST be cloned by hand (`git clone --depth N --filter=blob:none --sparse`, then `sparse-checkout set <cone>`) and registered manually. The driver needs a sparse table (`name|cone`) applied on fresh clones AND after every reset. `git gc --aggressive` / `git maintenance run` are safe on `blob:none` promisor clones (the filter persists in `remote.<name>.promisor`/`partialclonefilter`).

## Converting existing plain directories into submodules (dirs → own repos → gitlinks)

Per crate: `git init -q -b Current`, commit the EXISTING content first, add the remote, push. Superproject: `git rm -r --cached <path>` per crate (working trees stay), write `.gitmodules` via `git config -f .gitmodules submodule.<path>.<key> <value>` (never hand-edit the file), `git add <path>` (gitlink - `-f` under `ignore = all`), then `git submodule init` + `sync` + `update --init --recursive`; re-attach freshly-hydrated detached checkouts (`git -C <sub> checkout <branch>`). Gate every push on integrity: recorded gitlink (`git ls-tree HEAD <sub>`) == `git -C <sub> rev-parse HEAD`, tracked-file counts match, key source file non-empty with expected header (`#![allow(non_snake_case)]`), manifest contains the crate name, root history still holds pre-conversion files. A branch advancing AFTER registration shows `+` - that is a gitlink bump (`git add -f <sub>` + commit), never a revert. Wipe-and-restore (`mv <live> <Backup>`, fresh `git clone --recurse-submodules`) only after gitlinks AND submodule branches are pushed (`git ls-remote` reaches the intended heads); the fresh clone's `git submodule status` (zero `+`/`-`, `(heads/<branch>)`) is the restore gate. Full walkthrough: `references/dir-to-submodule-conversion.md`.

## Verify independently - never trust the script's own summary

A driver reporting "33/33 ok" is a self-report. Per repo assert `@{upstream}` == `origin/<current-branch>`, dirty count 0, `remote` list exactly `origin`, origin URL == roster URL, exactly one local branch; then assert the negatives (no `https://github.com`, no stray `Current`/`Previous`/`Source` branch). Run `scripts/verify-fleet-sync.sh <fleet-dir>`.

**Always print a count from a verification loop** - a loop whose input parse yielded zero rows "passes" having checked nothing. Iterate the driver's own roster (`--list | awk '!/^total:/{print $1}'`) - never a directory glob (it drags in non-module dirs, and `[ -d "$d/.git" ]` silently skips submodule-style dirs whose `.git` is a FILE - use `test -e`).

## Remote-sync verification (parent + submodules)

Answered with behind-counts, not prose. Per parent: `git rev-list --count HEAD..<remote>/<branch>` (behind) + the reverse (ahead) - 0/0 = parent needs nothing. Then each submodule:

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

- **Never hardcode the submodule's remote name - read it per checkout (`git -C "$sm" remote -v`)** - the SAME repo can be `Source` in one parent and `origin` in another; guessing `origin` against a `Source`-wired submodule yields a silent wrong-tip comparison.
- **Detached checkout with a current gitlink is a no-op for the parent:** recorded gitlink == remote tip → re-attach with `git -C "$sm" checkout -B <branch> <remote>/<branch>` at the same commit; NO gitlink bump is staged or committed (a bump here is an unrequested commit). Reserve the bump for a genuinely older recorded gitlink.
- Verify the pass: `git branch --show-current` per submodule + `git submodule status` (no `+`) + parent `git status` (no `M <submodule>`).

## Repairing remotes fleet-wide (fleet remote hygiene)

Scripted via machine-local fleet wrapper scripts calling `Fn/Configure/{Remote,Fetch,Branch}.sh` plus `Fn/Cache.sh` (a DUPLICATE in both trees - patch BOTH copies or one keeps the bug). User remote/branch conventions: `references/user-repo-conventions.md`.

**The empty-Parent-remote bug (always-on rule):** when parent/upstream resolution fails, scripts must store the `null`/`null` SENTINEL - never empty strings - and every guard must check BOTH parts non-empty AND non-null. Mechanism: `gh repo view --json parent` emits JSON `null` for non-forks; an empty failure branch makes `Parent="$OwnerParent/$NameParent"` = `"/"`, which passes `[ "$Parent" != "null/null" ]`, and git records a host-only remote (`ssh://git@github.com/.git`). Full guard set, sweep recipe, macOS bash 3.2 loop pattern, functional test: `references/remote-hygiene.md`.

## Formatter parity between editor and CI (byte-identical mirrors)

Nightly-only rustfmt options (`space_after_colon = false`, `imports_granularity`) format DIFFERENTLY under VSCode's default formatter - the CI fmt gate fails on every push while the code looks fine locally. Point the editor at the CI toolchain via `.vscode/settings.json` `rust-analyzer.rustfmt.overrideCommand` (`rustup run nightly-<CI-pin> rustfmt --edition <Y>`); bind Python to the repo's ruff (`ruff.toml`). A byte-equal-to-source file must be excluded from EVERY formatter (`rustfmt.toml` `ignore` + `ruff.toml` `extend-exclude`). To reformat the pair: format the SOURCE first, then copy it over the mirror - never format the mirror alone, never `cargo fmt --all` blindly. Full recipe: `references/formatter-parity.md`.

## Reporting to the user

State the decisive facts: exit code, counts, the _specific_ drift found (which repo was on the wrong branch name, which URLs changed). Flag anything destructive you cleared (stale locks) and what you verified was safe to discard before resetting. Do not commit moved submodule pointers in the superproject unless asked - a fleet reset legitimately moves many gitlinks, and that is the user's commit to make.

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