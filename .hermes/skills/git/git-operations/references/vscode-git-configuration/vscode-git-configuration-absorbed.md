# VSCode Git Configuration (absorbed from `vscode-git-configuration`)

## Overview

Configure VSCode's Git extension for complex repositories, monorepos, and environments with nested git repositories. For the PlayForm/Aphrodite monorepo the nested repos are the three real submodules (`plugins/aphrodite`, `vendor/headroom`, `vendor/rtk`) plus accidental `.git` dirs in runtime/generated trees.

## Common Symptoms

- Console shows `> git rev-parse --show-toplevel [XXms]` repeating hundreds of times
- VSCode becomes sluggish on workspace open
- Git output panel flooded with timing logs

## Root Causes

1. Unlimited recursive scanning: `git.autoRepositoryDetection: "subFolders"`, `repositoryScanMaxDepth: -1`
2. No command filtering: `git.commandsToLog` unset
3. Missing exclusions: `git.ignoredRepositories` doesn't include nested git paths

## Configuration Strategy

### Step 1: Identify Problematic Nested Git Repos

```bash
find /path/to/workspace -type d -name ".git" | grep -vE "(your|real|submodule|path)"
```

### Step 2: Configure `git.ignoredRepositories`

Use absolute paths to each `.git` directory's **parent folder**. Individual git roots, not parent folders. In the Aphrodite monorepo, keep the three real submodules OUT of this list (they must stay detected for the gitlink-bump workflow); ignore only accidental nested repos such as `.hermes` when it carries a private config clone.

### Step 3: Silence Command Logging

```json
{ "git.commandsToLog": [] }
```

### Step 4: Reduce Scan Depth Limits

```json
{ "git.repositoryScanMaxDepth": 3, "git.detectSubmodulesLimit": 10 }
```

### Step 5: Keep Submodule Detection Enabled

```json
{ "git.detectSubmodules": true }
```

## Troubleshooting

| Symptom               | Cause                       | Fix                                       |
| --------------------- | --------------------------- | ----------------------------------------- |
| No reduction in logs  | `git.commandsToLog` not set | Set to `[]` in correct settings scope     |
| Some repos still scan | Relative/glob paths         | Use absolute paths to .git parent folders |
| Nothing changes       | VSCode not fully restarted  | Quit VSCode completely and relaunch       |

See `references/vscode-git-configuration/` for Aphrodite monorepo-specific paths and complete settings catalog.
See `scripts/vscode-git-configuration/generate-aphrodite-settings.sh` for generating `ignoredRepositories`.
