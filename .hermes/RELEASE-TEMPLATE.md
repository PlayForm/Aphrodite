# Release Template

Replace `{BIN_VERSION}` and `{PLUGIN_VERSION}` with actual values.
Replace `{PREV_VERSION}` with the prior release tag.

---

**[Compare {PREV_VERSION}...{BIN_VERSION}](https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...Aphrodite/{BIN_VERSION})**

## Aphrodite {BIN_VERSION} 💋 Plugin v{PLUGIN_VERSION}

### Summary

One paragraph. What this release is. Why it matters. 2–3 sentences max.

### Changes

- **Feature**: description
- **Fix**: description
- **Chore**: description
- **Docs**: description

### Infrastructure

- Build: `cargo build --release -p aphrodite` ✅
- Tests: `cargo test -p aphrodite` ✅ (NNN passed)
- Python tests: `ruff check` + `pyright` ✅
- Lint: `cargo clippy` ✅
- Submodules: synced to latest commits

### What Ships

| Artifact | Platform |
|----------|----------|
| `aphrodite-aarch64-apple-darwin` | macOS ARM64 |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64 |
| Plugin v{PLUGIN_VERSION} | Hermes (standalone repo) |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...Aphrodite/{BIN_VERSION}
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom

---

## Section Rules

1. **Summary**: Always present. 1–3 sentences. Even a one-liner is better than empty.
2. **Changes**: Bullet list per type (Feature/Fix/Chore/Docs). Omit empty categories.
3. **Infrastructure**: Always present. Shows test counts, build status, lint results.
4. **What Ships**: Always present. Lists every binary asset + plugin version.
5. **Links**: Always present. Minimum: compare link + CHANGELOG. Include plugin + fork links for major releases.

## Anti-Pattern (DO NOT)

```
**Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v0.8.42...Aphrodite/v0.8.43
```

This is the current state of 30+ releases. A bare compare link with zero description tells users nothing about what changed.
