# Truncating a fleet to shallow depth N (verified recipe)

Goal: documentation-reference mirrors keep only the last N commits. Full
history is dead weight. Verified on a 33-repo fleet: **5.4G → 4.1G**
(RooCode `.git` 623M→174M, oxc 207M→14M, roto 4.5M→936K).

## The trap that makes this look done when it isn't

A first attempt using `fetch --depth 2` across all branches reported
`shallow=YES` for all 33 repos and exit 0 — **and freed almost nothing**
(5.4G→4.4G, mostly incidental). `rev-list --count HEAD` still showed 7031
commits in one repo.

**A shallow graft only constrains FUTURE fetches.** History already on disk
stays alive as long as _any_ ref reaches it. Fetching
`+refs/heads/*:refs/remotes/origin/*` keeps every branch tip, and each tip
anchors its full ancestry. Tags anchor commits independently. Reflogs pin
unreachable commits so `gc` frees zero bytes.

So shallowness is not one flag — it is: **narrow the refs, drop the tags,
expire the reflogs, then gc.**

## Correct order (order is load-bearing)

```bash
default=$(git ls-remote --symref origin HEAD | awk '/^ref:/{print $2}' | sed 's|refs/heads/||')

# 1. delete every remote ref EXCEPT the default branch -- BEFORE fetching
git for-each-ref --format='%(refname)' refs/remotes \
	| grep -v "^refs/remotes/origin/$default$" \
	| while read -r r; do git update-ref -d "$r"; done

# 2. tags pin commits independently of branches
git tag -l | while read -r t; do git tag -d "$t"; done

# 3. make the narrowness persistent for future fetches
git config remote.origin.fetch "+refs/heads/$default:refs/remotes/origin/$default"
git config remote.origin.tagOpt --no-tags

# 4. now fetch shallow
git fetch origin --depth 2 --no-tags --force \
	"+refs/heads/$default:refs/remotes/origin/$default"

# 5. a fetch can still auto-follow a tag into the kept commits
git tag -l | while read -r t; do git tag -d "$t"; done

# 6. delete non-default LOCAL branches too -- their tips pin full history

# 7. reclaim. Skipping the reflog expiry frees NOTHING.
git reflog expire --expire=now --expire-unreachable=now --all
git gc --prune=now --quiet
```

### Ordering pitfall that cost a run

Deleting the other remote refs **after** the fetch (instead of before) deleted
`origin/<default>` itself, leaving `[origin/main: gone]` and a broken
`@{upstream}`. Symptom: `git for-each-ref refs/remotes` returns 0 refs and
`rev-parse --abbrev-ref --symbolic-full-name '@{upstream}'` exits 128.
Delete-then-fetch, so the fetch recreates the one ref you want.

Related: a `grep -v` filter that matches nothing silently deletes everything
it was supposed to protect. Anchor the pattern (`^...$`) and prefer an explicit
`[ "$r" = "refs/remotes/origin/$default" ] && continue`.

## Fresh clones

```bash
git clone --depth 2 --no-tags --shallow-submodules --recursive "$url" "$dir"
git submodule update --init --recursive --force --depth 2 # fall back without --depth
```

## Verifying depth — do NOT use `rev-list --count HEAD`

`rev-list --count HEAD` counts _all_ reachable commits, so a merge commit at
HEAD makes a correct depth-2 repo report **3**. Three repos in the fleet showed
3 and were all correct: HEAD was a merge, so depth 2 legitimately includes both
parents.

Authoritative checks:

```bash
wc -l < "$(git rev-parse --absolute-git-dir)/shallow" # graft entries == depth
git rev-list --count --first-parent HEAD              # <= N
test -f "$(git rev-parse --absolute-git-dir)/shallow" # is it shallow at all
```

An unshallow escape hatch is worth building in (`--full`): restore
`remote.origin.fetch` to `+refs/heads/*:...`, unset `tagOpt`, then
`git fetch origin --unshallow --tags --force`.

## Interpreting the size that remains

Depth only shrinks **history**. A large `.git` after truncation can be a large
_current tree_: RooCode stayed at 174M because `static/img/**.gif` holds
multi-MB GIFs (one 15MB) in the checked-out commit. Diagnose before promising
more savings:

```bash
git ls-tree -r -l HEAD | sort -k4 -n | tail -3 # biggest blobs at HEAD
du -sh .git/objects
```

Only a sparse checkout excludes those — no depth setting will.

## Progress monitoring during a parallel run

The driver buffers each module's output to `$LOGDIR/<slug>.log` and replays it
only at the end, so `grep '    ok:' <log>` returns 0 for the whole run and
looks stalled. `gc` on multi-hundred-MB repos takes minutes. Track progress
with `du -sh <fleet-dir>` (the number falls as repacks land) rather than
tailing the log, and let `notify_on_complete` deliver the exit code.
