---
name: git-operations
description: "Use when maintaining git repositories in the PlayForm/Aphrodite monorepo. Covers history cleanup, LFS fixes, background commit watching, and VSCode Git config for monorepos and submodules."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: git
category_taxonomy: git/git-operations
date: 2026-09-25
metadata:
    hermes:
        tags: [git, history-cleanup, lfs, bfg, background-watcher, vscode, monorepo, submodules]
        related_skills: [submodule-fleet-management, save-oneshot-tooling]
status: active
---

# Git Operations

Umbrella skill: git repository maintenance, history cleanup, background commit
automation, and VSCode IDE git configuration - framed around the
PlayForm/Aphrodite monorepo (Rust proxy crate `crates/aphrodite`, Hermes bridge
`crates/aphrodite-hermes`, the `plugins/aphrodite` submodule with git remote
`Source`, and vendored deps `vendor/headroom` + `vendor/rtk`). Each section is
a summary; detailed reference material lives in `references/<subskill>/`.

## 1. History cleanup & LFS

Remove large accidentally-committed files from git history, reset repos, configure LFS,
and force-push a clean state. Covers both nuclear (`git init`) and targeted (BFG Repo
Cleaner) approaches.

| Situation                                                                                       | Tool                       |
| ----------------------------------------------------------------------------------------------- | -------------------------- |
| Agent/state repo, no valuable history                                                           | `git init` (nuclear clean) |
| Preserve commit messages/history                                                                | `git filter-repo`          |
| Delete specific folder from history (e.g. a stray `.hermes` committed into `plugins/aphrodite`) | BFG Repo Cleaner           |

```bash
# BFG: delete folder from all history
bfg --delete-folders .hermes --no-blob-protection
git reflog expire --expire=now --all && git gc --prune=now --aggressive

# Verify cleaned
git log --oneline -- .hermes # should return nothing
```

**Pitfall:** purge, then expire reflog and run aggressive gc in the SAME pass - a
cleaned history resurrects from reflog/loose objects otherwise. Verify with `git log`
on the purged path before pushing.

Full procedure, pitfalls, and LFS quota fixes:
`references/git-cleanup/git-cleanup-absorbed.md`.
Worked logs and state-file notes: `references/git-cleanup/`.
Verifier script: `scripts/git-cleanup/verify-clean.py`.

## 2. Background commit watcher

Run a background bash script that polls git status, auto-stages changes, runs a
formatter, and periodically commits via external commit tools. No file editing - only
stage and commit. Watch the Aphrodite root (`crates/`, `plugins/`, `vendor/`,
`.githooks`).

```bash
# Start
terminal(background=true, command="/path/to/watcher-commit.sh", workdir="/project/root")

# Script pattern: loop every POLL_INTERVAL: git status --porcelain → git add -A
# Every COMMIT_INTERVAL if staged: formatter → git add -A → commit (with fallback)
```

| Variable        | Default                       | Purpose                    |
| --------------- | ----------------------------- | -------------------------- |
| `WATCH_DIR`     | Aphrodite root (or submodule) | Working directory          |
| `INTERVAL`      | 60s                           | Commit attempt frequency   |
| `POLL_INTERVAL` | 5s                            | Change detection frequency |

Submodule support: use the `find_repos` generator for recursive multi-repo scanning -
two-pass sweep, staging then committing. Discovers the Aphrodite root plus the three
submodules: `plugins/aphrodite` (remote `Source`), `vendor/headroom`, `vendor/rtk`.

Pitfalls and architecture: `references/background-git-watcher/background-git-watcher-absorbed.md`.
Submodule setup: `references/background-git-watcher/recursive-multi-repo-scanning.md`.

## 3. VSCode git configuration

Configure VSCode's Git extension for the monorepo's nested git repos: the three real
submodules (`plugins/aphrodite`, `vendor/headroom`, `vendor/rtk`) plus accidental
nested repos (e.g. a `.git` inside `~/.hermes` state or generated/build dirs).

### Symptoms

- Console flooded with `> git rev-parse --show-toplevel [XXms]`
- VSCode sluggish on workspace open
- Git output panel overflow

### Key settings

```json
{
	"git.ignoredRepositories": ["<workspace>/PlayForm/Aphrodite/.hermes"],
	"git.commandsToLog": [],
	"git.repositoryScanMaxDepth": 3,
	"git.detectSubmodulesLimit": 10
}
```

**Pitfall:** `git.ignoredRepositories` requires ABSOLUTE paths to each `.git` directory's
parent folder - not globs, not parent-directory catch-alls. Expand `<workspace>` to the
absolute clone path locally; never commit a personal path.

Troubleshooting table and detailed setup:
`references/vscode-git-configuration/vscode-git-configuration-absorbed.md`.
Aphrodite monorepo paths: `references/vscode-git-configuration/aphrodite-monorepo-paths.md`.
Settings reference: `references/vscode-git-configuration/vscode-git-settings-reference.md`.
Script: `scripts/vscode-git-configuration/generate-aphrodite-settings.sh`.

## 4. Shared .githooks for submodule fleets (PlayForm convention)

The canonical PlayForm hook set lives in the Aphrodite repo's `.githooks/`
(post-commit / post-checkout / post-merge / pre-push all exec
`lib/bump-submodule-gitlinks.sh`). Copy it into BOTH the parent and each
submodule repo and set `core.hooksPath .githooks` in each (restore a
package.json `prepare: "git config core.hooksPath .githooks"` when present).
The lib is parent-only (Guard 1 skips when a superproject exists - this is
what fixed the pre-be55fff recursion), enumerates REAL gitlinks (mode 160000) only, honors `ignore = dirty`, and commits pointer bumps with
`--no-verify -s` + `|| true` (the external auto-committer may hold the
index; the bump converges on the next git action).

Family additions on top of Aphrodite's set (Auth-Cloudflare has these):

- post-checkout in a SUBMODULE: if HEAD is detached and `Current` exists,
  `git checkout Current` - keeps submodule commits reachable (a detached
  auto-commit dangles as soon as the submodule is re-synced); verified live:
  `git checkout <sha>` in a submodule auto-returns to Current via the hook.
- pre-commit: refuse commits on a detached HEAD (`git symbolic-ref -q HEAD`
  fails -> exit 1 with "checkout Current"), plus a secret scan for
  `cfut_` / `Authorization: *** / `HERMES_CUSTOM_API_CLOUDFLARE_COM_API_KEY=`.

Pitfall: a secret-scan pattern that contains its own target literal
SELF-BLOCKS the commit of the hook (the scan matches the hook file's own
source). Write the pattern with the bracket trick:
`HERMES_CUSTOM_API_CLOUDFLARE_COM_API_[K]EY=` - the regex still matches a
real key while the file's literal no longer does. Also: after switching a
submodule with `checkout <remote-tracking-ref>` the LOCAL branch stays
stale; a later `checkout Current` lands on the OLD tip and the parent's
post-commit hook bumps the gitlink DOWN to it - fetch + MERGE (never
rebase - user forbids it) the remote tip back in, then re-bump.

## Related skills

- `submodule-fleet-management` - bulk-reset/sync many submodules (fleet-level, read-only mirrors)
- `save-oneshot-tooling` - commit-message generation stack
- `code-quality-improvement` - broader codebase maintenance with git integration
- `hermes-shell-hooks` - can integrate with pre_tool_call hooks for git operations
