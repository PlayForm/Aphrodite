#!/bin/bash
# release.sh — auto-bump, build, tag, and release aphrodite
# Usage: ./scripts/release.sh [patch|minor|major] [--push]
# Default: patch bump, no push

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$REPO_ROOT/crates/aphrodite/Cargo.toml"

BUMP="${1:-patch}"
PUSH="${2:-}"

# Read current version
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
MAJOR=$(echo "$CURRENT" | cut -d. -f1)
MINOR=$(echo "$CURRENT" | cut -d. -f2)
PATCH=$(echo "$CURRENT" | cut -d. -f3)

case "$BUMP" in
	major) NEW="$((MAJOR + 1)).0.0" ;;
	minor) NEW="$MAJOR.$((MINOR + 1)).0" ;;
	patch) NEW="$MAJOR.$MINOR.$((PATCH + 1))" ;;
	*) echo "Invalid bump: $BUMP (use patch|minor|major)" && exit 1 ;;
esac

echo "=== aphrodite release: $CURRENT → $NEW ==="
echo ""

# Bump version
sed -i '' "s/version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO_TOML"
echo "[1/5] Version bumped: $CURRENT → $NEW"

# Check compilation first (fast)
cd "$REPO_ROOT"
cargo check --release -p aphrodite 2>&1 | tail -1
echo "[2/5] Check passed"

# Build release binary
cargo build --release -p aphrodite 2>&1 | tail -1
echo "[3/5] Release build: $(ls -lh target/release/aphrodite | awk '{print $5}')"

# Run tests
cargo test -p aphrodite 2>&1 | tail -1
echo "[4/5] Tests passed"

# Commit + tag
git add -u
git commit -m "release(aphrodite): v$NEW"
git tag "v$NEW"
echo "[5/5] Committed + tagged v$NEW"

echo ""
echo "=== Release v$NEW complete ==="
echo "  Binary: target/release/aphrodite"
echo "  Commit: $(git rev-parse --short HEAD)"

if [ "$PUSH" = "--push" ]; then
	echo ""
	echo "Pushing..."
	git push origin Current
	git push origin "v$NEW"
	echo "Pushed."
else
	echo "  Push:   ./scripts/release.sh $BUMP --push"
fi
