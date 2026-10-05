**[Compare Aphrodite/v1.6.1...Aphrodite/1.6.2](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.1...Aphrodite/1.6.2)**

## Aphrodite 1.6.2 💋 Plugin v2.2.2

### Summary

1.6.2 is a version-track release pairing plugin v2.2.2 with binary 1.6.2.
Its one functional fix: the release workflows on Current regain the in-tree
`SHA256SUMS.txt` generation (Build.yml Finalize, committing inside the child
submodule) and the child-gitlink advance (Publish.yml) that were lost when
Current's workflows were identity-restored during the 1.6.0 sync. From this
release on, the pinned plugin tree's `SHA256SUMS.txt` is regenerated and
committed by the release procedure itself, matching the file header's
documented contract.

### Changes

- **Fix (release infra)**: Current's `Build.yml` Finalize job now regenerates
  `plugins/aphrodite/SHA256SUMS.txt` (`BINARY_VERSION: 1.6.2` plus the four
  per-target blocks from the release assets) and commits it inside the child
  submodule on its Current branch; `Publish.yml` then advances the parent
  gitlink to that commit. The release line no longer ships a stale
  `SHA256SUMS.txt` (the mechanism was lost in the 1.6.0 identity-restore).
- **Chore (version tracks)**: plugin v2.2.2, `BINARY_VERSION` 1.6.2; binary
  crates + `package.json` moved 1.6.1 -> 1.6.2 together.
- **Chore (release prep)**: README badges moved to 1.6.2 / v2.2.2; release
  notes authored per `.hermes/release/RELEASE-TEMPLATE.md`.

### Infrastructure

- Version track verified: `crates/aphrodite` + `crates/aphrodite-hermes`
  `1.6.2` (including the `../aphrodite` pin), `package.json` 1.6.2,
  `plugin.yaml` v2.2.2, `BINARY_VERSION` 1.6.2, README badges 1.6.2 / v2.2.2.
- No Rust code in the 1.6.1 -> 1.6.2 range; no test suite re-run.
- Workflows on the release line verified to carry the child-submodule steps
  (Build.yml: generate + commit in-tree sums; Publish.yml: advance gitlink).

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.2                                                        | Hermes (standalone repo)                 |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.1...Aphrodite/1.6.2
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **crates.io**: https://crates.io/crates/aphrodite
