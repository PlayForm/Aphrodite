# Aphrodite Monorepo: VSCode Git Ignored Repositories

## Problem

The PlayForm/Aphrodite monorepo contains a small number of nested git
repositories plus large build-output trees. VSCode's Git extension recursively
scans every nested `.git`, executing hundreds of `git rev-parse
--show-toplevel` calls, flooding the console and degrading performance. The
scan also re-detects the three real submodules on every refresh.

## What Is Actually Nested

| Path                                         | Kind                                                                                              | Belongs in `ignoredRepositories`?         |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `plugins/aphrodite`                          | Real submodule (remote `Source`)                                                                  | ❌ keep detected                          |
| `vendor/headroom`, `vendor/rtk`              | Real submodules (vendored upstreams)                                                              | ❌ keep detected                          |
| `.hermes/`                                   | Hermes runtime home - only a nested git repo if the private Hermes config repo was cloned into it | ⚠️ only then                              |
| `crates/*/target`                            | Cargo build output - NOT a git repo                                                               | ❌ not a repo; use `files.watcherExclude` |
| `build/` (Build.yml / Publish.yml artifacts) | Generated output                                                                                  | ⚠️ only if nested clones appear           |

## Reasoning: Keep Real Submodules Detected

`vendor/headroom`, `vendor/rtk`, and `plugins/aphrodite` are intentional
submodules (gitlinks, mode 160000, recorded in `git ls-tree`). Do NOT add them
to `git.ignoredRepositories` - that hides them from the Source Control view,
blocks the `.githooks` gitlink-bump workflow, and hides the "modified (new
commits)" signal you need before committing pointer updates. Bound the scan
instead with `git.repositoryScanMaxDepth` and `git.detectSubmodulesLimit`.

## Solution

Add only the _accidental_ nested repos to `git.ignoredRepositories` - the
`.hermes` runtime home (when it carries a nested `.git` from a private config
clone) and any generated dirs that contain checked-out clones. All paths use
the public `<workspace>` placeholder - expand it to the absolute clone path in
your local settings (VSCode requires absolute paths), never commit a personal
path.

```json
{
	"git.ignoredRepositories": [
		"<workspace>/PlayForm/Aphrodite/.hermes",
		"<workspace>/PlayForm/Aphrodite/build"
	],
	"git.repositoryScanMaxDepth": 3,
	"git.detectSubmodulesLimit": 10,
	"git.autoRepositoryDetection": "subFolders",
	"git.commandsToLog": []
}
```

## Keep Build Output Out of the Watch

`crates/*/target` are not git repos, so `ignoredRepositories` has no effect on
them - silence the churn with the file watcher instead:

```json
{
	"files.watcherExclude": {
		"**/target": true,
		"**/.hermes/**": true
	},
	"search.exclude": {
		"**/target": true
	}
}
```

## Additional Settings

```json
{
	"git.repositoryScanMaxDepth": 3,
	"git.detectSubmodulesLimit": 10,
	"git.autoRepositoryDetection": "subFolders"
}
```

These preserve submodule detection for the three real submodules while
avoiding deep recursion into generated/build trees.

## Notes

- Settings live in VSCode user settings (`settings.json`) or workspace
  `.vscode/settings.json`; VSCode restart required for changes to take effect.
- The paths listed are specific to this workspace; adjust if the monorepo
  layout changes (new crates, new vendored submodules).
- Regenerate the JSON snippet with
  `scripts/vscode-git-configuration/generate-aphrodite-settings.sh`.
