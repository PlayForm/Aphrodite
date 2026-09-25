# Recursive Multi-Repo Scanning Pattern

## Problem

The Aphrodite monorepo uses git submodules (each its own repo): a single-repo
watcher only sees changes in the top-level repo. Changes inside submodules go
uncommitted because each submodule is a separate working tree with its own
`.git` file. The submodule fleet: `plugins/aphrodite` (git remote `Source`),
`vendor/headroom`, `vendor/rtk`.

## Solution

Extend the watcher to discover all repos under a root, skip excluded directories,
then iterate each repo independently for both staging and committing.

### Discovery Function

```bash
find_repos() {
	echo "$WATCH_DIR"

	# plugins/aphrodite submodule (has .git file)
	if [ -f "$WATCH_DIR/plugins/aphrodite/.git" ] || [ -d "$WATCH_DIR/plugins/aphrodite/.git" ]; then
		echo "$WATCH_DIR/plugins/aphrodite"
	fi

	# Vendored submodules under vendor/ (headroom, rtk)
	for DIR in "$WATCH_DIR"/vendor/*/; do
		BASENAME=$(basename "$DIR")
		case "$BASENAME" in
			headroom | rtk) ;; # real submodules - include
			*) continue ;;     # anything else - skip
		esac
		# Submodule .git files are 1-line gitdir pointers, not directories
		if [ -f "${DIR}.git" ] || [ -d "${DIR}.git" ]; then
			echo "${DIR%/}"
		fi
	done
}
```

**Key detail:** Submodules store `.git` as a _file_ containing `gitdir: ...`,
not as a directory. Guard with both `-f` and `-d` to handle bare repos too.
Note that the parent's `.githooks` auto-bump submodule gitlinks on
post-commit - the watcher's parent pass must tolerate that (commit with
`|| true`).

### Staging Loop

```bash
while IFS= read -r REPO; do
	cd "$REPO" 2> /dev/null || continue
	CHANGES=$(git status --porcelain 2> /dev/null)
	if [ -n "$CHANGES" ]; then
		git add -A 2> /dev/null
		STAGED_COUNT=$(git diff --cached --name-only 2> /dev/null | wc -l | tr -d ' ')
		STAGED_COUNT=${STAGED_COUNT:-0}
		if [ "$STAGED_COUNT" -gt 0 ]; then
			STAGED_ANY=true
		fi
	fi
done < <(find_repos)
```

### Per-Repo Commit Routing

Some projects use different commit aliases per repo tier. Route based on path:

```bash
determine_commit_cmd() {
	local REPO_PATH="$1"
	if [ "$REPO_PATH" = "$WATCH_DIR" ] || [ "$REPO_PATH" = "$WATCH_DIR/plugins/aphrodite" ]; then
		echo "git gcommit|git gcommit-hermes"
	else
		# Vendored deps (vendor/headroom, vendor/rtk): plain commit, no
		# project message stack - keep third-party trees neutral
		echo "git commit"
	fi
}
```

Then `eval "$COMMIT_CMD"` and `eval "$FALLBACK_CMD"` to run the right tool.

### Submodule Pointer Update

Committing inside each submodule does **not** auto-update the parent's gitlink.
The parent (the Aphrodite root) will show the submodule as "modified (new
commits)". The watcher should also commit the parent if submodule pointers
changed:

```bash
# After committing submodules, check if the root has updated gitlinks
cd "$WATCH_DIR" 2> /dev/null || continue
if git diff --name-only | grep -q '^'; then
	git add -A 2> /dev/null
	# commit if staged
fi
```

The parent's `.githooks/post-commit` bumps gitlinks for real mode-160000
entries (`git ls-tree` records the parent's pointer); if the auto-committer
holds the index, the bump converges on the next git action.
