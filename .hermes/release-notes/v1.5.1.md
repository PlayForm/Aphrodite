> **Draft** - planned 1.5.1 binary release note; `{VERSION}` / `{PLUGIN_VERSION}`
> placeholders mark values to fill at release time.

**[Compare Aphrodite/v1.5.0...Aphrodite/{VERSION}](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.0...Aphrodite/{VERSION})**

## Aphrodite 1.5.1 💋 Plugin v{PLUGIN_VERSION}

### Summary

1.5.1 is the headroom publication release: the fork's accumulated changes -
upstream sync 43dc9836, the nightly-toolchain migration, and the
packaging/deps fixes - finally reach crates.io as `aphrodite-headroom-core`
0.1.3, replacing the stale 0.1.2 (July 2026) that external consumers of
1.5.0 still resolved against. The parent pin in crates/aphrodite moves to
0.1.3 and the fork gets tag `aphrodite-v0.10.0`.

### Changes

- **Chore (headroom publication)**: `aphrodite-headroom-core` 0.1.3
  published to crates.io, carrying the fork's accumulated changes since the
  July 0.1.2 bump: upstream merge 43dc9836 (headroomlabs-ai/headroom@main,
  396 behind / 31 ahead), `ml` feature defaulted OFF (d9f12039),
  package-name references fixed to `aphrodite-headroom-core` (39ae7079),
  ml-cluster dep pin (84c8d117), vendored submodule removed (1f80236c), and
  the nightly-toolchain syntax batch (5fda221a..02706ea1).
- **Chore (deps)**: crates/aphrodite's `aphrodite-headroom-core` pin moves
  0.1.2 -> 0.1.3.
- **Chore (fork tag)**: PlayForm/Headroom tagged `aphrodite-v0.10.0` at
  02706ea1.

### Verification

- Fork commit range analyzed: `c6b61470..02706ea1` (107 commits) inside
  vendor/headroom - the content 0.1.3 publishes.
- Diffstat: `git diff --stat c6b61470..02706ea1` = 1021 files changed,
  +136220/-45505 (upstream-sync dominated).
- Binary-side diff (pin bump + tag) lands at release time; live
  Infrastructure checks to be recorded in the final note.

### What Ships

| Artifact                             | Platform                 |
| ------------------------------------ | ------------------------ |
| `aphrodite-aarch64-apple-darwin`     | macOS ARM64              |
| `aphrodite-x86_64-apple-darwin`      | macOS Intel              |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64             |
| `aphrodite-x86_64-pc-windows-msvc`   | Windows x86_64           |
| Plugin v{PLUGIN_VERSION}             | Hermes (standalone repo) |

Plus `aphrodite-headroom-core` 0.1.3 on crates.io (new in 1.5.1).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.5.0...Aphrodite/{VERSION}
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core
