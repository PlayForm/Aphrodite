# BFG Cleanup Pattern

Remove a sensitive/tracked file from entire git history.

```bash
# 1. Create mirror clone
git clone --mirror /path/to/repo /tmp/repo.mirror

# 2. Run BFG (requires: brew install bfg)
cd /tmp/repo.mirror
bfg --delete-files FILENAME --no-blob-protection

# 3. Clean up
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# 4. Push to remote (add remote if needed)
git remote add github git@github.com:Org/Repo.git
git push github --force --all

# 5. Update local working copy
cd /path/to/working/repo
git fetch <remote>  # check remote name with: git remote -v
git reset --hard <remote>/Current
```

## Pitfalls

- BFG only processes the default branch by default. Use `--all` flag for all branches.
- Local working copy becomes out of sync after force push — must hard-reset.
- The `--force` push is destructive to all clones. Coordinate with collaborators.
- Mirror clone has no working tree — operations are on the bare repo.
- After cleanup, the file should be in `.gitignore` to prevent re-tracking.
- `git ls-files FILENAME` should return empty after successful cleanup.
