> **Draft** - next binary release note. `{VERSION}` / `{PLUGIN_VERSION}` stay
> as placeholders until a version sibling cuts the bump; `{PENDING}` marks a
> value that only exists after the release task runs. The previous cycle's note
> is final in [`release-notes-v1.8.0.md`](release-notes-v1.8.0.md) - this file
> tracks what comes next and is intentionally empty.

**[Compare Aphrodite/v1.8.0...Aphrodite/{VERSION}](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/{VERSION})**

## Aphrodite {VERSION} 💋 Plugin v{PLUGIN_VERSION}

### Summary

{To be written when the cycle's scope is set.}

### Changes

- {No entries yet.}

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ({PENDING})
- Gates: `bash Maintain/Check.sh` ({PENDING}), `cargo ci-test` ({PENDING})
- Version track: crates + pin + `package.json` {VERSION}; `plugin.yaml`
  {PLUGIN_VERSION}; `BINARY_VERSION` bumped LAST, after the release assets
  exist; README badges v{VERSION} / v{PLUGIN_VERSION}

### What Ships

`Build.yml`'s 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/{VERSION}
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
