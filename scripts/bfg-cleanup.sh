#!/usr/bin/env bash
set -euo pipefail
# BFG Cleanup Script — removes all dev artifacts, secrets, and ignored files
# from ALL git history. Run ONCE before making repo fully public.

echo "=== Aphrodite BFG History Cleanup ==="
echo

# Check BFG is installed
if ! command -v bfg &>/dev/null; then
  echo "Installing BFG..."
  brew install bfg || { echo "Install Java + BFG manually: https://rtyley.github.io/bfg-repo-cleaner/"; exit 1; }
fi

REPO="PlayForm/Aphrodite"
MIRROR="/tmp/aphrodite-clean.git"

# 1. Fresh mirror clone
echo "1/6 Cloning mirror..."
rm -rf "$MIRROR"
git clone --mirror "git@github.com:$REPO.git" "$MIRROR"
cd "$MIRROR"

# 2. Strip ALL ignored files and folders
echo "2/6 Deleting ignored artifacts..."

# Folders to purge from ALL history
bfg --delete-folders \
  .hermes \
  __pycache__ \
  .idea \
  .vscode \
  target \
  profiles \
  .git-rewrite \
  --no-blob-protection .

# Files to purge from ALL history
bfg --delete-files \
  ".DS_Store" \
  "Thumbs.db" \
  "*.pyc" \
  "*.pyo" \
  "*.log" \
  "*.lock" \
  "*.swp" \
  "*.swo" \
  "*~" \
  ".env" \
  ".env.sh" \
  "aphrodite.local.toml" \
  "*.pem" \
  "*.key" \
  "id_rsa" \
  "id_rsa.pub" \
  "*.tok" \
  "*.token" \
  --no-blob-protection .

# 3. Redact secrets from all blobs
echo "3/6 Redacting secrets..."
bfg --replace-text <(cat <<'SECRETS'
APHRODITE_API_KEY==>SK-REDACTED
HEADROOM_DEEPSEEK_KEY==>SK-REDACTED
DEEPSEEK_API_KEY==>SK-REDACTED
api_key==>SK-REDACTED
notify_key==>SK-REDACTED
SECRETS
) --no-blob-protection .

# 4. Strip files larger than 10MB (binary releases)
echo "4/6 Stripping large blobs..."
bfg --strip-blobs-bigger-than 10M --no-blob-protection .

# 5. Expire and GC
echo "5/6 Expiring reflog + aggressive GC..."
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# 6. Force push
echo "6/6 Force pushing..."
echo
echo "READY to push? This DESTROYS remote history and replaces it."
echo "Remote: $REPO"
echo "Press Ctrl+C to abort, ENTER to force push."
read -r
git push --force --all
git push --force --tags

echo
echo "=== Done. History is clean. ==="
echo "Now commit these files and push normally:"
echo "  .gitignore .gitattributes .dockerignore .npmignore .editorconfig rustfmt.toml ruff.toml"
