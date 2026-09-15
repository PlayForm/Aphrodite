#!/usr/bin/env bash
# lib/bump-submodule-gitlink.sh - shared by post-commit / post-checkout.
#
# Bump the parent repo's recorded gitlink to this submodule's current HEAD,
# so the parent always references the latest submodule commit and
# `git submodule status` / `git status` never show a stale pointer ("+" /
# "M <submodule>"). No-op in a plain repo (no superproject), when the
# pointer is already current, or while the parent is mid-merge/cherry-pick.
set -euo pipefail

SUPER="$(git rev-parse --show-superproject-working-tree 2>/dev/null || true)"
if [[ -z "$SUPER" ]]; then
	exit 0
fi

SUB_TOP="$(git rev-parse --show-toplevel)"
NEW_HASH="$(git rev-parse HEAD)"

# Relative submodule path inside the parent (BSD-safe: python3, no GNU realpath).
SUB_REL="$(python3 -c "import os,sys;print(os.path.relpath(sys.argv[1],sys.argv[2]))" "$SUB_TOP" "$SUPER")"

# Skip while the parent is mid-operation - its own commit will record us.
GITDIR="$(git -C "$SUPER" rev-parse --absolute-git-dir 2>/dev/null || true)"
if [[ -n "$GITDIR" ]] && { [[ -f "$GITDIR/MERGE_HEAD" ]] || [[ -f "$GITDIR/CHERRY_PICK_HEAD" ]]; }; then
	exit 0
fi

# Only commit when the recorded gitlink differs from the new submodule HEAD.
RECORDED="$(git -C "$SUPER" ls-files -s -- "$SUB_REL" 2>/dev/null | awk '{print $2}')"
if [[ "$RECORDED" == "$NEW_HASH" ]]; then
	exit 0
fi

git -C "$SUPER" add -- "$SUB_REL"
SHORT="$(git rev-parse --short HEAD)"
git -C "$SUPER" commit -m "chore: bump $SUB_REL submodule to $SHORT" >/dev/null 2>&1 || true