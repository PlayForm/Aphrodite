**[Compare Aphrodite/v1.6.0...Aphrodite/1.6.1](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.0...Aphrodite/1.6.1)**

## Aphrodite 1.6.1 💋 Plugin v2.2.1

### Summary

1.6.1 is a patch release pairing the plugin's catalog-cleanup with the
binary version track. The plugin manifest now declares exactly the 13 tools
the production dylib registers - the dev-only `aphrodite_debug` tool was
removed from `provides_tools` (it is `debug_assertions`-gated out of release
builds), which closes the catalog-review rule-6 mismatch on the pinned tree.
The plugin moves to v2.2.1 with `BINARY_VERSION` 1.6.1, and the version
track (crates + package.json) follows to 1.6.1.

### Changes

- **Chore (plugin manifest)**: `provides_tools` declares the 13 production
  tools; `aphrodite_debug` (dev-only, compiled out of release dylibs)
  removed from the manifest and the install message.
- **Chore (version tracks)**: plugin v2.2.1, `BINARY_VERSION` 1.6.1; binary
  crates + `package.json` moved 1.6.0 -> 1.6.1 together.
- **Chore (release prep)**: README badges moved to 1.6.1 / v2.2.1; pinned
  nightly-2026-05-01 rustfmt canonicalization carried from 1.6.0.

### Infrastructure

- Tests: `cargo test -p aphrodite --lib` ✅ (398 passed, 0 failed, 1
  ignored)
- Tests: `cargo test -p aphrodite-hermes --lib` ✅ (55 passed)
- Lint: `cargo +nightly-2026-05-01 fmt --all -- --check` ✅
- FFI: ABI unchanged since 1.6.0 (committed `_bindings.py` matches)

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.1                                                        | Hermes (standalone repo)                 |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.0...Aphrodite/1.6.1
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **crates.io**: https://crates.io/crates/aphrodite
