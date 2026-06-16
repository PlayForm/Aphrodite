#!/bin/bash
# auto-release.sh — commit all changes, bump version, build, tag, release
# auto-release.sh — commit, bump, build, tag, push — full pipeline
# Usage: ./scripts/auto-release.sh ["commit message"]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$REPO_ROOT/crates/aphrodite/Cargo.toml"
MSG="${1:-}"

cd "$REPO_ROOT"

# Sync submodules to their remote tracking branches
git submodule update --remote --recursive --merge
git add --force vendor/headroom 2>/dev/null || true

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

# Read current version, bump patch
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2"."$3+1}')

# Bump
sed -i '' "s/version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO_TOML"
echo "[bump] $CURRENT → $NEW"

# Build
cargo build --release -p aphrodite 2>&1 | tail -1
echo "[build] OK"

# Run tests
cargo test -p aphrodite 2>&1 | tail -1
echo "[test] OK"

# Commit version bump + tag (no editor prompts)
git add -u
git commit -m "release(aphrodite): v$NEW — $MSG"
git tag -d "v$NEW" 2>/dev/null || true
GIT_EDITOR=true git tag -a "v$NEW" -m "v$NEW" 2>/dev/null || git tag "v$NEW"
echo "[release] v$NEW tagged"

# Push — always sync with remote
git push origin Current 2>&1 | tail -1 || echo "[push] Current skipped (auth?)"
git push origin "v$NEW" 2>&1 | tail -1 || echo "[push] tag skipped (auth?)"
echo "[push] done"

echo ""
echo "=== v$NEW released ==="
echo "  $(git log --oneline -3 | paste -sd '|' -)"
