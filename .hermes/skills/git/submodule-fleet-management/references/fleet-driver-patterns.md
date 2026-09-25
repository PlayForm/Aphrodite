# Fleet driver patterns and verification (worked recipes)

Reference for SKILL.md sections: "Enumerating the fleet correctly", "Driver script
shape", "Stale `index.lock` self-healing", "Tag-heavy repos", "Interrupted parallel
runs", "Hand-registered submodules", "Verify independently", "Remote-sync
verification". The rules live in SKILL.md; full worked detail here. All commands are
byte-stable.

## Enumerating the fleet correctly

**`git -C <dir> rev-parse --git-dir` is NOT a repo test** - it walks UP to the enclosing repo. Use `test -e "$dir/.git"` (file for submodules, dir for clones). Audit the roster in **both** directions with `-maxdepth 3`; both lists empty is the only acceptable "covers everything". Recipe: reset-and-branch-sync.md §Pitfalls.

## Driver script shape

- `set -uo pipefail`, **never `set -e`** - one repo failing must not abort the rest; explicit `|| return 1` per step.
- Parallel jobs write to `$LOGDIR/<slug>.log` + `<slug>.status`; derive pass/fail from status files. Slugify nested paths (`a/b` → `a_b`).
- Exit non-zero with the failed list - callers branch on the **exit code**, not scraped text.
- Flags: `--list`, `--dry-run`, `--only a,b`, `--jobs N`, `--keep-local-branches`; `--dry-run` surfaces branch-name drift first.
- Clean with `clean -dffx` (double `-f`) - a single `-f` refuses nested git repos.

## Stale `index.lock` self-healing

An interrupted run leaves `index.lock`, blocking checkouts/resets. In a submodule it lives at `<super>/.git/modules/<path>/index.lock` - find via `git rev-parse --absolute-git-dir`; clear only when no live git process holds it (`pgrep -qf "[g]it .*$dir"`). Build into the driver so the fleet self-heals. Full recipe: reset-and-branch-sync.md §4.

## Tag-heavy repos: batch-delete with SHORT names

A `git tag -l | while read` loop spawns one `git tag -d` process PER TAG (thousands = minutes). Batch-delete first:

```sh
git tag -l | xargs -n 100 git tag -d
```

`git tag -d` accepts ONLY short names - it rejects `refs/tags/<name>` refnames; a batch run with stderr swallowed (`>/dev/null 2>&1`) fails SILENTLY with zero tags deleted.

## Interrupted parallel runs: the missing status file names the in-flight module

The module whose `<slug>.status` file is absent (or log truncated at its header) was mid-flight when the run died. Re-run `--only <that-module>` - the reset is idempotent and repairs the partial truncation. Do NOT re-run the whole fleet: `pgrep` for live git processes first, let a still-running orphan finish before judging any state.

## Hand-registered submodules are uninitialized until `git submodule init`

Manual registration (manual clone, patch `.gitmodules`, `git add <path>`) leaves `submodule.<name>.url` out of repo config; `git submodule status` shows a leading `-` (not initialized). `git submodule init <path>` clears it. `git submodule add` accepts neither `--filter` nor `--sparse` - sparse/partial submodules MUST be cloned by hand (`git clone --depth N --filter=blob:none --sparse`, then `sparse-checkout set <cone>`) and registered manually. The driver needs a sparse table (`name|cone`) applied on fresh clones AND after every reset. `git gc --aggressive` / `git maintenance run` are safe on `blob:none` promisor clones.

## Verify independently - never trust the script's own summary

A driver reporting "33/33 ok" is a self-report. Per repo assert `@{upstream}` == `origin/<current-branch>`, dirty 0, `remote` exactly `origin`, origin URL == roster URL, one local branch; assert the negatives (no `https://github.com`, no stray `Current`/`Previous`/`Source`). Run `scripts/verify-fleet-sync.sh <fleet-dir>`.

**Always print a count from a verification loop** - zero parsed rows "passes" having checked nothing. Iterate the driver's own roster (`--list | awk '!/^total:/{print $1}'`) - never a directory glob (it drags in non-module dirs; `[ -d "$d/.git" ]` skips submodule-style dirs whose `.git` is a FILE - use `test -e`).

## Remote-sync verification (parent + submodules)

Behind-counts, not prose. Per parent: `git rev-list --count HEAD..<remote>/<branch>` (behind) + the reverse (ahead) - 0/0 = parent needs nothing. Then each submodule:

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

- **Never hardcode the submodule's remote name - read it per checkout (`git -C "$sm" remote -v`)** - the SAME repo can be `Source` in one parent and `origin` in another.
- **Detached checkout with a current gitlink is a no-op for the parent:** recorded gitlink == remote tip → re-attach with `git -C "$sm" checkout -B <branch> <remote>/<branch>` at the same commit; NO bump is staged. Reserve the bump for a genuinely older recorded gitlink.
- Verify: `git branch --show-current` per submodule + `git submodule status` (no `+`) + parent `git status` (no `M <submodule>`).

