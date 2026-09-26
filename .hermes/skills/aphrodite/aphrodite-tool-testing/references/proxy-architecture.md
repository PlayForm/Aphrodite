# Proxy Architecture (context)

- **Token proxy** (token listener - port is a config property, default
  `:9798`; read live from `aphrodite.toml` `ports`, never assume) -
  token-level compression; management endpoints require an API key.
- **Cache proxy** (cache listener - port is a config property, default
  `:9797`; read live) - cache-mode compression; management endpoints accept
  any loopback caller.
- Both share the CCR database at `~/.hermes/aphrodite/ccr.db`.
- Binary: `~/.hermes/aphrodite/binaries/aphrodite` (CLAIM: auto-updated - no
  probe in the source).
- Runtime home layout: `~/.hermes/aphrodite` holds binaries/, `aphrodite.toml`,
  the BINARY_VERSION pin, `ccr.db`, directives/, and logs/;
  `~/.hermes/plugins/aphrodite` holds ONLY `plugin.yaml` + `__init__.py`
  (hooks-only plugin install; `aphrodite setup` writes both).
- CLAIM: dylib hot-reloads on file modification - no probe in the source.

The two ports are runtime-derived: read them live from the active config
(`aphrodite.toml` `ports`), never assume the defaults.
