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

# Sync submodules to their remote tracking branches (non-fatal - may fail offline)
git submodule update --remote --recursive --merge || echo "[submodule] sync skipped (offline or no remote)"
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
sed -i '' "s/release-v$CURRENT-blue/release-v$NEW-blue/" README.md
sed -i '' "s/\"version\":\"v$CURRENT\"/\"version\":\"v$NEW\"/" README.md
sed -i '' "3s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$REPO_ROOT/package.json"
echo "[bump] README release badge + package.json → $NEW"

# Always keep BINARY_VERSION tracking the binary version, regardless of
# whether a plugin version source was found below.
echo "$NEW" > plugins/aphrodite/BINARY_VERSION

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
	# Sync the README plugin badge to the new plugin version
	sed -i '' "s|plugin-v$PLUGIN_CURRENT-purple|plugin-v$PLUGIN_NEW-purple|" README.md
	# Optional submodule files - skip silently if absent
	[[ -f plugins/aphrodite/pyproject.toml ]] && sed -i '' "s/version = \"$PLUGIN_CURRENT\"/version = \"$PLUGIN_NEW\"/" plugins/aphrodite/pyproject.toml
	[[ -f plugins/aphrodite/__init__.py ]] && sed -i '' "s/aphrodite v$PLUGIN_CURRENT -/aphrodite v$PLUGIN_NEW -/" plugins/aphrodite/__init__.py
	echo "[bump] plugin $PLUGIN_CURRENT → $PLUGIN_NEW"

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
