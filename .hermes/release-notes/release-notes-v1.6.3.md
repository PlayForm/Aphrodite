**[Compare Aphrodite/v1.6.2...Aphrodite/1.6.3](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.2...Aphrodite/1.6.3)**

## Aphrodite 1.6.3 💋 Plugin v2.2.3

### Summary

1.6.3 fixes issue #40: the plugin's Python shim and its Rust half (dylib +
proxy binary) each resolved the runtime home on their own - the shim from
`$HERMES_HOME`, the Rust side from `$HOME` - so whenever `HERMES_HOME !=
$HOME/.hermes` (the shipped Docker image, profile gateways) the shim looked
for binaries in a directory that never existed, disabled the plugin, and
orphaned the previous runtime home (config incl. the provider API key,
`ccr.db`). The runtime home is now ONE decision shared by both halves, made
identically by the shim and the Rust binary, with adopt-and-warn legacy
handling for upgrades and full scratch-home isolation for the catalog
validate probe. Binary `1.6.2 → 1.6.3`, plugin `2.2.2 → 2.2.3`.

### Changes

- **Fix (issue #40, runtime home)**: the shim resolves the runtime home once
  at import (`$APHRODITE_HOME` override → `<hermes-home>/aphrodite` →
  `~/.hermes/aphrodite`) and exports it to the Rust half through
  `APHRODITE_HOME` + `APHRODITE_DIRECTIVES_DIR` (env setdefault), so the
  dylib and the spawned proxy binary agree by construction; per-Hermes-home
  isolation (profiles) now extends to the Rust half. The startup log names
  the decision: `runtime home: <path> (decided by APHRODITE_HOME
override|HERMES_HOME|legacy adoption|default)`.
- **Fix (issue #40, upgrades)**: when the Hermes-home-derived home holds no
  install but the pre-2.2 `~/.hermes/aphrodite` does, the legacy home is
  ADOPTED with a one-line warning (never migrated), so upgrades keep finding
  `binaries/` and `aphrodite.toml`. Adoption never fires for throwaway
  homes (catalog-validate probe scratch, `~/.hermes/cache/scratch`,
  `/tmp/hermes-*`), preserving the probe's isolation from the real install.
- **Fix (issue #40, layout heal)**: `layout_check.py` follows the same
  single decision (`APHRODITE_HOME` when set), so required dirs are never
  (re)created under a second, shadow home; a relative `APHRODITE_HOME` now
  warns.
- **Fix (issue #40, Rust side)**: new shared `home` module
  (`crates/aphrodite/src/home.rs`) - `runtime_home()`, `config_path()`,
  `directives_dir()`, `ccr_db_path()` - consumed by the engine binary
  (config fallback, ccr.db, config watcher), `aphrodite setup` (bootstraps
  the Hermes home the plugin will use), and the dylib (directives
  provisioning, debug flags); `$HERMES_HOME` is honored with the same
  precedence as the shim. The unused `dirs` dependency is dropped from
  `aphrodite-hermes`.
- **Hardening**: the shim import can never raise on a hostile environment
  (degraded fallback + warning); empty env overrides fall through to the
  next precedence level; tilde expansion on Rust env paths; the layout heal
  never crashes on symlink loops.
- **Tests**: new `tests/test_runtime_home.py` (8 subprocess cases: default,
  HERMES_HOME, explicit override, legacy adoption, both-installs,
  temp/scratch homes, hostile env); `tests/test_layout_check.py` +1
  APHRODITE_HOME-relocation case; Rust `home::tests` 7 cases (precedence
  matrix, tilde, empty values, degraded); directives precedence tests
  updated for the HERMES_HOME step.
- **Docs**: plugin README gains the "Runtime home (one decision, shared by
  both halves)" section incl. the ≤ 2.1.5 upgrade note;
  `docs/config/env-vars.md` `APHRODITE_HOME` / `HERMES_HOME` rows corrected
  (they previously documented the two-half divergence as behavior);
  `docs/architecture/10-component.md` boundary note updated. Fixes #40.

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅
  (aphrodite 1.6.3, aphrodite-hermes 1.6.3)
- Tests: `cargo test -p aphrodite` ✅ 434 passed (405 lib + 29 bins /
  integration), 0 failed, 1 ignored
- Tests: `cargo test -p aphrodite-hermes` ✅ 56 passed, 0 failed
- Python: `tests/test_runtime_home.py` ✅ 8/8; `tests/test_layout_check.py`
  ✅ 15/15; `test_dylib_candidates.py` ✅; `ruff check plugins/aphrodite/` ✅
- Lint: `cargo clippy -p aphrodite -p aphrodite-hermes --lib -D warnings` ✅
  (only pre-existing manifest notes)
- Format: `cargo +nightly-2026-05-01 fmt --all -- --check` ✅ (the pinned
  canonical toolchain); `npx prettier --check .hermes/release-notes/*.md` ✅
  (the repo-wide `.hermes/**/*.md` gate additionally reports 13
  pre-existing violations in `.hermes/skills/` - concurrent work, untouched)
- FFI: build.rs "header unchanged" - committed `_bindings.py` matches the
  ABI (no C-surface drift)
- Install: `aphrodite setup` from the fresh release binary ✅ -
  `~/.hermes/aphrodite` (binary 1.6.3, dylib, `BINARY_VERSION` 1.6.3) and
  `~/.hermes/plugins/aphrodite` (loader, manifest) verified; end-to-end
  smoke: the built binary loads its config from `$HERMES_HOME/aphrodite`
  when `HERMES_HOME != $HOME/.hermes` (Docker shape)

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.3                                                        | Hermes (standalone repo)                 |

`aphrodite-headroom-core` stays at 0.1.3 (published with 1.5.1; the fork
holds 2 dependency-update commits post-0.1.3 - fastembed→5, crate version
refresh - carried in the fork's RELEASE-CYCLE ledger, no version bump in
this release).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.2...Aphrodite/1.6.3
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
