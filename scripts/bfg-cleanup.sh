#!/usr/bin/env bash
set -euo pipefail

echo "=== Aphrodite BFG History Cleanup ==="

if ! command -v bfg &> /dev/null; then
	echo "Installing BFG..."
	brew install bfg || {
		echo "ERROR: Install Java 11+ and BFG"
		exit 1
	}
fi

REPO="git@github.com:PlayForm/Aphrodite.git"
MIRROR="/tmp/aphrodite-bfg-clean/mirror.git"
rm -rf "$MIRROR"

# 1. Clone
echo "1/4 Cloning mirror..."
git clone --mirror "$REPO" "$MIRROR"
cd "$MIRROR"

# 2. Delete folders (BFG takes ONE glob per flag)
echo "2/4 Deleting private folders..."
bfg --delete-folders "{.hermes,.headroom,__pycache__,.idea,.vscode,target,profiles}" --no-blob-protection .

# 3. Delete files (BFG takes ONE glob per flag)
echo "3/4 Deleting private files..."
bfg --delete-files "*.{pyc,pyo,log,lock,swp,swo,env,env.sh,pem,key,tar.gz}" --no-blob-protection .
bfg --delete-files "{.DS_Store,auth.json,state.db,aphrodite.local.toml}" --no-blob-protection .

# 4. Strip large blobs (>1MB)
echo "4/4 Stripping large blobs..."
bfg --strip-blobs-bigger-than 1M --no-blob-protection .

# Done
git reflog expire --expire=now --all
git gc --prune=now --aggressive

echo
echo "=== DONE ==="
echo "Push:  cd $MIRROR && git push --force --all && git push --force --tags"
echo "Sync:  cd /Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite && git fetch origin && git reset --hard origin/Current"
