**[Compare Aphrodite/v1.5.0...Aphrodite/v1.5.1](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.0...Aphrodite/v1.5.1)**

## Aphrodite 1.5.1 💋 Plugin v2.1.5

### Summary

1.5.1 is the headroom publication release: the fork's accumulated changes -
upstream sync `43dc9836`, the nightly-toolchain migration, and the
packaging/deps fixes - finally reach crates.io as `aphrodite-headroom-core`
0.1.3, replacing the stale 0.1.2 (July 2026) that external consumers of 1.5.0
still resolved against. The parent pin in `crates/aphrodite` moves to 0.1.3
and the fork gets tag `aphrodite-v0.10.0`.

### Changes

- **Chore (headroom publication)**: `aphrodite-headroom-core` 0.1.3 published
  to crates.io, carrying the fork's accumulated changes since the July 0.1.2
  bump: upstream merge `43dc9836` (headroomlabs-ai/headroom@main, 396 behind
  / 31 ahead), `ml` feature defaulted OFF (`d9f12039`), package-name
  references fixed to `aphrodite-headroom-core` (`39ae7079`), ml-cluster dep
  pin (`84c8d117`), vendored submodule removed (`1f80236c`), and the
  nightly-toolchain syntax batch (`5fda221a..02706ea1`).
- **Chore (deps)**: `crates/aphrodite`'s `aphrodite-headroom-core` pin moves
  0.1.2 -> 0.1.3.
- **Chore (fork tag)**: PlayForm/Headroom tagged `aphrodite-v0.10.0` at the
  0.1.3 release commit (`0ea0f27d`).
- **Chore (repo)**: `.hermes/ export-ignore` dropped from `.gitattributes`
  (archive layout alignment with the sync-back).

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ (35.45s;
  aphrodite 1.5.1, aphrodite-hermes 1.5.1, aphrodite-headroom-core 0.1.3)
- Tests: `cargo test -p aphrodite` ✅ 422 passed, 0 failed, 1 ignored (393
  lib + 29 bins)
- Tests: `cargo test -p aphrodite-hermes` ✅ 52 passed, 0 failed
- Fork tests: `cargo test -p aphrodite-headroom-core` ✅ 5 passed, 0 failed
- Python: `test_finalize_bindings.py` 23/23 OK; `test_check_ffi_contract.py`
  14/14 (55 asserts); `check_ffi_contract.py` 0 violations; `ruff check` ✅;
  `pyright` 0 errors
- Lint: `cargo clippy -p aphrodite -- -D warnings` ✅; `prettier --check
  .hermes/**/*.md` ✅
- Runtime: `aphrodite_rebuild` -> dylib 1.5.1 loaded; `aphrodite_test` full
  -> 3/3 compress/retrieve round-trips byte-identical

### What Ships

| Artifact                             | Platform                 |
| ------------------------------------ | ------------------------ |
| `aphrodite-aarch64-apple-darwin`     | macOS ARM64              |
| `aphrodite-x86_64-apple-darwin`      | macOS Intel              |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64             |
| `aphrodite-x86_64-pc-windows-msvc`   | Windows x86_64           |
| Plugin v2.1.5                        | Hermes (standalone repo) |

Plus `aphrodite-headroom-core` 0.1.3 on crates.io (new in 1.5.1).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.0...Aphrodite/v1.5.1
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core