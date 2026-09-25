# Hermes Agent Runtime Files to Exclude

This document lists the specific large and transient files produced by Hermes Agent that should be excluded from Git version control and, if committed, purged from history.

---

## Large State Files (>100MB GitHub limit)

| File/Directory     | Typical Size         | Purpose                                                      | Keep?               |
| ------------------ | -------------------- | ------------------------------------------------------------ | ------------------- |
| `state.db`         | 100-200 MB           | SQLite database storing agent memory, session cache, context | ❌ No - regenerable |
| `state.db-wal`     | ~1 MB                | SQLite write-ahead log ( accompanies `state.db`)             | ❌ No               |
| `state.db-shm`     | ~1 KB                | SQLite shared memory                                         | ❌ No               |
| `state-snapshots/` | 100+ MB per snapshot | Point-in-time backups of `state.db`                          | ❌ No - transient   |

**Why excluded:** These are runtime state, not source. They accumulate quickly and exceed GitHub's 100MB per-file limit.

---

## Other Runtime / Cache Directories

| Path                              | Size             | Purpose                                            | Keep?                                                      |
| --------------------------------- | ---------------- | -------------------------------------------------- | ---------------------------------------------------------- |
| `logs/`                           | Varies           | Agent and subprocess stdout/stderr logs            | ❌ No - reproducible                                       |
| `lsp/`                            | 99 MB (reported) | Language Server Protocol cache (code intelligence) | ❌ No - regenerable                                        |
| `hermes-agent/` (in `~/.hermes/`) | 1.5 GB           | Hermes agent installation/binary                   | ⚠️ Maybe - if vendored; otherwise regenerate via installer |

**Note:** `hermes-agent/` size is large because it includes the entire agent runtime. If this is a custom-built agent, consider whether versioning the binary itself is necessary. For standard installs, it can be re-downloaded.

---

## Session Files

| Path        | Size        | Purpose                                       | Keep?                |
| ----------- | ----------- | --------------------------------------------- | -------------------- |
| `sessions/` | 75 MB total | JSON conversation sessions (history, context) | ✅ Yes - but use LFS |

**Recommendation:** Store sessions in **Git LFS** (`sessions/**`). Total is well under LFS quota for moderate usage. Each individual session JSON is <1MB, far below GitHub's 100MB limit, so LFS is optional but recommended to keep repo size manageable.

---

## Configuration & Metadata (Keep in Git)

These files are small and should be versioned:

- `config.yaml` - Hermes configuration
- `memories/MEMORY.md` & `memories/USER.md` - persistent memory
- `skills/` - skill definitions (code + docs)
- `agent-hooks/` - shell hook scripts
- `audits/` - audit reports
- `plugins/` - plugin code
- `shared/nous_auth.json` - auth token (keep private but backed up)

---

## Exclude Pattern (for `.gitignore`)

```gitignore
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

# Hermes agent installation (if reinstallable)
# Uncomment if you want to exclude the entire agent binary
# hermes-agent/

# Lock files
*.lock

# OS
.DS_Store

# Sessions (optional - only if NOT using LFS)
# sessions/
```

**If using LFS for sessions:**

```bash
git lfs track "sessions/**"
```

Then keep `sessions/` in Git (LFS will handle the blobs).

---

## Cleanup Procedure Reference

See the parent skill [`git-operations`](../SKILL.md) for the full step-by-step procedure to remove these files from history and force-push a clean repository.

Key steps:

1. Backup `.git`
2. Remove `.git`, create fresh repo
3. Add `.gitignore` with patterns above
4. Initialize LFS for `sessions/**` (recommended)
5. Commit and force push

---

## Session-Specific Notes

**Date:** 2026-05-14
**Repository:** `<owner>/Hermes` (private Hermes Agent config repo)
**Files removed:** `state.db` (119 MB), `state-snapshots/` (multiple ~100+ MB snapshots), `logs/`, `lsp/` (99 MB)
**LFS configured:** Yes - `sessions/**` tracked in LFS
**Push status:** Force push stalled due to LFS quota exceeded; remedy: either purchase LFS quota or recommit sessions as regular Git files (75 MB total, well under per-file limits)
**Remote branch:** `Current`
**Commit SHA after clean:** `bcca1ce` (root-commit)
**Backup location:** `~/.hermes/.git.backup/`

**Decision made:** Use Option B (regular Git for sessions) recommended to avoid ongoing LFS costs; total session data is only 75 MB.
