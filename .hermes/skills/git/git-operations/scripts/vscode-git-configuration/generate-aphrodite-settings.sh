#!/usr/bin/env bash
# Generate VSCode Git ignoredRepositories settings for the PlayForm/Aphrodite monorepo
# Outputs a JSON snippet for settings.json. VSCode requires ABSOLUTE paths, so
# set WORKSPACE_ROOT to your absolute clone path when running; the '<workspace>'
# placeholder is the public convention - never commit a personal path.
set -euo pipefail

WORKSPACE_ROOT="${WORKSPACE_ROOT:-<workspace>/PlayForm/Aphrodite}"

cat <<EOF
{
  "git.ignoredRepositories": [
    "${WORKSPACE_ROOT}/.hermes",
    "${WORKSPACE_ROOT}/build"
  ],
  "git.repositoryScanMaxDepth": 3,
  "git.detectSubmodulesLimit": 10,
  "git.autoRepositoryDetection": "subFolders",
  "git.commandsToLog": [],
  "files.watcherExclude": {
    "**/target": true,
    "**/.hermes/**": true
  },
  "search.exclude": {
    "**/target": true
  }
}
EOF