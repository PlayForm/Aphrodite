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

**Class:** many git repos at once (submodule/vendor fleet): pristine resets, branch-name/remote normalization, roster/coverage auditing, destructive-operation discipline.

References:

- `references/submodule-fleet-reset/reset-and-branch-sync.md` - reset recipe, default-branch resolution, enumeration, verification.
- `references/submodule-fleet-reset/shallow-depth-truncation.md` - shallow-depth truncation.
- `references/gitlink-hooks.md` - hook roles, batteries, pitfalls (CRLF, `GIT_DIR`/`GIT_WORK_TREE`), phantom gitlinks, standalone clones.
- `references/cleanup-discipline.md` - repair-in-place, scrub, backup recipes.
- `references/remote-hygiene.md` - empty-Parent bug, `null`/`null` sentinel, URL sweep, bash 3.2 loops.
- `references/formatter-parity.md` - byte-identical mirrors between editor and CI.
- `references/dir-to-submodule-conversion.md` - plain dirs to own repos to gitlinks walkthrough.
- `references/fleet-driver-patterns.md` - driver shape, `index.lock`, tags, sparse, verification.
- `references/user-repo-conventions.md` - user remote/branch conventions (`Source`, `Parent`, `Current`).

## Core rules

1. **Never assume `main`:** `git ls-remote --symref origin HEAD`; fall back to `refs/remotes/origin/HEAD`, then `main master trunk production develop`.
2. **Sync the branch NAME, not just the commit:** `git checkout --force -B "$default" "refs/remotes/origin/$default"` then `branch --set-upstream-to`; custom nomenclature is replaced by the upstream name.
3. **Local-only branches are deleted by default**; `--keep-local-branches` is the escape hatch.
4. **`origin` is the only remote.** Sweep strays (`Source`, `Parent`, malformed) and remote-less submodules: a missing `origin` makes `git fetch origin` fail (nonexistent remote NAME falls back to path resolution). Probe `git -C <sub> config --get remote.origin.url`; wire origins from `.gitmodules` URLs (`git remote add origin <url>`) then fetch.
5. **Honor the user's URL scheme uniformly:** convert **every** GitHub remote to `ssh://git@github.com/...` and assert zero `https://github.com` survivors; non-GitHub hosts (codeberg, gitlab) keep their scheme.
6. **Audit before any `--hard` reset. Classify, do not count:** "modifications" may be staged deletions of upstream files; "unpushed commits" may be upstream authors on a stale ref. Probe `status --porcelain --untracked-files=no`, `log --branches --not --remotes --format='%an'`, `merge-base --is-ancestor`.

Stop if: `status --porcelain --untracked-files=no` shows tracked modifications not classified as staged deletions of upstream files. Recovery: `log --branches --not --remotes --format='%an'` + `merge-base --is-ancestor`.

## Detached HEAD from the superproject gitlink

A **gitlink** is a pinned commit SHA in the superproject's index - NOT a branch. `git submodule update` checks out **detached** at that SHA. Stay on-branch:

```sh
git -C <submodule-path> checkout <branch>   # e.g. Current
git add <submodule-path>                     # stage the gitlink bump, no commit
```

Staging makes it stick; without it, `git submodule update` snaps back. `git submodule status`: `+` = ahead; `(heads/<branch>)` = pinned. **`ignore = all` makes `git add <sub>` SKIP the gitlink** - use `git add -f <sub>`. Compare the RECORDED gitlink (`git ls-tree HEAD <sub>`): equal = checkout-only drift; older = real advance (confirm `git -C <sub> merge-base --is-ancestor <old> <new>` first).

Before ANY reset (dangling auto-commit rescue): `git -C <sub> symbolic-ref -q HEAD` (fails = detached) + `git -C <sub> branch --contains HEAD` (empty = unreferenced) = at risk; rescue `git -C <sub> branch -f Current <sha> && git -C <sub> checkout Current`. Detail: `references/gitlink-hooks.md`.

## Always-on-branch + auto-synced gitlink hooks

Durable fix: ONE shared `.githooks/` dir both repos point at (submodule: `git config core.hooksPath ../../.githooks`). Gate every hook on `git rev-parse --show-superproject-working-tree` non-empty.

Invariant: BOTH repos stay on their CONFIGURED branch (`submodule.<name>.branch` in `.gitmodules`; never hardcode `Current`); `git submodule status`/`git status` show no `+`/`M`.

Roles: `pre-commit` refuses detached-HEAD commits (`git symbolic-ref -q HEAD || exit 1`); `post-commit` bumps the parent gitlink; `post-checkout` re-attaches to the CONFIGURED branch then bumps; shared `lib/bump-submodule-gitlink.sh` (python3 `os.path.relpath`; skips on `MERGE_HEAD`/`CHERRY_PICK_HEAD`; bumps only when `git -C <super> ls-files -s -- <sub>` differs). Detail: `references/gitlink-hooks.md`.

Stop if: any battery step fails. Recovery: fix the failing hook (`references/gitlink-hooks.md`), re-run the battery.

## Parent-side-only variant (current design in Aphrodite / Aphrodite-Release) [CLAIM]

Submodule-side hooks fail when the submodule's own hooks are unreliable. Now the hooks run in the PARENT only: `core.hooksPath = .githooks` in the SUPERPROJECTS; shared `lib/bump-submodule-gitlinks.sh` + hooks (`post-commit`, `post-merge`, `post-checkout` branch-only via `$3 == 1`, `pre-push`) commit the parent's OWN gitlinks. [CLAIM: "now run" is live state - probe `git -C <super> config --get core.hooksPath`.]

Submodule repos must NOT carry their own `.githooks/` (standing rule: "only updates to the gitlink are necessary from the parent repository"); removing one: `git rm -r .githooks` + `git config --unset core.hooksPath`.

Guards (ALL must pass):

1. Superproject guard: only run when the current repo IS the parent (own `.gitmodules` declares submodules).
2. Real-gitlink-only: bump only mode `160000` paths (`git ls-files -s -- <path>`).
3. Recursion guard: `GITLINK_BUMP_ACTIVE` env var - the hook's own commit re-fires `post-commit`.
4. Mid-operation skip: no bump while the parent holds `MERGE_HEAD` / `CHERRY_PICK_HEAD` / is mid-rebase.
5. No-op when unchanged: only commit when `git ls-files -s -- <sub>` differs from the submodule HEAD.

Commit shape: `git commit -o -- "${DIRTY[@]}" --no-verify -s`, wrapped in `|| true`. Battery A-E: `references/gitlink-hooks.md`.

## Resolving a submodule merge conflict (parallel implementations)

Take the SUPERSET; dedupe (guards, tests, env-var entries, gitlink bumps; check `plugin.yaml` too); end marker-block replacements with the FILE's line terminator (missing CRLF glues the next line INTO a comment); after two mangled fuzzy-patch attempts, use an exact-byte replacement script. Detail: `references/merge-conflict-resolution.md`.

## Cleanup discipline (standing user rules)

- **NEVER `git rebase` - any form, any repo, ever.** Cleanup: `git reset`/merge/commit-forward only.
- **`git push --force` is authorized WHEN NEEDED**, after `git fetch Source` + `git log Source/Current..Current` prove remote-only commits are pure descendants.
- **Rollback:** re-verify AFTER the auto-committer sweeps (it can re-commit an identical state = same hash).
- **Never `git reset`/`git checkout` a file to "restore" it - repair in place** (`git checkout -- <file>` restores from the INDEX = NO-OP under an auto-committer; recovery: `git fsck --lost-found` + `git cat-file blob <hash> > file`; `MM` → `git add`).

- **Distributed/release branches carry NO dev scaffolding:** `git rm -r .hermes skills <agent-docs>`, strip `.gitignore` negations (`!.hermes/`), pin `.githooks/* text eol=lf`; verify `git ls-files | grep -E '^(\.hermes/|skills/)'` empty.
- **Back up before scrub:** `git archive HEAD <paths...> | tar -x -C <scratch-dir>`; untracked dev dirs SURVIVE `git rm` - `mv` them out.

## Enumerating the fleet correctly

**`git -C <dir> rev-parse --git-dir` is NOT a repo test** (walks UP to the enclosing repo); use `test -e "$dir/.git"`. Audit the roster in **both** directions with `-maxdepth 3`. Detail: `references/fleet-driver-patterns.md` §Enum.

## Driver script shape

`set -uo pipefail`, **never `set -e`** (`|| return 1` per step); `$LOGDIR/<slug>.log` + `<slug>.status` per module (slugify `a/b` → `a_b`); exit non-zero with the failed list; flags `--list --dry-run --only a,b --jobs N --keep-local-branches`; `clean -dffx` (double `-f`). Detail: `references/fleet-driver-patterns.md` §Driver.

## Stale `index.lock` self-healing

`index.lock` at `<super>/.git/modules/<path>/index.lock` (`git rev-parse --absolute-git-dir`); clear only when `pgrep -qf "[g]it .*$dir"` finds no live git process. Detail: `references/fleet-driver-patterns.md` §Lock.

## Tag-heavy repos: batch-delete with SHORT names

`git tag -d` takes ONLY short names (rejects `refs/tags/<name>`); stderr-swallowed batches fail SILENTLY:

```sh
git tag -l | xargs -n 100 git tag -d
```

## Interrupted parallel runs: the missing status file names the in-flight module

Missing `<slug>.status` = the module mid-flight when the run died; re-run `--only <that-module>` (idempotent). Do NOT re-run the whole fleet: `pgrep` live git first. Detail: `references/fleet-driver-patterns.md` §Interrupted.

## Hand-registered submodules are uninitialized until `git submodule init`

Manual registration leaves `submodule.<name>.url` out of repo config (`git submodule status` shows `-`); `git submodule init <path>` clears it. Sparse/partial submodules MUST be hand-cloned (`git submodule add` takes neither `--filter` nor `--sparse`): `git clone --depth N --filter=blob:none --sparse` + `sparse-checkout set <cone>`. Detail: `references/fleet-driver-patterns.md` §Sparse.

## Converting existing plain directories into submodules (dirs → own repos → gitlinks)

**Order:** per crate `git init -q -b Current` + commit existing content + push; superproject `git rm -r --cached <path>`, `.gitmodules` via `git config -f .gitmodules submodule.<path>.<key> <value>`, `git add <path>`, `git submodule init` + `sync` + `update --init --recursive`, re-attach detached checkouts. Gate every push on integrity: recorded gitlink == `git -C <sub> rev-parse HEAD`. Full walkthrough: `references/dir-to-submodule-conversion.md`.

## Verify independently - never trust the script's own summary

Never trust the script's own summary. Re-derive: `@{upstream}` == `origin/<current-branch>`, dirty 0, `remote` exactly `origin`, origin URL == roster URL, one local branch + negatives (no `https://github.com`, no stray `Current`/`Previous`/`Source`). `scripts/verify-fleet-sync.sh <fleet-dir>`; always print a count; iterate the driver's own roster (`--list | awk '!/^total:/{print $1}'`), never a glob. Detail: `references/fleet-driver-patterns.md` §Verify.

## Remote-sync verification (parent + submodules)

Behind-counts, not prose: per parent `git rev-list --count HEAD..<remote>/<branch>` + reverse = 0/0; per submodule `behind=$(git -C "$sm" rev-list --count "$rec..<remote>/$br")` = 0 after `git -C "$sm" fetch -q <remote> --prune` (recorded gitlink vs CONFIGURED branch: `git config -f .gitmodules --get submodule.$sm.branch`). Never hardcode the sub's remote name (per checkout: `git -C "$sm" remote -v`); detached + current gitlink = no-op (re-attach `git -C "$sm" checkout -B <branch> <remote>/<branch>`; NO bump). Script: `references/fleet-driver-patterns.md` §Sync.

## Repairing remotes fleet-wide (fleet remote hygiene)

Machine-local wrapper scripts call `Fn/Configure/{Remote,Fetch,Branch}.sh` + `Fn/Cache.sh` (DUPLICATE in both trees - patch BOTH). Conventions: `references/user-repo-conventions.md`.

**The empty-Parent-remote bug (always-on rule):** when parent/upstream resolution fails, store the `null`/`null` SENTINEL - never empty strings - and every guard must check BOTH parts non-empty AND non-null. Detail: `references/remote-hygiene.md`.

## Formatter parity between editor and CI (byte-identical mirrors)

Nightly-only rustfmt options (`space_after_colon = false`, `imports_granularity`) drift the CI gate. Fix: `.vscode/settings.json` `rust-analyzer.rustfmt.overrideCommand` = `rustup run nightly-<CI-pin> rustfmt --edition <Y>`; bind Python to the repo's ruff (`ruff.toml`). Byte-equal-to-source files: exclude from EVERY formatter (`rustfmt.toml` `ignore` + `ruff.toml` `extend-exclude`); reformat SOURCE-first. Full recipe: `references/formatter-parity.md`.

## Reporting to the user

Exit code, counts, the _specific_ drift found. Do not commit moved submodule pointers unless asked.

## Claim-to-test table

| Claim                                                    | Test                                                                                                                               |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| A gitlink is a pinned commit SHA, not a branch           | `git ls-tree HEAD <sub>` shows mode `160000` + SHA; `git submodule status` shows `(heads/<branch>)` only after a branch checkout   |
| Staging the gitlink makes the pointer stick              | `git add <submodule-path>` then `git submodule update` keeps the submodule on the branch; without the stage it snaps back detached |
| `ignore = all` makes `git add <sub>` skip the gitlink    | `git add <sub>` prints "Skipping submodule due to ignore=all"; `git add -f <sub>` stages it                                        |
| A dangling auto-commit is at risk before a reset         | `git -C <sub> symbolic-ref -q HEAD` fails AND `git -C <sub> branch --contains HEAD` is empty                                       |
| The hook set keeps both repos on their configured branch | `git submodule status` shows no `+`; `git status` shows no `M <submodule>`                                                         |
| Hooks actually fire                                      | `GIT_TRACE=1 git commit --amend --no-edit 2>&1 \| grep -iE 'hook\|post-commit\|bump'`                                              |
| No phantom gitlink in the committed tree                 | `git ls-tree HEAD -- <path>` empty; `git ls-files -s \| grep 160000` empty                                                         |
| `origin` is the only remote, fleet-wide                  | `git -C <dir> remote` prints only `origin`; negative assert: zero `https://github.com` remotes                                     |
| Each repo's upstream matches its current branch          | `scripts/verify-fleet-sync.sh <fleet-dir>`; per repo `@{upstream}` equals `origin/<current-branch>`                                |
| Every gitlink is current with its remote tip             | per submodule `git -C "$sm" rev-list --count "$rec..<remote>/<branch>"` = 0                                                        |
| Conversion integrity before any push                     | per sub recorded gitlink (`git ls-tree HEAD <sub>`) == `git -C <sub> rev-parse HEAD`                                               |
| Local branch names mirror upstream defaults              | `git ls-remote --symref origin HEAD` default matches the local branch name per repo                                                |
