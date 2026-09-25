# Submodule Fleet Reset & Branch-Name Sync

Reference for bulk-resetting a directory of reference/documentation repos to
pristine upstream state, keeping local branch names identical to upstream
defaults. Origin case: a bulk-reset script driving 44 documentation repos
used as local-LLM references.

---

## 1. Resolve the upstream default branch — never assume `main`

A hardcoded `main`, or `git remote show origin | awk '/HEAD branch/'`, is
fragile. `remote show` needs the network every time and its output is
locale/format-sensitive. Use the symref, with layered fallbacks:

```bash
remote_default_branch() {
	local dir="$1" a b rest ref cand
	# 1. authoritative: ask the remote
	while read -r a b rest; do
		if [ "$a" = "ref:" ]; then
			printf '%s\n' "${b#refs/heads/}"
			return 0
		fi
	done < <(git -C "$dir" ls-remote --symref origin HEAD 2> /dev/null)

	# 2. offline: cached origin/HEAD
	ref="$(git -C "$dir" symbolic-ref --short refs/remotes/origin/HEAD 2> /dev/null)"
	[ -n "$ref" ] && {
		printf '%s\n' "${ref#origin/}"
		return 0
	}

	# 3. last resort: well-known names
	for cand in main master trunk production develop; do
		git -C "$dir" rev-parse --verify -q "refs/remotes/origin/$cand" \
			> /dev/null 2>&1 && {
			printf '%s\n' "$cand"
			return 0
		}
	done
	return 1
}
```

Then cache it so later runs resolve offline:
`git -C "$dir" remote set-head origin "$default"`

**Real defaults this caught in one 33-repo fleet** — proof that assuming `main`
would have silently mangled four repos:

| repo                | default      |
| ------------------- | ------------ |
| `cloudflare-docs`   | `production` |
| `grpc`, `grpc-node` | `master`     |
| `tauri-docs`        | `v2`         |
| everything else     | `main`       |

## 2. Force local branch NAME to equal the upstream name

Tracking the right commit is not enough — the local branch must be _named_ the
same, or push targets and tooling drift. Custom local nomenclature (e.g. a
`Current` branch someone set up years ago) must be replaced by the upstream
name:

```bash
git -C "$dir" checkout --force -B "$default" "refs/remotes/origin/$default"
git -C "$dir" branch --set-upstream-to="origin/$default" "$default"
git -C "$dir" reset --hard "refs/remotes/origin/$default"
git -C "$dir" clean -dffx
```

`-B` creates-or-resets in one step and works even from a detached HEAD.
Note `clean -dffx` uses double `-f`: a single `-f` refuses to delete nested
directories that are themselves git repos.

For every _other_ local branch: set upstream if a same-named remote branch
exists, otherwise it is local-only. Deleting local-only branches by default is
what actually keeps names mirrored; expose `--keep-local-branches` as the
escape hatch rather than making deletion opt-in.

## 3. `origin` must be the only remote

Fleets accumulate stray remotes (`Source`, `Parent`, sometimes a malformed
`ssh://git@github.com/.git`). Sweep them:

```bash
while read -r rem; do
	[ "$rem" = "origin" ] && continue
	git -C "$dir" remote remove "$rem" && echo "removed stray remote: $rem"
done < <(git -C "$dir" remote)
```

## 4. Stale `index.lock` recovery

A killed/interrupted git run leaves `index.lock` behind and every later
checkout/reset in that repo fails with _"Another git process seems to be
running"_. In a submodule the lock lives under the **superproject**:
`<super>/.git/modules/<path>/index.lock` — not in the submodule directory.

Locate it robustly and only clear it when no git process actually holds it:

```bash
gitdir="$(git -C "$dir" rev-parse --absolute-git-dir 2> /dev/null)"
for lock in "$gitdir/index.lock" "$gitdir/HEAD.lock"; do
	if [ -n "$gitdir" ] && [ -f "$lock" ]; then
		if ! pgrep -qf "[g]it .*$dir" 2> /dev/null; then
			rm -f "$lock" && echo "cleared stale $(basename "$lock")"
		else
			echo "!!! $(basename "$lock") held by a live git process"
			return 1
		fi
	fi
done
```

The `[g]it` bracket trick stops `pgrep` matching its own command line.
Before deleting, record evidence (`ls -la "$lock" > /tmp/hermes/...`) — a
0-byte lock dated weeks ago is unambiguously stale.

---

## Pitfalls

### `git -C <dir> rev-parse --git-dir` is NOT a repo test

It **succeeds for a plain subdirectory** of a git repo, because git walks _up_
to the enclosing repository. Using it to enumerate a fleet reports ordinary
content directories as repos and prints the parent's branch/remote for them.

In the origin session this falsely flagged three plain content dirs as repos
sharing one odd remote, which looked like three misconfigured submodules. They
were just tracked folders of the superproject.

Test for an actual repo root instead:

```bash
test -e "$dir/.git" # file (submodule) or directory (normal clone)
```

Cross-check the fleet list against reality in **both** directions:

```bash
find . -maxdepth 3 -name .git | sed 's|^\./||; s|/\.git$||' | sort > on-disk
comm -13 in-script on-disk # on disk, missing from the script
comm -23 in-script on-disk # in the script, not on disk
```

Note `-maxdepth 3`: nested submodules (`wezterm/wezterm`) sit one level deeper
than the rest and a `maxdepth 2` scan misses them entirely.

### Audit "dirty"/"unpushed" before hard-resetting anything

A reset script is destructive. Do not trust the raw counts — classify them:

- **3,595 "modifications"** turned out to be `D ` staged _deletions of upstream
  files_. A reset restores them; nothing was authored locally.
- **13 "unpushed commits"** were all upstream authors (renovate bot, project
  bot, outside contributors) sitting on a branch whose tracking ref had gone
  stale — not local work.

Distinguishing probes:

```bash
git -C "$d" status --porcelain --untracked-files=no      # tracked only
git -C "$d" log --branches --not --remotes --format='%an <%ae>' | sort -u
git -C "$d" merge-base --is-ancestor <sha> origin/<default> && echo CONTAINED
```

Only after that classification is a `--hard` reset safe to run. Save ref tips
to `/tmp` first as evidence:
`git for-each-ref --format='%(refname) %(objectname)' > /tmp/hermes/<r>-refs-before.txt`

### Parallelism needs per-module log files

Interleaved parallel output is unreadable and, worse, unattributable. Write
each module to `$LOGDIR/<slug>.log` plus a `<slug>.status` file, then replay
logs in list order and derive pass/fail from the status files. Slugify nested
paths (`wezterm/wezterm` → `wezterm_wezterm`) or the log path breaks.

Do not use `set -e` in a fleet driver — one repo failing must not abort the
other 32. Use `set -uo pipefail` and explicit `|| return 1` per step.

### Report failures from structured state, not scraped text

The failing step's stderr is often redirected into a per-module log, so the
driver's own output shows only the summary. Derive the failed list from
`.status` files and exit non-zero, so callers branch on the exit code instead
of grepping for `!!!`. In the origin session the per-module log simply ended
mid-fetch with no error text; the actual cause (`index.lock`) was only found by
re-running the failing git command directly in that repo.

---

## Verification (run before declaring done)

Never trust the script's own "33/33 ok" — verify independently. See
`scripts/verify-fleet-sync.sh`, which asserts per repo:
upstream == `origin/<branch>`, dirty == 0, remotes == exactly `origin`, origin
URL == roster URL, exactly one local branch, and no `https://github.com`.

Also assert the negative: no stray custom branch name (`Current`/`Previous`/
`Source`) remains anywhere in the fleet.

## Useful flags for a fleet driver

`--list` (print roster + count, exit), `--dry-run` (resolve defaults and show
name drift without touching anything), `--only a,b`, `--jobs N`,
`--keep-local-branches`. `--dry-run` is the cheap way to spot branch-name drift
before any destructive step — it is what surfaced `Current` vs upstream `v2`.

When parsing `--list` output in a verification loop, remember it prints
space-separated columns and a trailing `total: N` line; a `grep '|'` filter
silently yields zero rows, and a naive `awk '{print $1}'` feeds `total:` in as
a bogus module name. Iterate the ROSTER (`./Reset.sh --list | awk
'!/^total:/{print $1}'`), never a directory glob - a glob includes non-module
dirs and a `[ -d .git ]` gate skips every submodule-style dir (its `.git` is a
FILE), silently checking a fraction of the fleet (44-module fleet -> 25).
**Always print a count from verification loops so an empty pass is visible.**

---

## Adding a module to the fleet - three registries, one git

The Documentation/Module fleet keeps THREE overlapping lists in sync whenever
a module is added:

1. `Reset.sh` MODULES - the operational roster: `name|ssh://git@github.com/...`
   lines, case-insensitive alphabetical, SSH URLs ONLY (never https); nested
   paths use `dir/name`.
2. `Module/.gitmodules` - a CURATED parallel list (every entry carries
   `ignore = all`). Git does NOT read it; it mirrors the roster. Same URLs,
   same order.
3. `Documentation/.gitmodules` (repo root) - the ONLY registry git reads
   (`path = Module/<name>`). `git submodule add <url> Module/<name>` writes it,
   clones, and initializes in one step; manual registration means patching it,
   `git add Module/<name>`, then `git submodule init Module/<name>` - skip the
   init and `git submodule status` shows a `-` prefix forever.

Verify upstream BEFORE adding: `git ls-remote ssh://git@github.com/<o>/<r>.git
HEAD`. A 404 means deleted or private - GitHub redirects renames, so 404 is
never a rename. For a dead listed repo, search GitHub for a same-name living
equivalent and ASK the user (substitute candidates vs skip) before registering;
register the substitute under the ORIGINAL local name.

Normal module: `git submodule add <url> Module/<name>` (it clones FULL history
first; Reset.sh truncates it later). Sparse module - the fleet's
`SPARSE_MODULES` table is `name|subdir`, currently `cpython|Lib/ctypes` - clone
by hand (`git clone --depth 1 --no-tags --filter=blob:none --sparse`, then
`sparse-checkout set <subdir>`) because `git submodule add` accepts neither
`--filter` nor `--sparse`. Cone mode materializes top-level files plus the
cone dir - expected, not a defect.

## Reset + maintenance recipe (user's standing expectations for this fleet)

- Depth 1-2, size-minimal: `./Reset.sh --only <names> --depth 1` (default
  depth is 2). Upstream default branches resolve dynamically - `pypdfium2` is
  a real default branch; never assume `main`/`master`.
- Advanced maintenance per module: `git maintenance run` -> `git reflog expire
--expire=now --expire-unreachable=now --all` -> `git gc --aggressive
--prune=now`. All three are safe on `blob:none` promisor clones (the filter
  persists via `remote.<name>.promisor`/`partialclonefilter`).
- 1-to-1 with remote: final `git fetch origin <br>` + `reset --hard
origin/<br>`, then assert `HEAD == origin/<default>`, `status --porcelain`
  empty, `rev-list --count HEAD` == 1, zero tags.
- Never commit - stage gitlinks only; the user's Save tool sweeps. A full
  reset moves every module tip, so the parent index gitlinks lag afterward;
  that is expected and left for the sweep.
