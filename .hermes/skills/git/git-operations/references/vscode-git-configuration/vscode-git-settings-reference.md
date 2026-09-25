# VSCode Git Extension Settings Reference

## Repository Detection

| Setting                       | Type                                | Default          | Description                                                                                                                                          |
| ----------------------------- | ----------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `git.autoRepositoryDetection` | `"subFolders"` \| `true` \| `false` | `true`           | Scan subfolders of the workspace for git repositories. Set to `"subFolders"` to detect only immediate child folders, or `false` to disable entirely. |
| `git.repositoryScanMaxDepth`  | `number`                            | `-1` (unlimited) | Maximum depth (in folders) to scan for git repositories from the workspace root. Use `3` for monorepos to avoid deep recursion.                      |
| `git.detectSubmodules`        | `boolean`                           | `true`           | Detect git submodules in repositories. Keep this `true` if you use submodules.                                                                       |
| `git.detectSubmodulesLimit`   | `number`                            | `-1` (unlimited) | Maximum number of submodules to detect per repository. Use `10` or your expected max to prevent runaway scanning.                                    |
| `git.ignoredRepositories`     | `string[]`                          | `[]`             | Array of absolute paths to git repositories to ignore entirely. Add nested git repos here rather than disabling submodule detection.                 |

## Output and Logging

| Setting              | Type       | Default                                                                                                                          | Description                                                                                                                                                  |
| -------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `git.commandsToLog`  | `string[]` | `["add", "commit", "diff", "fetch", "merge", "mv", "pull", "push", "rebase", "reset", "revert", "rm", "stash", "status", "tag"]` | Array of git command names to log in the output panel. Set to `[]` to disable all git command logging (stops `> git rev-parse --show-toplevel [XXms]` spam). |
| `git.loggingEnabled` | `boolean`  | `true`                                                                                                                           | Enable Git extension logging. Set to `false` to suppress all Git extension logs (more aggressive than `commandsToLog`).                                      |
| `git.showProgress`   | `boolean`  | `true`                                                                                                                           | Show progress for long-running git operations.                                                                                                               |
| `git.verboseCommit`  | `boolean`  | `false`                                                                                                                          | Include full commit message in the commit input when committing.                                                                                             |

## Performance and Behavior

| Setting                             | Type                                | Default         | Description                                                                                                              |
| ----------------------------------- | ----------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `git.autofetch`                     | `boolean`                           | `true`          | Automatically fetch changes from remote repositories.                                                                    |
| `git.autofetchPeriod`               | `number`                            | `300` (seconds) | How often to autofetch.                                                                                                  |
| `git.autorefresh`                   | `boolean`                           | `true`          | Automatically refresh git information.                                                                                   |
| `git.openRepositoryInParentFolders` | `"always"` \| `"never"` \| `"auto"` | `"auto"`        | When opening a folder that is within a git repo, whether to also open the repo as a workspace folder.                    |
| `git.statusLimit`                   | `number`                            | `10000`         | Maximum number of changed files for which to compute detailed status. Set to `0` for unlimited (may impact performance). |

## Exclude Patterns

These settings control which files/folders VSCode's Git integration ignores:

| Setting                   | Type                      | Description                                                                                                    |
| ------------------------- | ------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `files.exclude`           | `Record<string, boolean>` | Hide files from the explorer. Common: `"**/.git": true`, `"**/node_modules": true`                             |
| `search.exclude`          | `Record<string, boolean>` | Exclude from search.                                                                                           |
| `files.watcherExclude`    | `Record<string, boolean>` | Exclude from file watching (improves performance).                                                             |
| `git.ignoredRepositories` | `string[]`                | **Key setting**: Absolute paths to git repos to ignore entirely (not treated as submodules or separate repos). |

## Troubleshooting Checklist

- [ ] **Console spam `rev-parse --show-toplevel`**: Add nested git repo paths to `git.ignoredRepositories`
- [ ] **Slow workspace opening**: Set `git.repositoryScanMaxDepth` to `3` or lower
- [ ] **Too many submodule detections**: Set `git.detectSubmodulesLimit` to a reasonable number (e.g., `10`)
- [ ] **Repositories not detected at all**: Ensure `git.autoRepositoryDetection` is `"subFolders"` or `true`, and increase `repositoryScanMaxDepth`
- [ ] **Submodules still showing as separate repos**: Path may not match `ignoredRepositories` exactly; ensure absolute path is correct

## Notes

- Settings can be configured at **User** level (applies to all workspaces) or **Workspace** level (`.vscode/settings.json` per project)
- Changes typically require VSCode restart to take effect
- Use absolute paths in `git.ignoredRepositories` (e.g., `"<workspace>/deps"`, not `"deps"` or `"./deps"`)
- Aphrodite-specific path list: `references/vscode-git-configuration/aphrodite-monorepo-paths.md`
