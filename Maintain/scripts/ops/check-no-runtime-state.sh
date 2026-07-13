#!/usr/bin/env bash
# repo-guard: fail if runtime state or secrets are accidentally tracked.
# Run from the repo root; exit code 0 = clean, 1 = leak detected.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

leaks=0

check_tracked() {
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

# Profiles: only example/ may be tracked
profile_leaks=$(git ls-files profiles/ | grep -v '^profiles/example/' || true)
if [ -n "$profile_leaks" ]; then
	echo "REPO-GUARD: non-example profile files tracked:" >&2
	echo "$profile_leaks" >&2
	leaks=1
fi

# Secrets: no .env, auth.json, state.db, or credential patterns tracked
check_tracked '\.env(\.sh)?$|auth\.json$|state\.db' "secret or state file"
check_tracked 'sk-[A-Za-z0-9_-]{20,}|ghp_|gho_|xox[baprs]-|AKIA[0-9A-Z]{16}|-----BEGIN' "credential pattern in tracked file content"

exit $leaks
