#!/bin/bash
# auto-release.sh - commit all changes, bump version, build, tag, release
# auto-release.sh - commit, bump, build, tag, push - full pipeline
# Usage: ./scripts/auto-release.sh [--minor] ["commit message"]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CARGO_TOML="$REPO_ROOT/crates/aphrodite/Cargo.toml"
REMOTE="${GIT_REMOTE:-origin}"
FAILURES=()

# Parse --minor before capturing the commit message, so `--minor` never gets
# treated as the message itself.
BUMP=patch
if [ "${1:-}" = "--minor" ]; then
	BUMP=minor
	shift
fi
MSG="${1:-}"

cd "$REPO_ROOT"

# Sync submodules to their remote tracking branches - OPT-IN ONLY
# (report 08 F3/T4): floating all three submodules to their remote branch
# tips at release time meant a release's actual contents (whatever landed
# upstream by release day) could silently differ from anything anyone
# reviewed. Default is now "release exactly what is pinned" - set
# SYNC_SUBMODULES=1 to deliberately float pins as part of this release, and
# the before/after `git submodule status` makes the pin bump visible in the
# release log instead of riding along silently.
if [ "${SYNC_SUBMODULES:-0}" = "1" ]; then
	echo "[submodule] SYNC_SUBMODULES=1 - syncing to remote branch tips"
	echo "[submodule] before:"
	git submodule status
	git submodule update --remote --recursive --merge || echo "[submodule] sync skipped (offline or no remote)"
	echo "[submodule] after:"
	git submodule status
else
	echo "[submodule] SYNC_SUBMODULES not set - releasing exactly what is pinned (no float)"
fi
# (vendor submodule handled by git submodule)

# Stage all changes
git add -u
git add docs/ Maintain/scripts/ plugins/aphrodite/ 2>/dev/null || true

# Use provided message or auto-generate from last commit
if [ -z "$MSG" ]; then
	MSG=$(git log -1 --format=%s)
	# Strip any existing release prefix
	MSG=$(echo "$MSG" | sed 's/^release(aphrodite): //')
fi

# Commit if there are changes
if ! git diff --cached --quiet; then
	git commit -m "$MSG"
	echo "[commit] $MSG"
else
	echo "[skip] nothing to commit"
fi

# Read current version, bump patch (or minor if --minor was passed)
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ "$BUMP" = "minor" ]; then
	NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2+1".0"}')
else
	NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
fi

# Bump Cargo.toml (line 3 only - avoid matching dependency versions)
sed -i '' "3s/version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO_TOML"
# Bump aphrodite-hermes Cargo.toml - package version + dependency
HERMES_TOML="$REPO_ROOT/crates/aphrodite-hermes/Cargo.toml"
sed -i '' "3s/version = \"$CURRENT\"/version = \"$NEW\"/" "$HERMES_TOML"
sed -i '' "s/aphrodite = { path = \"..\/aphrodite\", version = \"$CURRENT\"/aphrodite = { path = \"..\/aphrodite\", version = \"$NEW\"/" "$HERMES_TOML"
echo "[bump] aphrodite-hermes Cargo.toml → $NEW"

# Sync the README release badge + example health output, and package.json,
# to the new binary version. These track the binary version track (not the
# plugin version track), so they're bumped unconditionally here rather than
# inside the plugin-version block below.
#
# Pattern-based, not $CURRENT-based (F15): a $CURRENT-anchored sed silently
# matches nothing - not an error, just a no-op - the moment a string has
# already drifted out of sync with Cargo.toml's version (e.g. from a manual
# edit, or a prior run of this same script that missed a spot). Matching the
# version-shaped pattern instead means every run self-heals any prior drift,
# rather than permanently orphaning whatever string wasn't $CURRENT.
sed -i '' -E "s/release-v[0-9]+\.[0-9]+\.[0-9]+-blue/release-v$NEW-blue/" README.md
sed -i '' -E "s/\"version\":\"v[0-9]+\.[0-9]+\.[0-9]+\"/\"version\":\"v$NEW\"/" README.md
# The proxy /health example prints a bare version (no `v` prefix), so the
# `v`-anchored pattern above skips it. Bumped separately rather than by
# loosening that pattern - dropping the `v` there would also rewrite unrelated
# `"version":"..."` JSON samples elsewhere in the README.
sed -i '' -E "s/\"version\":\"[0-9]+\.[0-9]+\.[0-9]+\"/\"version\":\"$NEW\"/" README.md
sed -i '' "3s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$REPO_ROOT/package.json"
echo "[bump] README release badge + package.json → $NEW"

# Post-bump stale-string guard: catch any $CURRENT reference the sed passes
# above should have caught but somehow didn't (a genuine bug in this script),
# rather than letting it silently ship - a stale version string in a shipped
# release is exactly the failure mode F15 exists to prevent.
# Matches bare $CURRENT too, not just v$CURRENT: the /health example that
# escaped the `v`-anchored sed above would also have escaped a `v`-anchored
# guard, which is how it survived several releases unnoticed.
if grep -rn "$CURRENT" README.md "$REPO_ROOT/package.json" 2>/dev/null; then
	echo "ERROR: stale $CURRENT reference(s) found above after bumping to v$NEW - fix the sed pattern that missed them" >&2
	exit 1
fi

# Always keep BINARY_VERSION tracking the binary version, regardless of
# whether a plugin version source was found below.
echo "$NEW" >plugins/aphrodite/BINARY_VERSION

# ── Plugin version bump (submodule files - may not all exist) ──
# Plugin version track is independent of binary version track
PLUGIN_CURRENT=""
# Prefer plugin.yaml as canonical source
if [[ -f plugins/aphrodite/plugin.yaml ]]; then
	PLUGIN_CURRENT=$(grep '^version:' plugins/aphrodite/plugin.yaml | head -1 | awk '{print $2}' | tr -d '"')
elif [[ -f plugins/aphrodite/pyproject.toml ]]; then
	PLUGIN_CURRENT=$(grep '^version' plugins/aphrodite/pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi
if [[ -n "$PLUGIN_CURRENT" ]]; then
	PLUGIN_NEW=$(echo "$PLUGIN_CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
	# Sync plugin.yaml - version field + install_message
	sed -i '' "s/version: $PLUGIN_CURRENT/version: $PLUGIN_NEW/" plugins/aphrodite/plugin.yaml
	sed -i '' "s/aphrodite v$PLUGIN_CURRENT -/aphrodite v$PLUGIN_NEW -/" plugins/aphrodite/plugin.yaml
	# Sync the README plugin badge to the new plugin version (pattern-based,
	# not $PLUGIN_CURRENT-based - see the F15 note above the binary-track bump).
	sed -i '' -E "s|plugin-v[0-9]+\.[0-9]+\.[0-9]+-purple|plugin-v$PLUGIN_NEW-purple|" README.md
	# Optional submodule files - skip silently if absent
	[[ -f plugins/aphrodite/pyproject.toml ]] && sed -i '' "s/version = \"$PLUGIN_CURRENT\"/version = \"$PLUGIN_NEW\"/" plugins/aphrodite/pyproject.toml
	[[ -f plugins/aphrodite/__init__.py ]] && sed -i '' "s/aphrodite v$PLUGIN_CURRENT -/aphrodite v$PLUGIN_NEW -/" plugins/aphrodite/__init__.py
	echo "[bump] plugin $PLUGIN_CURRENT → $PLUGIN_NEW"
	if grep -rn "plugin-v$PLUGIN_CURRENT-purple" README.md 2>/dev/null; then
		echo "ERROR: stale plugin-v$PLUGIN_CURRENT-purple badge found after bumping to v$PLUGIN_NEW" >&2
		exit 1
	fi

	# ── Commit + tag + push submodule ──
	# The plugin lives in a git submodule (Aphrodite-Hermes repo).
	# Version bumps above are in the submodule's working tree - they must
	# be committed, tagged, and pushed in the submodule repo so the release
	# is reproducible from a fresh clone. BINARY_VERSION was already written
	# unconditionally above so download.sh always finds the correct binary
	# version, whether or not a plugin version source exists.
	SUBMODULE_REMOTE="${SUBMODULE_REMOTE:-Source}"
	SUBMODULE_BRANCH="${SUBMODULE_BRANCH:-Current}"
	(
		cd plugins/aphrodite
		git add plugin.yaml BINARY_VERSION
		[[ -f pyproject.toml ]] && git add pyproject.toml 2>/dev/null || true
		[[ -f __init__.py ]] && git add __init__.py 2>/dev/null || true
		git commit -m "release: plugin v$PLUGIN_NEW" || echo "[submodule] nothing to commit"
		git tag "v$PLUGIN_NEW" 2>/dev/null || echo "[submodule] tag v$PLUGIN_NEW already exists"
	)
	git -C plugins/aphrodite push "$SUBMODULE_REMOTE" "$SUBMODULE_BRANCH" 2>&1 | tail -1 || true
	[ "${PIPESTATUS[0]}" -eq 0 ] || FAILURES+=("submodule push branch $SUBMODULE_BRANCH")
	git -C plugins/aphrodite push "$SUBMODULE_REMOTE" "v$PLUGIN_NEW" 2>&1 | tail -1 || true
	[ "${PIPESTATUS[0]}" -eq 0 ] || FAILURES+=("submodule push tag v$PLUGIN_NEW")
	echo "[submodule] plugin v$PLUGIN_NEW committed + tagged + pushed"
else
	echo "[bump] plugin skipped - no version source found"
fi
echo "[bump] bin $CURRENT → $NEW"

# Build
cargo build --release -p aphrodite 2>&1 | tail -1
echo "[build] OK"

# Run tests
cargo test -p aphrodite 2>&1 | tail -1
echo "[test] OK"

# Commit version bump + tag (no editor prompts)
git add -u
git commit -m "release(aphrodite): v$NEW - $MSG"
git tag -d "Aphrodite/v$NEW" 2>/dev/null || true
GIT_EDITOR=true git tag -a "Aphrodite/v$NEW" -m "v$NEW" 2>/dev/null || git tag "Aphrodite/v$NEW"
echo "[release] Aphrodite/v$NEW tagged"

# Push - always sync with remote
git push "$REMOTE" Current 2>&1 | tail -1 || true
[ "${PIPESTATUS[0]}" -eq 0 ] || FAILURES+=("push Current")
git push "$REMOTE" "Aphrodite/v$NEW" 2>&1 | tail -1 || true
[ "${PIPESTATUS[0]}" -eq 0 ] || FAILURES+=("push tag Aphrodite/v$NEW")
echo "[push] done"

# Sync submodule pointer - plugin v$PLUGIN_NEW is now committed + tagged in submodule
SUBMODULE_SHA=$(cd plugins/aphrodite && git rev-parse HEAD)
git update-index --cacheinfo 160000,"$SUBMODULE_SHA",plugins/aphrodite 2>/dev/null
git commit -m "chore: sync aphrodite submodule → plugin v$PLUGIN_NEW" 2>/dev/null || echo "[sync] submodule pointer already current"
git push "$REMOTE" Current 2>&1 | tail -1 || true
[ "${PIPESTATUS[0]}" -eq 0 ] || FAILURES+=("push submodule sync")
echo "[sync] submodules done"

echo ""
echo "=== v$NEW released ==="
echo "  $(git log --oneline -3 | paste -sd '|' -)"

if [ "${#FAILURES[@]}" -gt 0 ]; then
	echo ""
	echo "=== release completed with failures ==="
	for f in "${FAILURES[@]}"; do
		echo "  FAILED: $f"
	done
	exit 1
fi
