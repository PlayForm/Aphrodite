#!/bin/sh
# Shared submodule-sync logic for post-checkout/post-merge and the
# package.json `prepare` bootstrap step.
#
# Design (superseding the "pointer-respecting, not --remote-tracking"
# behavior from a 2026-07-13 analysis pass): local dev checkouts should
# always float each submodule (plugins/aphrodite, vendor/headroom,
# vendor/rtk) to its configured tracking branch (`submodule.<name>.branch`
# in .gitmodules, currently `Current` for all three) instead of resetting
# back to whatever SHA happens to be pinned in the superproject's index.
# Whenever that float actually advances a submodule, the resulting pin
# bump is auto-committed here (pathspec-scoped, so it never sweeps up any
# other staged work) - so the pin stays close to each submodule's tip on
# its own, and a fresh clone's plain `git submodule update --init
# --recursive` (no `--remote` needed) lands near-latest too.
#
# Safety - this must NOT repeat the two prior incidents that motivated the
# original dirty-guard (a plain `git submodule update` silently discarding
# submodule work):
#   - A submodule with uncommitted changes is skipped (existing guard).
#   - A submodule whose HEAD is NOT an ancestor of the fetched tracking
#     branch (i.e. it has local commits - committed, just not yet pushed -
#     that aren't upstream yet) is ALSO skipped, never force-reset. Only a
#     clean fast-forward-equivalent move is ever performed.
#   - An in-progress rebase/merge/cherry-pick in the superproject skips the
#     whole thing outright - auto-committing mid-operation is asking for
#     trouble.
# Anything skipped is reported on stderr, never silently dropped.

sync_submodules() {
	repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || return 0
	cd "$repo_root" || return 0
	[ -f .gitmodules ] || return 0

	if [ -e .git/MERGE_HEAD ] || [ -e .git/CHERRY_PICK_HEAD ] || [ -d .git/rebase-merge ] || [ -d .git/rebase-apply ]; then
		echo "githooks: skipping submodule sync - an operation is in progress" >&2
		return 0
	fi

	changed=""
	skipped=""

	all_paths=$(git config -f .gitmodules --get-regexp '\.path$' 2>/dev/null)

	for path in $(echo "$all_paths" | awk '{print $2}'); do
		name=$(echo "$all_paths" | awk -v p="$path" '$2==p{print $1}' | sed -e 's/^submodule\.//' -e 's/\.path$//')
		branch=$(git config -f .gitmodules --get "submodule.$name.branch" 2>/dev/null)

		if [ ! -e "$path/.git" ]; then
			# Not initialized yet - plain init at the pinned SHA first; the
			# float below then advances it if a tracking branch is set.
			git submodule update --init -- "$path" >/dev/null 2>&1
		fi

		if [ -n "$(git -C "$path" status --porcelain 2>/dev/null)" ]; then
			skipped="$skipped $path(uncommitted)"
			continue
		fi

		if [ -z "$branch" ]; then
			# No tracking branch configured for this one - fall back to the
			# old, pointer-respecting behavior instead of guessing.
			git submodule update --init -- "$path" >/dev/null 2>&1
			continue
		fi

		# Resolve the remote to fetch from by matching .gitmodules' own
		# configured url, rather than assuming it's called "origin" - a
		# submodule's "origin" remote can point somewhere else entirely
		# (vendor/rtk's "origin" is the upstream rtk-ai/rtk repo, which has
		# no `Current` branch at all; the fork .gitmodules actually points
		# at is checked out under a remote named "Source"). Falls back to
		# "origin" when no remote matches, preserving the common case.
		sub_url=$(git config -f .gitmodules --get "submodule.$name.url" 2>/dev/null)
		remote=$(git -C "$path" remote -v 2>/dev/null | awk -v u="$sub_url" '$2==u && $3=="(fetch)"{print $1; exit}')
		[ -z "$remote" ] && remote="origin"

		before=$(git -C "$path" rev-parse HEAD 2>/dev/null)
		if ! git -C "$path" fetch --quiet "$remote" "$branch" 2>/dev/null; then
			skipped="$skipped $path(fetch-failed)"
			continue
		fi

		if ! git -C "$path" merge-base --is-ancestor HEAD "$remote/$branch" 2>/dev/null; then
			skipped="$skipped $path(local-commits-not-on-$branch)"
			continue
		fi

		git -C "$path" checkout -B "$branch" "$remote/$branch" --quiet 2>/dev/null

		after=$(git -C "$path" rev-parse HEAD 2>/dev/null)
		if [ "$before" != "$after" ]; then
			changed="$changed $path"
		fi
	done

	if [ -n "$skipped" ]; then
		echo "githooks: submodule sync skipped for:$skipped" >&2
	fi

	if [ -n "$changed" ]; then
		bump_msg="chore(submodules): auto-sync pin(s) to tracking branch tip
"
		for p in $changed; do
			bump_msg="$bump_msg
- $p -> $(git -C "$p" rev-parse --short HEAD)"
		done
		# Pathspec-only commit (git's default when paths are given on the
		# command line): commits ONLY these gitlinks, leaving any other
		# already-staged work exactly as staged - never swept into this.
		# shellcheck disable=SC2086
		if git commit --quiet -m "$bump_msg" -- $changed 2>/dev/null; then
			echo "githooks: auto-committed submodule pin bump for:$changed" >&2
		fi
	fi

	return 0
}

# Allow direct invocation (e.g. `sh .githooks/lib/sync-submodules.sh` from
# package.json's `prepare` script) as well as sourcing from another hook.
case "$0" in
	*sync-submodules.sh) sync_submodules ;;
esac
