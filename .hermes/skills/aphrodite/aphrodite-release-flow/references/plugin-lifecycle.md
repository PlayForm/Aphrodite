# Aphrodite Plugin Lifecycle (decision-tree runbook)

Companion to `engine-health-debugging.md` (running-engine diagnosis). This
file is the decision tree for plugin install/uninstall/key-sourcing and for
the three load-time symptom classes: plugin load failure, hook does not fire,
and stale dylib vs reload confusion. Same progression as the engine runbook:
classify the symptom → collect immutable evidence → bounded causes → safe
repair → exit criteria.

## Canonical runtime layout (post-relocation, 2026-09)

The plugin is a PURE LOADER: it never ships or holds runtime content. The
user-data home `~/.hermes/aphrodite/` is the single canonical store:

- `aphrodite.toml` - config, ALWAYS here (or `APHRODITE_CONFIG_PATH`); the
  binary's search order is `./aphrodite.toml` → `~/.hermes/aphrodite/`,
  never the plugin dir. `aphrodite setup` writes it here.
- `binaries/` - `aphrodite` binary + `libaphrodite_hermes.dylib` (loader
  resolution: env override → `~/.hermes/aphrodite/binaries/` → legacy
  plugin-dir fallback with a warning; `download.sh/.ps1` default `BINARY_DIR`
  to the canonical home).
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
Profiles were removed entirely - no `profiles/` anywhere. The old
`Maintain/install.sh/.ps1/.bat` installers are DELETED (they created wrong
symlinks); the plugin self-installs + self-heals at startup.

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
  never re-downloads, and `_check_version` only WARNS on mismatch. To force
  a refresh: delete `binaries/aphrodite` + `binaries/libaphrodite_hermes.dylib`
  in the canonical home (or run `download.sh` manually); next register()
  then fetches per the new version.
- `download.sh` version fallback chain: `BINARY_VERSION` file → `Cargo.toml`
  (monorepo checkouts) → GitHub API latest. In a monorepo checkout the
  Cargo.toml fallback wins when BINARY_VERSION is stale - so the dev loop can
  run the newest release (e.g. engine reports 1.4.5 while BINARY_VERSION
  still says 1.4.3) without any bump.

## Symptom decision tree

### L1 - Plugin load failure

**Evidence to collect**

- Hermes start log / plugin registration output; `~/.hermes/plugins/aphrodite`
  symlink target (`readlink`); plugin dir contents vs canonical layout;
  `~/.hermes/aphrodite/binaries/` presence; loaded dylib version.

**Bounded likely causes (discriminating test → safe repair)**

1. Missing/incompatible binary or dylib - `_ensure_binaries` could not fetch
   (no network, or `BINARY_VERSION` names a release whose assets do not
   exist) → check `download.sh` result and the release asset list; fix the
   pointer or the assets.
2. Broken symlink / wrong layout - symlink points at a stale path, or
   config/binaries sit in the plugin dir instead of the canonical home → the
   self-heal (`layout_check.py` + `layout_schema.json`) relocates/quarantines
   on startup; if it did not, recreate the symlink per the layout above.
3. Out-of-tree install refused - `hermes plugins uninstall` refuses
   out-of-tree symlink installs → use `hermes config unset
plugins.entries.aphrodite` + `hermes config set plugins.enabled '[...minus
aphrodite]'` (see Uninstall below).
4. Key missing at proxy spawn - "no API key configured" (engine runbook S1).

**Safe repair**

- One dimension at a time: fix the layout, then the pointer, then the key;
  restart Hermes after each; re-check the load.

**Exit criteria**

- Plugin registers in a fresh Hermes start; `aphrodite_stats` responds;
  version pair matches.

**Prohibited**

- Hand-editing generated bindings; deleting user data to "repair" a layout;
  re-creating removed installers or hooks.

### L2 - Hook does not fire

**Evidence to collect**

- Hermes valid-hook registry (which names are accepted); the actual
  `invoke_hook` call site in Hermes source (keyword names - line numbers are
  observational evidence, never durable coordinates); registration name in
  the plugin (`onsessionstart`, not `sessionstart`); a temporary log at the
  handler; restart + trigger + log inspection.

**Bounded likely causes (discriminating test → safe repair)**

1. Wrong registration name - the plugin registers a name the framework does
   not invoke (historical: `sessionstart` vs `onsessionstart`) → register the
   valid name; re-test.
2. Handler signature mismatch - the handler does not accept an explicitly
   required keyword or `**kwargs` → align the handler to the invocation's
   keyword names (the `stdout` vs `output` terminal-hook mismatch erases
   output despite a successful-looking invocation).
3. Hook registered but never invoked - dead integration → verify the
   invocation site exists; re-test in a fresh session.

**Safe repair**

- Fix the registration name/signature; restart Hermes (a fresh process is
  required - results from a stale process are not evidence); trigger the
  hook; inspect the log.

**Exit criteria**

- One log entry per trigger in a fresh process; the hook's effect is
  observable.

**Prohibited**

- Renaming invocation-site keywords to match the plugin; assuming the
  parameter names from memory instead of the verified source.

### L3 - Stale dylib vs reload confusion

**Evidence to collect**

- Loaded dylib version (`aphrodite_stats`) vs freshly built version;
  `hotreload/` file listing (`.dylib.<pid>.<n>` copies); whether Hermes was
  restarted after the change.

**Bounded likely causes (discriminating test → safe repair)**

1. Hot-reload already happened - the version bump swapped the dylib and
   restarted the engine (session counters reset to zero - NORMAL) → confirm
   the new version is loaded.
2. Stale process - the new dylib was copied into place but only takes effect
   on the next load → restart the Hermes session, then re-probe.
3. Stale symlink target - the dev-loop symlink points at an old build
   (e.g. `cargo clean` dangled it) → rebuild and re-link, or let
   `_ensure_binaries` fetch the released binaries.

**Safe repair**

- Rebuild `-p aphrodite -p aphrodite-hermes`; restart the session; re-probe
  version + behavior.

**Exit criteria**

- A fresh process reports the NEW version AND the new behavior.

**Prohibited**

- Attributing behavior to the new build while the old dylib is still loaded;
  editing `_bindings.py` by hand.

## Uninstall (complete removal from ~/.hermes)

1. Preserve anything valuable FIRST (skills, logs) to the personal scratch dir.
2. `rm ~/.hermes/plugins/aphrodite` (the symlink only - never the checkout),
   `rm -rf ~/.hermes/aphrodite` (data dir).
3. Sweep for LEGACY symlinks, not just the current paths: an old setup may
   have left bare `~/.hermes/directives` → repo `directives/` links
   predating the relocation (directives now live in
   `~/.hermes/aphrodite/directives/`, materialized by the binary - the bare
   path is forbidden). Sweep everything:
   `find ~/.hermes -maxdepth 3 -type l | while read l; do readlink "$l"; done`
   and remove any link whose target matches the Aphrodite repo. Do NOT remove
   `~/.hermes/plugins/aphrodite` re-created by the user as the live install,
   nor dev-side skill symlinks under the monorepo's `.hermes/`.
4. Config refs via the sanctioned CLI - `hermes plugins uninstall` REFUSES
   out-of-tree symlink installs, so use:
    - `hermes config unset plugins.entries.aphrodite`
    - `hermes config set plugins.enabled '[...minus aphrodite]'`
    - `platform_toolsets.*` / `known_plugin_toolsets.*` are dynamic sections;
      `hermes config set` needs `--force` there ("custom top-level key"
      notice) or they regenerate on next start without the plugin.
5. Cache: remove the aphrodite key from
   `~/.hermes/cache/plugin_toolset_keys.json`.
6. Remove plugin-shipped skills from `~/.hermes/skills/` (preserve first).
7. Verify: `grep -c aphrodite ~/.hermes/config.yaml` == 0, no dirs, no PATH
   binary, no cargo-installed `aphrodite`/`aphrodite-hermes-setup`
   (`cargo uninstall aphrodite aphrodite-hermes` - the setup binary's
   package is `aphrodite-hermes`, not `aphrodite-hermes-setup`).
8. A dying gateway with the plugin still in memory re-creates
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

- Binary `1.5.x` (parent Cargo.tomls + package.json + README badge).
- Plugin `2.2.x` (`plugin.yaml` version + install_message) - the checkout
  manifest.
- `BINARY_VERSION` file in the plugin repo = the binary release the plugin
  pairs with (what `download.sh`/`download.ps1` fetch). Bump LAST, only
  after the named release exists (ledger owner: `aphrodite-release-workflow`
  §1).

## Default system prompt vs plugin injection

- The default Hermes system prompt is assembled from `~/.hermes/SOUL.md`
  (feedback.md does NOT reach the prompt). Aphrodite instruction content
  belongs in the plugin's runtime injection (`session_inject` turn-0 +
  directives via `pre_llm_call`), NOT in SOUL.md - keep SOUL.md
  provider-agnostic so the prompt stays clean regardless of plugin state.
- A fresh session with no `~/.hermes/aphrodite/aphrodite.toml` runs on code
  defaults - the stats it reports (thresholds) reveal which defaults were in
  effect.
