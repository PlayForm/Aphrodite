# Fleet remote hygiene: worked recipes

Reference for SKILL.md ("Repairing remotes fleet-wide"). Each item is a
worked instance of a rule; the rule itself lives in SKILL.md. All commands
are byte-stable.

## The empty-Parent-remote bug, fully

The repo-fleet remote setup is scripted: machine-local fleet wrapper scripts
(original path omitted) call `Fn/Configure/{Remote,Fetch,Branch}.sh` plus
`Fn/Cache.sh`. Work in the canonical originals (the call target);
`Fn/Cache.sh` exists as a DUPLICATE in both trees - patch BOTH copies or one
keeps the bug.

Mechanism of the bug: `gh repo view --json parent` emits JSON `null` for
non-forks. An empty failure branch makes `Parent="$OwnerParent/$NameParent"`
= `"/"`, which passes `[ "$Parent" != "null/null" ]`, and git records a
host-only remote (`ssh://git@github.com/.git`).

Failure branches (Cache.sh): `OwnerParent="null"` / `NameParent="null"`.

Guards (every guard checks BOTH parts non-empty AND non-null):

```sh
[ -n "$OwnerParent" ] && [ "$OwnerParent" != "null" ] && [ -n "$NameParent" ] && [ "$NameParent" != "null" ]
```

Fetch.sh: fetch `Parent` only when `git remote get-url Parent` returns
non-empty.

Any `git remote add <name> "$(...)"`: capture the substitution, guard
non-empty, and `git remote remove <name>` before `add` (avoids
duplicate-add errors too).

## Sweeping already-broken repos

Normalize every remote URL with `sed -E 's#\.git$##; s#/+$##'` and remove
any that collapse to host-only or empty: `ssh://git@github.com`,
`git@github.com:`, `https://github.com`, `http://github.com`, empty. Legit
upstream Parents (e.g. ohmyzsh/ohmyzsh) survive.

Confirm fork status via
`gh api repos/<owner>/<repo> --jq '.fork,.parent.full_name'`: `fork=false`
means remove a stray `Parent` remote outright.

## macOS bash 3.2: no `mapfile`, pipe-subshell counters vanish

macOS bash 3.2 has NO `mapfile`/`readarray`, and counters inside
`find | while` pipe subshells vanish (everything reports 0). Use file-based
loops:

```sh
find ... > list
while IFS= read -r g; do ...; done < list
```

## Functional test (non-fork)

Throwaway `git init` dir with no remote: `gh repo view` fails → Cache.sh
must yield `OwnerParent=null NameParent=null`; run Remote.sh →
`git remote -v` must show only `Source`; Fetch.sh must exit 0. Proves the
guard without touching a real repo.
