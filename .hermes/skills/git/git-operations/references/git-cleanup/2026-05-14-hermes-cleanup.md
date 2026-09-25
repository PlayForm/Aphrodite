# Hermes Agent - Repository Cleanup Reference (2026-05-14)

**Session:** `git-cleanup` - reset `~/.hermes` from 2.3 GB to lightweight Git repo
**Remote:** `<owner>/Hermes` (branch: `Current`)
**Commit after clean:** `c03b094` (168 files, sessions in regular Git)
**Backup:** `~/.hermes/.git.backup/`

---

## Cleanup Summary

| Item               | Before                    | After           | Action                          |
| ------------------ | ------------------------- | --------------- | ------------------------------- |
| `state.db`         | ~1.5 GB                   | excluded        | `state.db` in `.gitignore`      |
| `state-snapshots/` | ~100+ MB each             | excluded        | `state-snapshots/` pattern      |
| `lsp/`             | 99 MB                     | excluded        | `lsp/` in `.gitignore`          |
| `hermes-agent/`    | 1.5 GB                    | excluded        | `hermes-agent/` in `.gitignore` |
| `logs/`            | ~5 MB                     | excluded        | `logs/` + `*.log`               |
| `sessions/`        | 75 MB total               | **kept in Git** | regular tracking (no LFS)       |
| LFS usage          | attempted, quota exceeded | removed         | `.gitattributes` deleted        |

---

## Final `.gitignore`

```gitignore
# Backup of previous git repo (local, not for commit)
.git.backup/

# Hermes runtime state - large, transient
state.db
state.db-wal
state.db-shm
state-snapshots/

# Logs
*.log
logs/

# Language server caches (regenerable)
lsp/

# Hermes agent installation (reinstallable via package manager)
hermes-agent/

# Binaries (reinstallable)
bin/

# Transient pastes
pastes/

# General cache
cache/

# Audio/image caches
audio_cache/
image_cache/

# Transient/empty directories
pairing/
hooks/
cron/
sandboxes/
spawn-trees/

# Nested git repositories (prevent accidental inclusion)
**/.git

# SQLite shared memory files
*.db-shm

# SQLite rollback journals (transient)
*.db-journal

# OS generated files
.DS_Store

# Lock files
*.lock
```

**Key decision:** Sessions tracked in regular Git (not LFS) - total 75 MB, largest file <2 MB, well under GitHub's 100 MB per-file limit. Avoids LFS quota management entirely.

---

## Verification Commands Used

```bash
# 1. After adding .gitignore, check what will be untracked
git ls-files --others --exclude-standard
# Expected: only .skills_prompt_snapshot.json and new session files created after commit

# 2. Find large tracked files (should only be sessions and small configs)
git ls-files | xargs ls -lh 2> /dev/null | awk '$5 ~ /[MKG]/ {print $5, $9}' | sort -rh | head -20
# Expected: sessions/ files up to ~2 MB, models_dev_cache.json ~1.8 MB

# 3. Confirm SQLite ephemeral files exist but are ignored (normal)
find . -name '*.db-wal' -o -name '*.db-shm' 2> /dev/null
# Expected: state.db-wal, state.db-shm present but not tracked

# 4. Check remote URL format (must be git@ not ssh://)
git remote -v
# Expected: git@github.com:<owner>/Hermes.git

# 5. Verify local LFS status (should be none if LFS removed)
git lfs ls-files
# Expected: empty

# 6. Post-push verification - clean clone test
cd /tmp
git clone git@github.com: test-clone < owner > /Hermes.git
cd test-clone
find . -type f -size +50M ! -path './.git/*' # should return nothing
rm -rf /tmp/test-clone
```

---

## LFS Quota Workaround

When `git push` fails with:

```
batch response: This repository exceeded its LFS budget
```

Even if your new commit doesn't use LFS, GitHub blocks pushes to repos over LFS quota. Remedies:

1. **Purchase LFS quota** - GitHub Data Pack
2. **Remove LFS tracking** - edit `.gitattributes`, `git rm --cached`, recommit
3. **Delete & recreate remote** (fastest for personal repos) - loses all metadata (issues, PRs, stars, releases, wiki)

---

## Pitfalls Encountered

| #   | Symptom                                 | Fix Applied                                                                             |
| --- | --------------------------------------- | --------------------------------------------------------------------------------------- |
| 1   | `ssh://github.com/...` URL format       | Changed to `git@github.com:user/repo.git` via `git remote set-url origin`               |
| 2   | LFS quota exceeded despite clean commit | Deleted & recreated remote repository (nuclear)                                         |
| 3   | Sessions accidentally in LFS pointers   | `git lfs untrack "sessions/**"`, removed `.gitattributes`, recommitted as regular files |
| 4   | Large files still showing after untrack | Verified `.gitignore` coverage with `git ls-files --others --exclude-standard`          |

---

## File Size Notes

| Path                           | Size (max observed)      | Tracking      |
| ------------------------------ | ------------------------ | ------------- |
| `sessions/session_*.json`      | 2.0 MB                   | Git (regular) |
| `sessions/request_dump_*.json` | 865 KB                   | Git (regular) |
| `models_dev_cache.json`        | 1.8 MB                   | Git (regular) |
| `state.db`                     | ~1.5 GB (excluded)       | `.gitignore`  |
| `state-snapshots/`             | ~100+ MB each (excluded) | `.gitignore`  |
| `lsp/`                         | 99 MB (excluded)         | `.gitignore`  |
| `hermes-agent/`                | 1.5 GB (excluded)        | `.gitignore`  |

---

## Related Skills

- `git-operations` - umbrella skill for Git repository cleanup operations
- `references/git-cleanup/hermes-state-files.md` - Hermes runtime state file reference
