---
name: aphrodite-release-workflow
description: "Auto-release, version sync, pre-release checklist, and release notes for aphrodite."
version: 2.0.0
platforms: [macos]
---

# Aphrodite Release Workflow

## Pre-Release Checklist

**MANDATORY before every release.** Run in order. Any failure blocks the release.

### 1. Dependency Pinning
All deps must use exact versions, not ranges. Dependabot or manual bumps only.

```bash
grep -c '>=' plugins/aphrodite/pyproject.toml  # must be 0
cargo check -p aphrodite  # catches Cargo range deps
```

### 2. Skills Separation
Dev skills in monorepo `skills/`, user skills in `plugins/aphrodite/skills/`.

**User (9, shipped):** tool-guide, output-formatting, presentation, proxy-lifecycle, context-efficiency, compression-architecture, context-engine-defaults, boundary-behaviors, coding-defaults
**Dev (9, monorepo):** benchmarking, hook-reference, release-workflow, development-lessons, cargo-upgrade, auto-expand-testing, upgrade-breakpoints, operations, version-patterns

```bash
ls plugins/aphrodite/skills/ | grep -E 'benchmarking|hook-reference|release-workflow|development' && echo "LEAK" || echo "clean"
```

### 3. --version Flag
Binary must respond instantly. `aphrodite.toml` existence skips clap parsing — handle `-V`/`--version` at top of `main()` before config loading.

```bash
target/release/aphrodite --version  # must print instantly, never hang
```

### 4. Nested .git
`plugins/aphrodite/` must NOT contain `.git`.

```bash
test -d plugins/aphrodite/.git && echo "REMOVE NESTED .git" || echo "clean"
```

### 5. Standalone Repo Sync
After bumping versions, sync to `PlayForm/Aphrodite-Hermes`:

```bash
# Copy updated plugin files (excluding binary, pycache)
# Commit + push to standalone repo
```

### 6. Import + Lint
```bash
python3 -c "import sys; sys.path.insert(0, 'plugins'); import aphrodite; print('OK:', aphrodite.__doc__[:60])"
ruff check plugins/aphrodite/
cargo check -p aphrodite
```

If any step fails, fix BEFORE releasing. A broken plugin means zero tools, no compression, no context engine — a silent degradation.

## Auto-Release

```bash
GIT_EDITOR=true scripts/auto-release.sh "descriptive message"
```
Handles: stage → commit → bump version → cargo build → cargo test → tag → push to `Source` remote.

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

Never use backticks with `gh release create --notes`. Use `--notes-file` with a heredoc:

```bash
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**
### Feature
- change
EOF
gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md ~/.hermes/aphrodite/aphrodite
```

## Cross-Module Import Pitfall

Adding a new import in one module (e.g., `from .live import _is_live_tool`) without defining the symbol in the target module silently kills the plugin at session start. Always test the full import chain before releasing: `python3 -c "import aphrodite"`.
