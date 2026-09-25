#!/usr/bin/env bash
#
# Verify every repo in a fleet is genuinely shallow at depth N.
#
# Complements verify-fleet-sync.sh (branch/remote/tracking state); this one
# only answers "is the history actually truncated, and did gc reclaim it?".
#
# Usage:
#   verify-fleet-depth.sh <fleet-dir> [depth]     # depth defaults to 2
#
# Why this exists: a fleet can report shallow=YES on every repo and still be
# carrying full history. A shallow graft constrains only FUTURE fetches --
# commits reachable from other branch tips, tags, or reflogs stay on disk, so
# gc frees nothing. The honest test is per-repo graft depth + ref narrowness.
#
# Per repo it asserts:
#   * .git/shallow exists
#   * first-parent depth <= N
#   * 0 tags (tags pin commits independently of branches)
#   * exactly 1 local branch and <= 1 remote-tracking ref
#
# CRITICAL: do NOT assert with `rev-list --count HEAD`. It counts ALL reachable
# commits, so a merge commit at HEAD makes a correct depth-2 repo report 3 --
# both parents legitimately sit at the shallow boundary. That is expected git
# behaviour, not a defect. First-parent count is the honest measure.
#
# Exit 0 only if every repo passes. Always prints a checked count so a failed
# discovery cannot masquerade as a clean pass.

set -uo pipefail

FLEET="${1:-}"
DEPTH="${2:-2}"

if [ -z "$FLEET" ] || [ ! -d "$FLEET" ]; then
	echo "usage: $(basename "$0") <fleet-dir> [depth]" >&2
	exit 2
fi

case "$DEPTH" in
'' | *[!0-9]*)
	echo "depth must be a positive integer" >&2
	exit 2
	;;
esac
[ "$DEPTH" -lt 1 ] && {
	echo "depth must be >= 1" >&2
	exit 2
}

cd "$FLEET" || exit 2

CHECKED=0
BAD=0
TOTAL_KB=0

# test -e <dir>/.git is the correct repo probe: `rev-parse --git-dir` also
# succeeds inside plain subdirectories because git walks UP to the parent repo.
# maxdepth 3 catches nested submodules sitting one level deeper.
while IFS= read -r g; do
	n="${g#./}"
	n="${n%/.git}"
	[ -e "$n/.git" ] || continue
	CHECKED=$((CHECKED + 1))

	flags=""
	gd="$(git -C "$n" rev-parse --absolute-git-dir 2>/dev/null)"

	if [ -z "$gd" ]; then
		printf 'BAD  %-26s NO_GITDIR\n' "$n"
		BAD=$((BAD + 1))
		continue
	fi

	if [ ! -f "$gd/shallow" ]; then
		flags="$flags NOT_SHALLOW"
		graft=0
	else
		graft="$(wc -l <"$gd/shallow" | tr -d ' ')"
	fi

	# honest depth: first-parent only
	fp="$(git -C "$n" rev-list --count --first-parent HEAD 2>/dev/null)"
	if [ -z "$fp" ]; then
		flags="$flags NO_HEAD"
	elif [ "$fp" -gt "$DEPTH" ]; then
		flags="$flags TOO_DEEP(first_parent=$fp)"
	fi

	nt="$(git -C "$n" tag -l 2>/dev/null | wc -l | tr -d ' ')"
	[ "$nt" = "0" ] || flags="$flags TAGS($nt)"

	nb="$(git -C "$n" for-each-ref refs/heads 2>/dev/null | wc -l | tr -d ' ')"
	[ "$nb" = "1" ] || flags="$flags LOCAL_BRANCHES($nb)"

	nr="$(git -C "$n" for-each-ref refs/remotes 2>/dev/null | wc -l | tr -d ' ')"
	[ "$nr" -le 1 ] || flags="$flags REMOTE_REFS($nr)"

	# total commit count is informational only -- a merge at HEAD inflates it
	allc="$(git -C "$n" rev-list --count HEAD 2>/dev/null)"
	kb="$(du -sk "$gd" 2>/dev/null | cut -f1)"
	[ -n "$kb" ] && TOTAL_KB=$((TOTAL_KB + kb))

	if [ -n "$flags" ]; then
		BAD=$((BAD + 1))
		printf 'BAD  %-26s graft=%-3s fp=%-3s%s\n' "$n" "$graft" "${fp:-?}" "$flags"
	else
		printf 'ok   %-26s graft=%-3s fp=%-3s reachable=%-4s %sM\n' \
			"$n" "$graft" "$fp" "${allc:-?}" "$((kb / 1024))"
	fi
done < <(find . -maxdepth 3 -name .git 2>/dev/null | sort)

echo "-------------------------------------------------------------"
printf 'checked: %d   clean: %d   bad: %d   depth<=%s   git total: %sM\n' \
	"$CHECKED" "$((CHECKED - BAD))" "$BAD" "$DEPTH" "$((TOTAL_KB / 1024))"

if [ "$CHECKED" -eq 0 ]; then
	echo "NOTHING CHECKED — no repos found under $FLEET (false green guard)" >&2
	exit 2
fi

if [ "$BAD" -gt 0 ]; then
	echo
	echo "If a repo is shallow but still large, the size may be the CURRENT TREE," >&2
	echo "not history. Check:  git -C <repo> ls-tree -r -l HEAD | sort -k4 -n | tail" >&2
	exit 1
fi
exit 0
