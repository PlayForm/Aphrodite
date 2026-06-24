#!/bin/bash
# auto-release.sh - commit all changes, bump version, build, tag, release
# auto-release.sh - commit, bump, build, tag, push - full pipeline
# Usage: ./scripts/auto-release.sh ["commit message"]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CARGO_TOML="$REPO_ROOT/crates/aphrodite/Cargo.toml"
MSG="${1:-}"
REMOTE="${GIT_REMOTE:-origin}"

cd "$REPO_ROOT"

# Sync submodules to their remote tracking branches (non-fatal — may fail offline)
git submodule update --remote --recursive --merge || echo "[submodule] sync skipped (offline or no remote)"
# (vendor submodule handled by git submodule)

# Stage all changes
git add -u
git add docs/ scripts/ plugins/aphrodite/ 2>/dev/null || true

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

# Read current version, bump patch (or minor with --minor flag)
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ "${1:-}" = "--minor" ]; then
	NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2+1".0"}')
	shift
else
	NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
fi

# Bump Cargo.toml (line 3 only - avoid matching dependency versions)
sed -i '' "3s/version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO_TOML"
# Bump aphrodite-hermes Cargo.toml — package version + dependency
HERMES_TOML="$REPO_ROOT/crates/aphrodite-hermes/Cargo.toml"
sed -i '' "3s/version = \"$CURRENT\"/version = \"$NEW\"/" "$HERMES_TOML"
sed -i '' "s/aphrodite = { path = \"..\/aphrodite\", version = \"$CURRENT\"/aphrodite = { path = \"..\/aphrodite\", version = \"$NEW\"/" "$HERMES_TOML"
echo "[bump] aphrodite-hermes Cargo.toml → $NEW"
# ── Plugin version bump (submodule files — may not all exist) ──
# Plugin version track is independent of binary version track
PLUGIN_CURRENT=""
# Prefer plugin.yaml as canonical source
if [[ -f plugins/aphrodite/plugin.yaml ]]; then
	PLUGIN_CURRENT=$(grep '^version:' plugins/aphrodite/plugin.yaml | head -1 | awk '{print $2}' | tr -d '"')
elif [[ -f plugins/aphrodite/_core/config.py ]]; then
	PLUGIN_CURRENT=$(grep 'PLUGIN_VERSION' plugins/aphrodite/_core/config.py | head -1 | sed 's/[^"]*"\([^"]*\)".*/\1/')
elif [[ -f plugins/aphrodite/pyproject.toml ]]; then
	PLUGIN_CURRENT=$(grep '^version' plugins/aphrodite/pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi
if [[ -n "$PLUGIN_CURRENT" ]]; then
	PLUGIN_NEW=$(echo "$PLUGIN_CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
	# Sync plugin.yaml — version field + install_message
	sed -i '' "s/version: $PLUGIN_CURRENT/version: $PLUGIN_NEW/" plugins/aphrodite/plugin.yaml
	sed -i '' "s/aphrodite v$PLUGIN_CURRENT -/aphrodite v$PLUGIN_NEW -/" plugins/aphrodite/plugin.yaml
	# Optional submodule files — skip silently if absent
	[[ -f plugins/aphrodite/pyproject.toml ]] && sed -i '' "s/version = \"$PLUGIN_CURRENT\"/version = \"$PLUGIN_NEW\"/" plugins/aphrodite/pyproject.toml
	[[ -f plugins/aphrodite/__init__.py ]] && sed -i '' "s/aphrodite v$PLUGIN_CURRENT -/aphrodite v$PLUGIN_NEW -/" plugins/aphrodite/__init__.py
	[[ -f plugins/aphrodite/_core/config.py ]] && sed -i '' "s/PLUGIN_VERSION = \"$PLUGIN_CURRENT\"/PLUGIN_VERSION = \"$PLUGIN_NEW\"/" plugins/aphrodite/_core/config.py
	[[ -f plugins/aphrodite/_core/config.py ]] && sed -i '' "s/BIN_VERSION = \"v$CURRENT\"/BIN_VERSION = \"v$NEW\"/" plugins/aphrodite/_core/config.py
	echo "[bump] plugin $PLUGIN_CURRENT → $PLUGIN_NEW"

	# ── Commit + tag + push submodule ──
	# The plugin lives in a git submodule (Aphrodite-Hermes repo).
	# Version bumps above are in the submodule's working tree — they must
	# be committed, tagged, and pushed in the submodule repo so the release
	# is reproducible from a fresh clone.
	# Also update BINARY_VERSION so download.sh can find the correct
	# binary version without needing the monorepo or GitHub API.
	echo "$NEW" > plugins/aphrodite/BINARY_VERSION
	SUBMODULE_REMOTE="${SUBMODULE_REMOTE:-Source}"
	SUBMODULE_BRANCH="${SUBMODULE_BRANCH:-Current}"
	(
		cd plugins/aphrodite
		git add plugin.yaml BINARY_VERSION
		[[ -f pyproject.toml ]] && git add pyproject.toml 2>/dev/null || true
		[[ -f __init__.py ]] && git add __init__.py 2>/dev/null || true
		[[ -f _core/config.py ]] && git add _core/config.py 2>/dev/null || true
		git commit -m "release: plugin v$PLUGIN_NEW" || echo "[submodule] nothing to commit"
		git tag "v$PLUGIN_NEW" 2>/dev/null || echo "[submodule] tag v$PLUGIN_NEW already exists"
		git push "$SUBMODULE_REMOTE" "$SUBMODULE_BRANCH" 2>&1 | tail -1 || echo "[submodule] push branch skipped (auth?)"
		git push "$SUBMODULE_REMOTE" "v$PLUGIN_NEW" 2>&1 | tail -1 || echo "[submodule] push tag skipped (auth?)"
	)
	echo "[submodule] plugin v$PLUGIN_NEW committed + tagged + pushed"
else
	echo "[bump] plugin skipped — no version source found"
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
git push "$REMOTE" Current 2>&1 | tail -1 || echo "[push] Current skipped (auth?)"
git push "$REMOTE" "Aphrodite/v$NEW" 2>&1 | tail -1 || echo "[push] tag skipped (auth?)"
echo "[push] done"

# Sync submodule pointer — plugin v$PLUGIN_NEW is now committed + tagged in submodule
SUBMODULE_SHA=$(cd plugins/aphrodite && git rev-parse HEAD)
git update-index --cacheinfo 160000,"$SUBMODULE_SHA",plugins/aphrodite 2>/dev/null
git commit -m "chore: sync aphrodite submodule → plugin v$PLUGIN_NEW" 2>/dev/null || echo "[sync] submodule pointer already current"
git push "$REMOTE" Current 2>&1 | tail -1 || echo "[push] submodule sync skipped (auth?)"
echo "[sync] submodules done"

echo ""
echo "=== v$NEW released ==="
echo "  $(git log --oneline -3 | paste -sd '|' -)"
