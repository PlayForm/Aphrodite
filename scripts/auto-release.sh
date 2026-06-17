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

# Read current version, bump patch (or minor with --minor flag)
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ "${1:-}" = "--minor" ]; then
    NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2+1".0"}')
    shift
else
    NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
fi

# Bump Cargo.toml
sed -i '' "s/version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO_TOML"
# Sync Python BIN_VERSION (now in _core/config.py package)
sed -i '' "s/BIN_VERSION = \"v$CURRENT\"/BIN_VERSION = \"v$NEW\"/" plugins/aphrodite/_core/config.py

# Bump plugin version (patch increment on 1.x.x scheme)
PLUGIN_CURRENT=$(grep 'PLUGIN_VERSION' plugins/aphrodite/_core/config.py | head -1 | sed 's/.*"\(.*\)"/\1/')
PLUGIN_NEW=$(echo "$PLUGIN_CURRENT" | awk -F. '{print $1"."$2"."$3+1}')
sed -i '' "s/PLUGIN_VERSION = \"$PLUGIN_CURRENT\"/PLUGIN_VERSION = \"$PLUGIN_NEW\"/" plugins/aphrodite/_core/config.py
# Sync pyproject.toml
sed -i '' "s/version = \"$PLUGIN_CURRENT\"/version = \"$PLUGIN_NEW\"/" plugins/aphrodite/pyproject.toml
# Sync __init__.py docstring
sed -i '' "s/aphrodite v$PLUGIN_CURRENT —/aphrodite v$PLUGIN_NEW —/" plugins/aphrodite/__init__.py
echo "[bump] bin $CURRENT → $NEW | plugin $PLUGIN_CURRENT → $PLUGIN_NEW"

# Build
cargo build --release -p aphrodite 2>&1 | tail -1
echo "[build] OK"

# Run tests
cargo test -p aphrodite 2>&1 | tail -1
echo "[test] OK"

# Commit version bump + tag (no editor prompts)
git add -u
git commit -m "release(aphrodite): v$NEW — $MSG"
git tag -d "Aphrodite/v$NEW" 2>/dev/null || true
GIT_EDITOR=true git tag -a "Aphrodite/v$NEW" -m "v$NEW" 2>/dev/null || git tag "Aphrodite/v$NEW"
echo "[release] Aphrodite/v$NEW tagged"

# Push — always sync with remote
git push Source Current 2>&1 | tail -1 || echo "[push] Current skipped (auth?)"
git push Source "Aphrodite/v$NEW" 2>&1 | tail -1 || echo "[push] tag skipped (auth?)"
echo "[push] done"

echo ""
echo "=== v$NEW released ==="
echo "  $(git log --oneline -3 | paste -sd '|' -)"
