---
name: aphrodite-release-workflow
description: "Auto-release, version sync, pre-release verification, and release notes for
    aphrodite."
version: 1.3.0
platforms: [macos]
---

# Aphrodite Release Workflow

## Pre-Release Verification

**MANDATORY before every release.** Missing symbols silently kill the plugin -
`Failed to load plugin` with no error.

```bash
cd /path/to/Aphrodite
python3 -c "import sys; sys.path.insert(0, 'plugins'); import aphrodite; print('OK:', aphrodite.__doc__[:60])"
ruff check plugins/aphrodite/ Maintain/scripts/ crates/
npx pyright plugins/aphrodite/
cargo check -p aphrodite
```

If any step fails, fix BEFORE releasing. A broken plugin means zero tools, no
compression, no context engine - a silent degradation users won't notice.

## Auto-Release

```bash
GIT_EDITOR=true Maintain/scripts/release/auto-release.sh "descriptive message"
```

Handles: stage → commit → bump version → cargo build → cargo test → tag → push
to `Source` remote.

## Version Sync

5 locations must match - auto-release.sh bumps all via `sed`:

1. `plugins/aphrodite/_core/config.py` - `BIN_VERSION` + `PLUGIN_VERSION`
2. `plugins/aphrodite/plugin.yaml` - `version:`
3. `plugins/aphrodite/pyproject.toml` - `version =`
4. `plugins/aphrodite/__init__.py` - docstring `aphrodite vX.Y.Z`
5. `crates/aphrodite/Cargo.toml` - `version =`

## Binary Symlink

```bash
ln -sf /path/to/repo/target/release/aphrodite ~/.hermes/aphrodite/aphrodite
```

## Release Notes — Content Standards

Every release MUST include: Summary, Changes, Infrastructure, What Ships, and Links.
See `.hermes/RELEASE-TEMPLATE.md` for the canonical template.

Anti-pattern (DO NOT): bare compare link with zero description.
30+ releases (v0.8.13–v0.8.43) currently ship with only:

```
**Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/...
```

### Shell Injection

Never use backticks with `gh release create --notes`. Use `--notes-file` with a
heredoc:

```bash
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**

## Aphrodite vX.Y.Z 💋 Plugin vA.B.C

### Summary
One paragraph. What this release is. 2–3 sentences.

### Changes
- **Feature**: description
- **Fix**: description

### Infrastructure
- Build: `cargo build --release -p aphrodite` ✅
- Tests: `cargo test -p aphrodite` ✅ (NNN passed)
- Python: `ruff check` + `pyright` ✅
- Lint: `cargo clippy` ✅

### What Ships
| Artifact | Platform |
|----------|----------|
| `aphrodite-aarch64-apple-darwin` | macOS ARM64 |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64 |
| Plugin vA.B.C | Hermes (standalone repo) |

### Links
- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/...
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
EOF
gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md \
  target/release/aphrodite-aarch64-apple-darwin \
  target/release/aphrodite-x86_64-unknown-linux-gnu
```

## Cross-Module Import Pitfall

Adding a new import in one module (e.g., `from .live import _is_live_tool`)
without defining the symbol in the target module silently kills the plugin at
session start. Always test the full import chain before releasing:
`python3 -c "import aphrodite"`.
