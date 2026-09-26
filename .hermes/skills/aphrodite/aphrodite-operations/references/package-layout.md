# Package and installer layout

Reference data for `aphrodite-operations`. The rules that decide when to check
these file sets live in SKILL.md; this file holds the byte-exact lists.

## Repo-side package: `plugins/aphrodite/`

The repo-side `plugins/aphrodite/` carries the full package:

- `__init__.py`
- `_bindings.py`
- `BINARY_VERSION`
- `download.sh` / `download.ps1`
- `layout_check.py`
- `layout_schema.json`
- `plugin.yaml`
- `README.md`
- `SHA256SUMS.txt`
- `tests/`

There is no `_core/` and no `_hooks/` in the repo package.

## Installed loader: `~/.hermes/plugins/aphrodite/`

The installed loader holds ONLY `plugin.yaml` + `__init__.py` (hooks-only layout).

## Runtime home: `~/.hermes/aphrodite/`

Layout (per `verification.source_of_truth` in SKILL.md):

- `binaries/` - `aphrodite` + `libaphrodite_hermes.dylib`, optional `libaphrodite.dylib`
- `aphrodite.toml`
- `BINARY_VERSION` pin (current: 1.6.2)
- `ccr.db`
- `directives/`
- `proxy-stderr.log`
