#!/usr/bin/env bash
# lib/bump-submodule-gitlinks.sh - shared by post-commit / post-merge /
# post-checkout / pre-push. Runs in the MAIN parent repository ONLY.
#
# After any parent git action, if a tracked submodule's recorded gitlink
# differs from HEAD, stage and commit the pointer update so the parent never
# leaves an orphaned/stale submodule pointer.
#
# Guards (each verified against the old hook's phantom-gitlink failure):
#  1. Skip entirely when this repo is itself a submodule (a superproject
#     exists) - vendor/ and plugins/ checkouts NEVER run this.
#  2. Only ever stage paths declared in .gitmodules whose index entry is a
#     genuine gitlink (mode 160000); never `git add .`, never computed
#     paths, never files or directories that are not real submodules.
#  3. Recursion guard (GITLINK_BUMP_ACTIVE) so the nested commit cannot
#     re-enter post-commit.
#  4. Skip while the parent is mid-merge / mid-cherry-pick / mid-rebase.
#  5. No-op when no pointer has changed (honors `ignore = dirty`, so a
#     dirty submodule worktree is never mistaken for a pointer update).
set -euo pipefail

# Guard 3: recursion - the nested commit below re-enters post-commit.
if [[ -n "${GITLINK_BUMP_ACTIVE:-}" ]]; then
	exit 0
fi

# Guard 1: this hook belongs to the MAIN parent only. Inside a submodule
# (vendor/*, plugins/*) a superproject exists - skip.
if [[ -n "$(git rev-parse --show-superproject-working-tree 2> /dev/null || true)" ]]; then
	exit 0
fi

ROOT="$(git rev-parse --show-toplevel)"

[[ -f "$ROOT/.gitmodules" ]] || exit 0

# Guard 4: mid-operation states - the parent's own commit will record us.
GITDIR="$(git rev-parse --absolute-git-dir)"
for STATE in MERGE_HEAD CHERRY_PICK_HEAD REBASE_HEAD; do
	if [[ -f "$GITDIR/$STATE" ]]; then
		exit 0
	fi
done

# Guard 2: enumerate REAL submodules - .gitmodules paths whose index entry
# is a genuine gitlink (mode 160000). Anything else is never touched.
SUBMODULES=()
while IFS=' ' read -r key path; do
	[[ -n "$path" ]] || continue
	mode="$(git ls-files -s -- "$path" 2> /dev/null | awk '{print $1}')"
	[[ "$mode" == "160000" ]] || continue
	SUBMODULES+=("$path")
done < <(git config -f "$ROOT/.gitmodules" --get-regexp '\.path$' 2> /dev/null)

# Guard 5: only changed pointers. `git diff HEAD` honors `ignore = dirty`,
# so a dirty submodule worktree is never mistaken for a pointer update.
DIRTY=()
for SUB in "${SUBMODULES[@]:-}"; do
	if ! git diff --quiet HEAD -- "$SUB" 2> /dev/null; then
		DIRTY+=("$SUB")
	fi
done

if [[ ${#DIRTY[@]} -eq 0 ]]; then
	exit 0
fi

export GITLINK_BUMP_ACTIVE=1

git add -- "${DIRTY[@]}"

DETAILS=""
for SUB in "${DIRTY[@]}"; do
	NEW="$(git ls-files -s -- "$SUB" 2> /dev/null | awk '{print $2}')"
	DETAILS="$DETAILS $SUB@${NEW:0:7}"
done

# `--only` + explicit paths: commit ONLY the gitlink updates, never sweep
# other staged changes into the mechanical bump. `|| true`: a transient
# failure (e.g. the external auto-committer holding the index) converges on
# the next parent git action.
git commit -o -m "chore: bump submodule gitlinks:$DETAILS" --no-verify -s -- "${DIRTY[@]}" > /dev/null 2>&1 || true
