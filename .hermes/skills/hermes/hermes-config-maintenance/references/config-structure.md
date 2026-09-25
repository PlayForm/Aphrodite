# Hermes config.yaml - toolset-related block layout

Top-level `~/.hermes/config.yaml` and each `~/.hermes/profiles/<name>/config.yaml`
share the same schema. Relevant blocks for toolset cleanup around the
Aphrodite plugin:

## platform_toolsets

Per-platform toolset allowlists. **This is the ONLY key that produces
"references unknown toolset" startup warnings.** Shape:

```yaml
platform_toolsets:
    cli:
        [
            clarify,
            code_execution,
            context_engine,
            delegation,
            file,
            memory,
            session_search,
            skills,
            terminal,
            todo,
        ]
    discord: [hermes-discord]
    google_chat: [] # plugin not installed -> empty list
    homeassistant: [hermes-homeassistant]
    qqbot: [hermes-qqbot]
    signal: [hermes-signal]
    slack: [hermes-slack]
    teams: [] # plugin not installed -> empty list
    telegram: [hermes-telegram]
    whatsapp: [hermes-whatsapp]
    yuanbao: [hermes-yuanbao]
```

## known_plugin_toolsets

Stale auto-discovery cache populated by earlier runs. Not the warning source,
but dead names here should be removed alongside `platform_toolsets`. Real entries
(like `spotify`) stay.

```yaml
known_plugin_toolsets:
    cli: [] # cleaned of a2a/aphrodite/etc.
```

## known_builtin_toolsets

Lists builtin toolsets per platform. Normally contains only valid names; rarely
needs editing.

## plugins

```yaml
plugins:
    disabled: [... platforms/a2a ...] # real plugin ids - LEAVE ALONE
    enabled: [aphrodite, ...] # real plugins - LEAVE ALONE
    entries: { aphrodite: { allow_tool_override: false } }
```

`platforms/a2a`, `aphrodite`, etc. here are **plugin** references, NOT toolsets.
Editing these to "fix" toolset warnings is wrong. The CCR tools the plugin
exposes (`aphrodite_retrieve`, `aphrodite_compress`, `aphrodite_stats`, ...)
are runtime tools, not config toolset names - they never appear in
`platform_toolsets` allowlists.

## _config_version

Integer at the bottom (e.g. `_config_version: 39`). Bumped by `hermes` on
format migrations. Don't hand-edit.

## The Aphrodite plugin's own config

Plugin settings live OUTSIDE config.yaml, in the runtime home:

- `~/.hermes/aphrodite/aphrodite.toml` - plugin configuration
- `~/.hermes/aphrodite/binaries/` - proxy binaries
- `~/.hermes/aphrodite/hotreload/` - hot-reloaded `libaphrodite_hermes.dylib`

A plugin detach or toolset cleanup never edits these files; it removes their
registration from Hermes config (`plugins.*` entries, env var names,
`platform_toolsets` / `known_plugin_toolsets` leftovers).

## `aphrodite setup` flags vs the toml template

`aphrodite setup` accepts `--api-key` / `--api-url` / `--model`, but the toml
template substitutes ONLY the ports (cache 9797 / token 9798) - there are no
placeholders for the other three, so they are parsed and ignored. Never claim
the flags write into the toml. Upstream config is env-driven:
`APHRODITE_API_URL` and `APHRODITE_MODEL`; the proxy's API key comes from the
`APHRODITE_API_KEY` env var or a `[defaults] api_key` in the toml.

## Where configs live

- Default: `~/.hermes/config.yaml`
- Profiles: `~/.hermes/profiles/<name>/config.yaml`
  (e.g. `dev-aphrodite`, plus any others you create)
- Dev profile plugin binding: `~/.hermes/profiles/dev-aphrodite/plugins/aphrodite`
  is a symlink into the repo's `plugins/aphrodite` (source mode)
