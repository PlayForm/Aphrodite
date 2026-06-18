---
name: aphrodite-release-workflow
description:
    "Auto-release, version sync, pre-release verification, and release notes for
    aphrodite."
version: 1.2.0
platforms: [macos]
---

# Aphrodite Release Workflow

## Pre-Release Verification

**MANDATORY before every release.** Missing symbols silently kill the plugin —
`Failed to load plugin` with no error.

```bash
cd /path/to/Aphrodite
python3 -c "import sys; sys.path.insert(0, 'plugins'); import aphrodite; print('OK:', aphrodite.__doc__[:60])"
ruff check plugins/aphrodite/ scripts/ crates/
npx pyright plugins/aphrodite/
cargo check -p aphrodite
```

If any step fails, fix BEFORE releasing. A broken plugin means zero tools, no
compression, no context engine — a silent degradation users won't notice.

## Auto-Release

```bash
GIT_EDITOR=true scripts/auto-release.sh "descriptive message"
```

Handles: stage → commit → bump version → cargo build → cargo test → tag → push
to `Source` remote.

## Version Sync

5 locations must match — auto-release.sh bumps all via `sed`:

1. `plugins/aphrodite/_core/config.py` — `BIN_VERSION` + `PLUGIN_VERSION`
2. `plugins/aphrodite/plugin.yaml` — `version:`
3. `plugins/aphrodite/pyproject.toml` — `version =`
4. `plugins/aphrodite/__init__.py` — docstring `aphrodite vX.Y.Z`
5. `crates/aphrodite/Cargo.toml` — `version =`

## Binary Symlink

```bash
ln -sf /path/to/repo/target/release/aphrodite ~/.hermes/aphrodite/aphrodite
```

## Release Notes — Shell Injection

Never use backticks with `gh release create --notes`. Use `--notes-file` with a
heredoc:

```bash
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**
### Feature
- change
EOF
gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md ~/.hermes/aphrodite/aphrodite
```

## Cross-Module Import Pitfall

Adding a new import in one module (e.g., `from .live import _is_live_tool`)
without defining the symbol in the target module silently kills the plugin at
session start. Always test the full import chain before releasing:
`python3 -c "import aphrodite"`.
