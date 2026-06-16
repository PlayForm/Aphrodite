#!/usr/bin/env bash
# auto-release.sh - Single-command aphrodite release pipeline.
#
# Usage: bash scripts/auto-release.sh [patch|minor|major]
#
# Steps:
#   1. Determine next version from Cargo.toml and semver bump kind
#   2. cargo build --release -p aphrodite
#   3. Bump version in crates/aphrodite/Cargo.toml
#   4. Bump BIN_VERSION and PLUGIN_VERSION in plugins/aphrodite/_core.py
#   5. git add → git commit -m "release(aphrodite): v<VERSION>"
#   6. git push origin HEAD:Current
#   7. git tag v<VERSION> && git push origin v<VERSION>
#   8. gh release create v<VERSION> target/release/aphrodite --title "aphrodite v<VERSION>"
#   9. Sync ~/.hermes/aphrodite/ binary
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# ── helpers ──────────────────────────────────────────────────────────────
die() { echo "ERROR: $*" >&2; exit 1; }
info() { printf "\033[36m▸ %s\033[0m\n" "$*"; }

# ── current version ──────────────────────────────────────────────────────
CRATE_TOML="crates/aphrodite/Cargo.toml"
CORE_PY="plugins/aphrodite/_core.py"

current_version=$(grep '^version = ' "$CRATE_TOML" | sed 's/.*"\(.*\)".*/\1/')
[ -n "$current_version" ] || die "could not read version from $CRATE_TOML"
info "current version: v$current_version"

# ── bump type ────────────────────────────────────────────────────────────
BUMP="${1:-patch}"
IFS='.' read -r major minor patch <<< "$current_version"
case "$BUMP" in
  patch) patch=$((patch + 1)) ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  major) major=$((major + 1)); minor=0; patch=0 ;;
  *)     die "bump must be: patch | minor | major" ;;
esac
new_ver="${major}.${minor}.${patch}"
info "new version: v$new_ver"

# ── 1. Build ─────────────────────────────────────────────────────────────
info "building aphrodite v$new_ver..."
cargo build --release -p aphrodite

# ── 2. Bump crate version ────────────────────────────────────────────────
info "bumping crate version → $new_ver"
sed -i '' "s/^version = \"$current_version\"/version = \"$new_ver\"/" "$CRATE_TOML"

# ── 3. Bump plugin constants ──────────────────────────────────────────────
info "bumping plugin BIN_VERSION + PLUGIN_VERSION"
# Parse current plugin version (last segment)
current_py=$(grep "^PLUGIN_VERSION" "$CORE_PY" | sed 's/.*"\(.*\)".*/\1/')
IFS='.' read -r py_major py_minor py_patch <<< "$current_py"
case "$BUMP" in
  patch) py_patch=$((py_patch + 1)) ;;
  minor) py_minor=$((py_minor + 1)); py_patch=0 ;;
  major) py_major=$((py_major + 1)); py_minor=0; py_patch=0 ;;
esac
new_py="${py_major}.${py_minor}.${py_patch}"

sed -i '' "s/^BIN_VERSION = \".*\"/BIN_VERSION = \"v$new_ver\"/" "$CORE_PY"
sed -i '' "s/^PLUGIN_VERSION = \".*\"/PLUGIN_VERSION = \"$new_py\"/" "$CORE_PY"
info "  BIN_VERSION → v$new_ver | PLUGIN_VERSION → $new_py"

# ── 4. Commit ────────────────────────────────────────────────────────────
info "committing..."
COMMIT_MSG="release(aphrodite): v$new_ver"
git add "$CRATE_TOML" "$CORE_PY" target/release/aphrodite
git commit -m "$COMMIT_MSG"

# ── 5. Push ──────────────────────────────────────────────────────────────
info "pushing to Current..."
git push aphrodite HEAD:Current

# ── 6. Tag + push tag ────────────────────────────────────────────────────
info "tagging v$new_ver..."
git tag "v$new_ver"
git push aphrodite "v$new_ver"

# ── 7. GitHub release ────────────────────────────────────────────────────
info "creating GitHub release..."
gh release create "v$new_ver" target/release/aphrodite \
  --repo PlayForm/Aphrodite \
  --title "aphrodite v$new_ver" \
  --notes "$(git log --oneline -10 "$(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD)..HEAD")"

# ── 8. Sync binary to ~/.hermes/aphrodite/ ──────────────────────────────
info "syncing binary to ~/.hermes/aphrodite/..."
mkdir -p ~/.hermes/aphrodite
cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
chmod +x ~/.hermes/aphrodite/aphrodite

# ── Done ─────────────────────────────────────────────────────────────────
info "✓ release v$new_ver complete"
echo ""
echo "  Crate:  crates/aphrodite → v$new_ver"
echo "  Plugin: PLUGIN_VERSION → $new_py"
echo "  Binary: $new_ver → ~/.hermes/aphrodite/aphrodite"
echo "  Tag:    v$new_ver pushed"
echo "  GH:     https://github.com/PlayForm/Aphrodite/releases/tag/v$new_ver"
