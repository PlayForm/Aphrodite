# Aphrodite plugin uninstall - worked detail

Full procedure behind the SKILL.md "Full uninstall" section. Inventory FIRST (names only, never values), then remove, then verify.

## 1. Inventory every footprint

- `~/.hermes/plugins/aphrodite` plugin dir - loader only: `plugin.yaml` + `__init__.py`, no symlink since `aphrodite setup` writes it.
- The `~/.hermes/aphrodite/` runtime home: aphrodite.toml, binaries/, BINARY_VERSION, ccr.db, directives/, logs/, hotreload/.
- `~/.hermes/bin/<binary>`.
- `~/.hermes/cache/aphrodite/`.
- Skills symlinks: `~/.hermes/skills/aphrodite-*`.
- The plugin's env var NAMES in `~/.hermes/.env` (never values).
- Every config.yaml block referencing the plugin: `model:` base_url/api_key, `custom_providers:` entry, `moa` reference_models entries, plus leftover `platform_toolsets` / `known_plugin_toolsets` names.

## 2. Back up to scratch FIRST

Copy config.yaml + .env into `~/.hermes/tmp/`; chmod 600 the .env copy. The removal is then reversible, and the credential survives only in the 600-perm backup.

## 3. Remove .env lines by VAR NAME

Filter script: split on `=`, drop only the named vars, write the rest back. Never read-then-rewrite the whole secrets file through context; never print values.

## 4. Update config.yaml via the guarded route

Python replace or `hermes config` (see the SKILL.md write-guard section). When removing the last entry of a list key, substitute the empty list (`reference_models: []`), never delete the key bare (valid YAML requires the value). Removing the active `model:` block leaves the default profile without a model for the next start - state that in the report.

## 5. Classify by OWNERSHIP before removing

A legacy `custom_providers:` entry + its `key_env` var and the active `model:` block may belong to a SEPARATE custom provider still in use ("the one you're currently using"), not to the plugin being detached. The plugin's own vars are the `*_BASE_URL`/account/token names; the legacy key_env is the custom provider's. When ownership is ambiguous, remove only the plugin's pieces and leave the custom provider's config in place. If over-removal happened, restore the exact pieces from the scratch backup (`config.yaml.bak` / `env.bak`) and re-verify YAML.

## 6. Verify

`grep -rni aphrodite ~/.hermes` - remaining hits in `~/.hermes/pastes/` are historical session logs, not plugin wiring; leave them. `~/.hermes/plugins/` should show only unrelated plugins.

## Symlinked/out-of-tree installs (manual uninstall)

`hermes plugins uninstall <name>` rejects any plugin whose symlink resolves outside `~/.hermes/plugins/` ("Invalid plugin name ... resolves outside the plugins dir") and has no `--force`. Manual sequence:

- `hermes config set plugins.enabled '[...]'` (rewrite the list without the name) + `hermes config unset plugins.entries.<name>`.
- `platform_toolsets.*` and `known_plugin_toolsets.*` are DERIVED keys: `hermes config set` refuses them ("not a recognized config key") unless passed `--force`, which then works - e.g. `hermes config set known_plugin_toolsets.cli '["a2a"]' --force`.
- Stale cache: `cache/plugin_toolset_keys.json` keeps a `toolset_keys` entry per installed plugin - remove the name surgically (the file regenerates; the entry is the only leftover a `grep` of config.yaml misses).
- Plugin-shipped skills sit flat in `~/.hermes/skills/<name>-*` - preserve them to `~/.hermes/tmp/` first (never delete worth-keeping material outright), then remove.

## Old gateway drain re-creates the runtime dir

A still-running OLD gateway (plugin loaded in memory, pre-restart) RE-CREATES the runtime dir during drain: `proxy-stderr.log` timestamps match the restart moment and the dir reappears after removal. Re-verify AFTER the gateway has restarted, not just immediately after removal; the new gateway has no registration and cannot recreate it.

## Claim-to-test

| Claim | Test | Pass condition |
| --- | --- | --- |
| Removal leaves no config registration | `grep -rni aphrodite ~/.hermes` | Hits only in `~/.hermes/pastes/` |
| Removal is reversible | `test -f ~/.hermes/tmp/config.yaml.bak` | Backup present, `.env` copy at 600 |
| Derived keys accept --force | `hermes config set known_plugin_toolsets.cli '["a2a"]' --force` | Key updates; YAML parses |