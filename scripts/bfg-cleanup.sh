#!/usr/bin/env bash
set -euo pipefail

echo "=== Aphrodite BFG History Cleanup ==="
echo

# Check BFG
if ! command -v bfg &>/dev/null; then
  echo "Installing BFG..."
  brew install bfg || {
    echo "ERROR: Install Java 11+ and BFG manually: https://rtyley.github.io/bfg-repo-cleaner/"
    exit 1
  }
fi

REPO="git@github.com:PlayForm/Aphrodite.git"
WORK_DIR="/tmp/aphrodite-bfg-clean"
MIRROR="$WORK_DIR/mirror.git"

mkdir -p "$WORK_DIR"
cd "$WORK_DIR"

# 1. Clone fresh mirror
echo "1/5 Cloning mirror..."
rm -rf "$MIRROR"
git clone --mirror "$REPO" "$MIRROR"
cd "$MIRROR"

# 2. Delete all private folders
echo "2/5 Deleting private folders from history..."
bfg --delete-folders \
  .hermes \
  .headroom \
  __pycache__ \
  .idea \
  .vscode \
  target \
  profiles \
  --no-blob-protection .

# 3. Delete all private files
echo "3/5 Deleting private files from history..."
bfg --delete-files \
  ".DS_Store" \
  "*.pyc" \
  "*.log" \
  "*.lock" \
  ".env" \
  ".env.sh" \
  "aphrodite.local.toml" \
  auth.json \
  state.db \
  --no-blob-protection .

# 4. Strip large blobs (>1MB)
echo "4/5 Stripping large blobs..."
bfg --strip-blobs-bigger-than 1M --no-blob-protection .

# 5. GC + force push
echo
echo "5/5 Cleaning and pushing..."
git reflog expire --expire=now --all
git gc --prune=now --aggressive

echo
echo "=== Ready to force push ==="
echo "  Remote: $REPO"
echo "  Run:    cd $MIRROR && git push --force --all && git push --force --tags"
echo "  Then:   cd your-repo && git fetch origin && git reset --hard origin/Current"
echo
echo "After push, commit the cleaned repo with:"
echo "  git add -A && git commit -m 'chore: clean repo after BFG history sanitization'"
