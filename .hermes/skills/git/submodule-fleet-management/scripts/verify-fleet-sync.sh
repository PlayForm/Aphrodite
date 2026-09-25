#!/usr/bin/env bash
#
# Independently verify a submodule fleet is fully synced to upstream.
# Does NOT trust any reset driver's own "N/N ok" self-report — re-derives
# every fact from git.
#
# Usage:
#   verify-fleet-sync.sh <fleet-dir> [roster-file]
#
#   <fleet-dir>    directory containing the repos (e.g. .../Documentation/Module)
#   [roster-file]  optional "name<TAB|space>url" list. When given, origin URLs
#                  are also checked against it. Without it, URLs are only
#                  checked for scheme policy.
#
# Per repo it asserts:
#   * @{upstream} == origin/<current branch>   (name-level sync, not just commit)
#   * worktree is clean (0 porcelain lines)
#   * "origin" is the ONLY remote
#   * exactly ONE local branch (no stray Current/Previous/Source)
#   * no https://github.com remote (SSH policy for GitHub hosts)
#   * origin URL matches the roster, when a roster is supplied
#
# Exit 0 only if every repo passes. Always prints a checked count, so an
# empty/failed parse can never masquerade as a clean pass.

set -uo pipefail

FLEET="${1:-}"
ROSTER="${2:-}"

if [ -z "$FLEET" ] || [ ! -d "$FLEET" ]; then
	echo "usage: $(basename "$0") <fleet-dir> [roster-file]" >&2
	exit 2
fi

cd "$FLEET" || exit 2

declare -A ROSTER_URL=()
if [ -n "$ROSTER" ]; then
	if [ ! -f "$ROSTER" ]; then
		echo "roster file not found: $ROSTER" >&2
		exit 2
	fi
	while read -r rn ru; do
		[ -z "${rn:-}" ] && continue
		case "$rn" in total:* | \#*) continue ;;
		esac
		[ -n "${ru:-}" ] && ROSTER_URL["$rn"]="$ru"
	done <"$ROSTER"
fi

# Discover real repos. test -e <dir>/.git is the correct probe:
# `git rev-parse --git-dir` succeeds for plain subdirectories too, because git
# walks UP to the enclosing repository. maxdepth 3 catches nested submodules.
REPOS=()
while IFS= read -r g; do
	r="${g#./}"
	r="${r%/.git}"
	REPOS+=("$r")
done < <(find . -maxdepth 3 -name .git 2>/dev/null | sort)

CHECKED=0
BAD=0

for n in "${REPOS[@]}"; do
	[ -e "$n/.git" ] || continue
	CHECKED=$((CHECKED + 1))

	b="$(git -C "$n" rev-parse --abbrev-ref HEAD 2>/dev/null)"
	up="$(git -C "$n" rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || echo NONE)"
	d="$(git -C "$n" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
	r="$(git -C "$n" remote | paste -sd, -)"
	u="$(git -C "$n" remote get-url origin 2>/dev/null || echo NONE)"
	nb="$(git -C "$n" for-each-ref refs/heads | wc -l | tr -d ' ')"

	flags=""
	[ "$up" = "origin/$b" ] || flags="$flags UPSTREAM_MISMATCH($up)"
	[ "$d" = "0" ] || flags="$flags DIRTY($d)"
	[ "$r" = "origin" ] || flags="$flags REMOTES($r)"
	[ "$nb" = "1" ] || flags="$flags LOCAL_BRANCHES($nb)"
	case "$u" in
	https://github.com/*) flags="$flags HTTPS_GITHUB" ;;
	esac
	if [ -n "${ROSTER_URL[$n]:-}" ] && [ "$u" != "${ROSTER_URL[$n]}" ]; then
		flags="$flags URL_MISMATCH"
	fi

	# stray custom branch names anywhere in the repo
	stray="$(git -C "$n" for-each-ref --format='%(refname:short)' refs/heads |
		grep -Ex 'Current|Previous|Source' | paste -sd, -)"
	[ -n "$stray" ] && flags="$flags STRAY_BRANCH($stray)"

	if [ -n "$flags" ]; then
		BAD=$((BAD + 1))
		printf 'BAD  %-26s %-12s%s\n' "$n" "$b" "$flags"
	else
		printf 'ok   %-26s %-12s %s\n' "$n" "$b" "$up"
	fi
done

echo "-------------------------------------------------------------"
printf 'checked: %d   clean: %d   bad: %d\n' \
	"$CHECKED" "$((CHECKED - BAD))" "$BAD"

if [ "$CHECKED" -eq 0 ]; then
	echo "NOTHING CHECKED — no repos discovered under $FLEET (false green guard)" >&2
	exit 2
fi
[ "$BAD" -eq 0 ] || exit 1
exit 0
