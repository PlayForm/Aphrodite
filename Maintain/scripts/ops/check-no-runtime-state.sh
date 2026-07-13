#!/usr/bin/env bash
# repo-guard: fail if runtime state or secrets are accidentally tracked.
# Run from the repo root; exit code 0 = clean, 1 = leak detected.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

leaks=0

# Checks tracked FILE PATHS against `pattern` (e.g. to catch a `.env` file
# that was tracked by name). Do not use this for content scanning - it only
# ever sees `git ls-files`' path list, never file bytes.
check_tracked_path() {
	local pattern="$1"
	local label="$2"
	local matches
	matches=$(git ls-files | grep -E "$pattern" || true)
	if [ -n "$matches" ]; then
		echo "REPO-GUARD: $label found in tracked files:" >&2
		echo "$matches" >&2
		leaks=1
	fi
}

# Checks the CONTENT of every tracked file against `pattern` via `git grep`.
# This is the check a credential-pattern scan actually needs - a secret can
# live inside any tracked file regardless of its name. (A prior version of
# this script called `check_tracked_path` for this job, which greps
# filenames only and therefore never matched a real leaked credential - it
# always reported clean regardless of file contents.)
check_tracked_content() {
	local pattern="$1"
	local label="$2"
	local matches
	matches=$(git grep -InE "$pattern" -- . ':!Maintain/scripts/ops/check-no-runtime-state.sh' 2>/dev/null || true)
	if [ -n "$matches" ]; then
		echo "REPO-GUARD: $label found in tracked file contents:" >&2
		echo "$matches" >&2
		leaks=1
	fi
}

# Profiles: only example/ may be tracked
profile_leaks=$(git ls-files profiles/ | grep -v '^profiles/example/' || true)
if [ -n "$profile_leaks" ]; then
	echo "REPO-GUARD: non-example profile files tracked:" >&2
	echo "$profile_leaks" >&2
	leaks=1
fi

# Secrets: no .env, auth.json, state.db, or credential patterns tracked
check_tracked_path '\.env(\.sh)?$|auth\.json$|state\.db' "secret or state file"
check_tracked_content 'sk-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----' "credential pattern"

exit $leaks
