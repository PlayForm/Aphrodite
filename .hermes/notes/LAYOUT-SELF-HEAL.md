# Layout Self-Heal (layout_check.py)

The Aphrodite plugin self-heals the `~/.hermes` layout at startup. Installers
are gone: this module is the mechanism that creates and maintains the layout
(plugin symlink, profile symlinks, runtime home), and it repairs broken
installs automatically (wrong symlinks, misplaced config/binaries, stray
plugin-source copies, dangling links, stale copies).

## Concept

- `plugins/aphrodite/layout_schema.json` is the machine-readable reference
  schema of the canonical layout: one entry per path with `kind`
  (dir|file|symlink), `required`, `contents_forbidden`, `canonical_target`,
  and `overrides` (env vars that relocate the path). It is versioned via
  `schema_version`.
- `layout_check.check_and_heal(home_dir=None, dry_run=False)` walks the
  actual `~/.hermes` state, compares it against the schema, and repairs
  deviations. It returns a report dict
  `{schema_version, dry_run, home_dir, checks, mismatches, actions_taken,
  warnings}`.
- Self-heal acts ONLY on `~/.hermes` runtime paths. It never deletes user
  data (everything displaced is moved to the runtime home, verified
  byte-identical first, and never overwrites an existing destination), never
  mutates the repo checkout (dev checkouts are reported, not moved), and is
  idempotent (a healed layout heals to zero actions).
- It never raises: every failure is logged via `logging.getLogger("aphrodite")`
  and degrades to report-only.

## Checks performed

1. Schema load (missing/malformed -> warn, report-only, never crash).
2. Required runtime dirs exist (`~/.hermes/aphrodite` is created if missing).
3. Plugin path `~/.hermes/plugins/aphrodite` is a symlink to the plugin's real
   path (created if missing; empty real dir converted; non-empty real dir or
   dangling link warn-and-skip).
4. No forbidden contents inside the plugin dir (`binaries/`, `aphrodite.toml`,
   `ccr.db`): moved into the runtime home (copy + verify + remove; identical
   duplicates deduped; differing destination warn-and-skip). Binaries newer
   than their canonical runtime-home copy are warn-and-skip (mid-refactor
   protection).
5. No stray plugin-source files in the runtime home (`__init__.py`,
   `plugin.yaml`, `README.md`, `download.sh`, `download.ps1`, `BINARY_VERSION`,
   `layout_check.py`, `layout_schema.json`): quarantined to
   `~/.hermes/aphrodite/.stale-backup/` when content verifies as an old or
   current plugin version; warn-and-skip when it cannot be compared.
6. Runtime binaries (`binaries/aphrodite` + platform lib) must not resolve
   into the plugin dir: such symlinks are replaced with verified real copies.
7. All 7 profile symlinks `~/.hermes/profiles/aphrodite-*` exist and point at
   the repo `profiles/` (recreated when missing; non-empty real dirs or
   dangling links warn-and-skip).
8. `aphrodite.toml` present in the runtime home (or at the env override).

The repo root for profile targets is resolved from the plugin's real path
(`Path.resolve()` - the plugin may itself be a symlink): `<plugin>/../..` or
`<plugin>/..` whichever contains `profiles/`.

## Env-var overrides honored

- `APHRODITE_CONFIG_PATH` - relocates `aphrodite.toml`; checks target this
  path and misplaced config is moved here.
- `APHRODITE_BINARY_PATH` - relocates the `aphrodite` binary.
- `APHRODITE_HERMES_DYLIB_PATH` - relocates `libaphrodite_hermes.{dylib,so,dll}`.

## Integration point

In `plugins/aphrodite/__init__.py`, invoke the check early in plugin startup
- at module-level init or inside the plugin's activate/registration entry,
before binaries or config are consumed:

```python
from .layout_check import check_and_heal

try:
    check_and_heal()  # never raises; failures degrade to report-only
except Exception:
    pass  # belt and braces: self-heal must never abort registration
```

`check_and_heal()` never raises, so the call cannot abort plugin
registration; the surrounding try/except is defensive only.