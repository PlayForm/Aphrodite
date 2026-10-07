**[Compare Aphrodite/v1.6.4...Aphrodite/v1.6.5](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.4...Aphrodite/v1.6.5)**

## Aphrodite 1.6.5 💋 Plugin v2.2.5

### Summary

1.6.5 is a plugin-hardening release driven by the `hermes-agent` catalog
review (PR 118488, teknium1): the native-library download now verifies
against the in-tree `SHA256SUMS.txt` **only** - refusing to install when it
is missing, when its `BINARY_VERSION` does not match, when the target has no
entry, or when an asset has no entry - and the runtime-home lookup no longer
falls back to the legacy `~/.hermes/aphrodite` home. Binary
`1.6.4 → 1.6.5`, plugin `2.2.4 → 2.2.5`.

### Changes

- **Fix (plugin, checksum verification)**: `download.sh` / `download.ps1`
  stop fetching the release-hosted per-target sums and warn-and-skip paths.
  The pinned tree's in-tree `SHA256SUMS.txt` is now the only checksum source
  of truth: missing file, `BINARY_VERSION` mismatch, missing target block, or
  missing asset entry is a hard refusal (exit 1), and a missing
  `shasum`/`sha256sum` tool is a hard refusal rather than a skipped check.
- **Fix (plugin, runtime home)**: `_runtime_home()` drops the legacy
  `~/.hermes/aphrodite` adoption. A non-default `HERMES_HOME` with no install
  reports "not installed" instead of quietly sharing one runtime home across
  profiles (`$APHRODITE_HOME` override and `<hermes-home>/aphrodite` remain
  the only homes; the now-dead adoption helpers and their `tempfile` import
  are removed).
- **Fix (plugin, manifest)**: `plugin.yaml` description text corrects
  "14 tools" → "13 tools" - the release dylib registers exactly the 13
  declared tools (`aphrodite_debug` is `debug_assertions`-gated out of
  release builds), matching what `hermes plugins validate` enforces.
- **Chore (repo, template sync)**: `crates/aphrodite/templates/__init__.py`
  re-synced byte-identical to the plugin shim (drift-guard) so the fix ships
  in every `aphrodite setup`-written loader.

### Verification

- Commit range: `Aphrodite/v1.6.4`..`Aphrodite/v1.6.5` (release-sync
  `23a4700b`, 13 files, +166/-138)
- Delta is shim + download scripts + manifest + template sync - no Rust
  engine change in the range (Rust suites unchanged from 1.6.4's
  verification: 434 passed / 0 failed aphrodite; 56 passed
  aphrodite-hermes)
- Python: `tests/test_layout_check.py` + catalog-review hardening cases

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.5                                                        | Hermes (standalone repo)                 |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.4...Aphrodite/v1.6.5
- **CHANGELOG.md**: [CHANGELOG.md](https://github.com/PlayForm/Aphrodite/blob/Current/CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite