# Background Git Watcher (absorbed from `background-git-watcher`)

## Overview

Automate the cycle: detect changes → stage → format → commit - as a background
process. Useful when another agent or the Aphrodite external auto-committer
environment is making file changes and you need to batch-commit them without
manual intervention. Watch the Aphrodite monorepo root (`crates/`, `plugins/`,
`vendor/`, `.githooks`).

## Architecture

```
Loop (every POLL_INTERVAL sec):
  1. git status --porcelain
  2. git add -A (if changes)
  3. Sleep POLL_INTERVAL

Every COMMIT_INTERVAL sec if staged:
  4. Run formatter
  5. git add -A (re-stage formatted)
  6. git gcommit (primary)
  7. git gcommit-hermes (fallback)
```

## Key Parameters

| Variable        | Default                     | Purpose                             |
| --------------- | --------------------------- | ----------------------------------- |
| `WATCH_DIR`     | Aphrodite root or submodule | Working directory                   |
| `INTERVAL`      | 60s                         | How often to attempt a commit       |
| `POLL_INTERVAL` | 5s                          | How often to check for file changes |

## Recursive Multi-Repo Scanning

When the monorepo has submodules (each its own git repo), extend the watcher to
discover and commit in each repo independently. Use the `find_repos` generator
that checks `-f .git` (submodule file) and `-d .git` (bare directory). For
Aphrodite that is the root plus `plugins/aphrodite` (remote `Source`),
`vendor/headroom`, and `vendor/rtk`.

## Running

```bash
terminal(background=true, command="/path/to/watcher-commit.sh", workdir="/project/root")
process(action="poll", session_id="...")
```

## Pitfalls

- **Empty STAGED_FILES_COUNT:** Guard with `STAGED_FILES_COUNT=${STAGED_FILES_COUNT:-0}`
- **`local` outside functions:** `local` only valid inside bash functions
- **`local` in subshell pipelines:** `while read ... done < <(find_repos)` with process substitution
- **pre_tool_call hooks on write_file:** Disable hook in config.yaml if it corrupts shebangs
- **Terminal buffering:** Use `exec > >(tee -a "$HOME/.hermes/tmp/watcher-commit.log") 2>&1` at top of script
- **Auto-committer race:** an external auto-committer may hold the index; commit with `|| true` fallbacks so the watcher converges on the next cycle
- **Submodule gitlinks:** committing inside a submodule does not bump the parent gitlink - the parent must commit the pointer update (or rely on the `.githooks` post-commit bump)

See `references/background-git-watcher/` for recursive multi-repo scanning details.
