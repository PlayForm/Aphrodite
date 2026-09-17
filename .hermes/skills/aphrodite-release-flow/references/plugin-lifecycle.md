# Aphrodite Plugin Lifecycle (install / uninstall / key sourcing)

## Canonical runtime layout (post-relocation, 2026-09)

The plugin is a PURE LOADER: it never ships or holds runtime content. The
user-data home `~/.hermes/aphrodite/` is the single canonical store:

- `aphrodite.toml` - config, ALWAYS here (or `APHRODITE_CONFIG_PATH`); the
  binary's search order is `./aphrodite.toml` → `~/.hermes/aphrodite/`,
  never the plugin dir. `aphrodite setup` writes it here.
- `binaries/` - `aphrodite` binary + `libaphrodite_hermes.dylib` (loader
  resolution: env override → `~/.hermes/aphrodite/binaries/` → legacy
  plugin-dir fallback with a warning; `download.sh/.ps1` default
  `BINARY_DIR` to the canonical home).
- `directives/` - materialized HERE by the dylib
  (`aphrodite_hermes_materialize_directives`) from the binary's embedded
  `builtin_directives/` (in `crates/aphrodite/src/`); never shipped in the
  plugin dir, user-modified files never overwritten.
- `ccr.db`, `hotreload/`, logs.

Ships in the plugin repo ONLY: `__init__.py` (loader shim), `plugin.yaml`,
`download.sh/.ps1`, `BINARY_VERSION`, `tests/`. No `binaries/`, no
`directives/`, no `aphrodite.toml` in the plugin dir - a startup self-heal
(`layout_check.py` + `layout_schema.json`) detects and relocates misplaced
config/binaries, recreates the plugin symlink, and quarantines stray
plugin-source copies found in the runtime home (never deletes user data).
Dev-side skills live in the MONOREPO `.hermes/skills/`, never shipped.
Profiles were removed entirely - no `profiles/` anywhere.

The old `Maintain/install.sh/.ps1/.bat` installers are DELETED (they
created wrong symlinks); the plugin self-installs + self-heals at startup.

## Install layouts (both via a symlink at `~/.hermes/plugins/aphrodite`)

- **Standalone layout**: `~/.hermes/plugins/aphrodite` → symlink →
  `~/.hermes/aphrodite` (the data dir). Everything lives under
  `~/.hermes/aphrodite/`: `binaries/`, `directives/`, `ccr.db`, `hotreload/`,
  logs, and `aphrodite.toml`. `aphrodite setup` writes into this layout
  (binary + config + plugin.yaml + shim + symlink + registration).
- **Dev-loop layout**: symlink → a git checkout of the plugin repo (the
  monorepo submodule `plugins/aphrodite` or any clone). The loader prefers
  the canonical `~/.hermes/aphrodite/binaries/`; dev builds symlink
  `binaries/aphrodite` + `binaries/libaphrodite_hermes.dylib` →
  `target/release/` in the canonical home; `_ensure_binaries` sees them
  exist and skips the download - so the plugin runs the LOCAL build, not
  the released one. The legacy plugin-dir `binaries/` is a warning-flagged
  fallback only; downloads no longer write into the checkout.
- `cargo clean` dangles dev symlinks → the plugin then auto-downloads
  release binaries into the canonical home (safe - but you are no longer
  testing the dev build).
- **Auto-update trigger is EXISTENCE, not version**: `_ensure_binaries`
  returns when both binary paths exist - bumping `BINARY_VERSION` alone
  never re-downloads, and `_check_version` only WARNS on mismatch. To
  force a refresh: delete `binaries/aphrodite` +
  `binaries/libaphrodite_hermes.dylib` in the canonical home (or run
  `download.sh` manually); next register() then fetches per the new
  version.
- `download.sh` version fallback chain: `BINARY_VERSION` file →
  `Cargo.toml` (monorepo checkouts) → GitHub API latest. In a monorepo
  checkout the Cargo.toml fallback wins when BINARY_VERSION is stale - so
  the dev loop can run the newest release (e.g. engine reports 1.4.5 while
  BINARY_VERSION still says 1.4.3) without any bump.

## Uninstall (complete removal from ~/.hermes)

1. Preserve anything valuable FIRST (skills, logs) to
   `~/Developer/.playform/Temporary/`.
2. `rm ~/.hermes/plugins/aphrodite` (the symlink only - never the checkout),
   `rm -rf ~/.hermes/aphrodite` (data dir).
   2b. Sweep for LEGACY symlinks, not just the current paths: an old setup
   may have left bare `~/.hermes/directives` → repo `directives/` links
   predating the relocation (directives now live in
   `~/.hermes/aphrodite/directives/`, materialized by the binary - the bare
   path is forbidden). Sweep everything:
   `find ~/.hermes -maxdepth 3 -type l | while read l; do readlink "$l"; done`
   and remove any link whose target matches the Aphrodite repo. Do NOT
   remove `~/.hermes/plugins/aphrodite` re-created by the user as the live
   install, nor dev-side skill symlinks under the monorepo's `.hermes/`.
3. Config refs via the sanctioned CLI - `hermes plugins uninstall` REFUSES
   out-of-tree symlink installs, so use:
    - `hermes config unset plugins.entries.aphrodite`
    - `hermes config set plugins.enabled '[...minus aphrodite]'`
    - `platform_toolsets.*` / `known_plugin_toolsets.*` are dynamic sections;
      `hermes config set` needs `--force` there ("custom top-level key"
      notice) or they regenerate on next start without the plugin.
4. Cache: remove the aphrodite key from
   `~/.hermes/cache/plugin_toolset_keys.json`.
5. Remove plugin-shipped skills from `~/.hermes/skills/` (preserve first).
6. Verify: `grep -c aphrodite ~/.hermes/config.yaml` == 0, no dirs, no PATH
   binary, no cargo-installed `aphrodite`/`aphrodite-hermes-setup`
   (`cargo uninstall aphrodite aphrodite-hermes` - the setup binary's
   package is `aphrodite-hermes`, not `aphrodite-hermes-setup`).
7. A dying gateway with the plugin still in memory re-creates
   `~/.hermes/aphrodite/` on drain (empty dir + proxy-stderr log) - remove
   it again AFTER the restart; the fresh gateway cannot recreate it.

## Proxy API key sourcing

- The proxy processes die at spawn with `no API key configured` when no key
  is set - Hermes' own provider config is NOT reused by the plugin. Sources:
  `APHRODITE_API_KEY` env (set in `~/.hermes/.env`, loaded at Hermes start)
  or `api_key` in the toml.
- The key chain is provider-agnostic: config → `APHRODITE_API_KEY` only
  (DeepSeek-specific fallbacks were removed deliberately - no provider
  defaults, no provider-named key vars).
- `~/.hermes/config.yaml` may hold the working key as `${SOME_ENV_VAR}`
  interpolation; resolve the referenced var (it may already live in
  `~/.hermes/.env`) before writing it anywhere. Never print the value.

## Version tracks (three, never conflated)

- Binary `1.4.x` (parent Cargo.tomls + package.json + README badge).
- Plugin `2.1.x` (`plugin.yaml` version + install_message) - the checkout
  manifest.
- `BINARY_VERSION` file in the plugin repo = the binary release the plugin
  pairs with (what `download.sh`/`download.ps1` fetch). Bump LAST, only
  after the named release exists.

## Default system prompt vs plugin injection

- The default Hermes system prompt is assembled from `~/.hermes/SOUL.md`
  (feedback.md does NOT reach the prompt). Aphrodite instruction content
  belongs in the plugin's runtime injection (`session_inject` turn-0 +
  directives via `pre_llm_call`), NOT in SOUL.md - keep SOUL.md
  provider-agnostic so the prompt stays clean regardless of plugin state.
- A fresh session with no `~/.hermes/aphrodite/aphrodite.toml` runs on code
  defaults - the stats it reports (thresholds) reveal which defaults were in
  effect.
