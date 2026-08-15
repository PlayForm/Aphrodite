---
name: aphrodite-release-workflow
description: "Auto-release, version sync, pre-release verification, and release notes for
    aphrodite."
version: 1.4.0
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

### crates.io publishing goes through Publish.yml, never manually

Do NOT run `cargo publish` locally. `.github/workflows/Publish.yml` builds and
publishes `aphrodite-headroom` → `aphrodite` → `aphrodite-hermes` (in that
dependency order) - but only on an explicit `workflow_dispatch` with
`publish_crates: true`; a plain `Aphrodite/v*` tag push alone does NOT publish
to crates.io (it only triggers `Build.yml`'s GitHub Release artifacts). This
gate exists because crates.io versions are immutable - a version number burned
by a bad manual `cargo publish` can never be reused (see the v1.3.1 incident in
`Maintain/CHANGELOG.md`'s "Why 1.3.2, not 1.3.1" note). Trigger it deliberately
via `gh workflow run Publish.yml -f publish_crates=true` (or the Actions UI)
only once the tagged release's artifacts are verified.

## Version Sync

The Aphrodite project has two independent version tracks:

- **Binary version** (`1.3.8` as of this writing) - Rust crates, must match across Cargo.toml files
- **Plugin version** (`2.0.10` as of this writing) - Hermes plugin, lives in the `plugins/aphrodite` submodule

Treat the numbers below as *shape*, not as current truth - read the live value
from `crates/aphrodite/Cargo.toml` (binary) and `plugins/aphrodite/plugin.yaml`
(plugin) before bumping. `auto-release.sh` reads both itself and its seds are
version-pattern-based precisely so a stale number in this document can never
misdirect a release.

**Binary version locations** (monorepo - bump these together):

1. `crates/aphrodite/Cargo.toml` - `version = "<bin>"` (line 3)
2. `crates/aphrodite-hermes/Cargo.toml` - `version = "<bin>"` (line 3, package)
3. `crates/aphrodite-hermes/Cargo.toml` - `aphrodite = { ..., version = "<bin>" }` (dependency)
4. `plugins/aphrodite/BINARY_VERSION` - plain-text `<bin>`, read by `download.sh`
5. `package.json` - `"version": "<bin>"` (line 3)

**Plugin version locations** (submodule `plugins/aphrodite/`):

6. `plugin.yaml` - `version: <plugin>` (line 2)
7. `plugin.yaml` - `install_message:` block contains `aphrodite v<plugin> -`
8. `pyproject.toml` - `version = "<plugin>"` (if file exists)
9. `__init__.py` - docstring `aphrodite v<plugin> - ...` (if version appears there)
10. `_core/config.py` - `BIN_VERSION` + `PLUGIN_VERSION` constants (if file exists)

**Documentation:**

11. `README.md` - release badge `release-v<bin>-blue`, plugin badge
    `plugin-v<plugin>-purple`, and the example health output `"version":"v<bin>"`

**What `auto-release.sh` bumps** (complete list):

- `crates/aphrodite/Cargo.toml` - binary version ✅
- `crates/aphrodite-hermes/Cargo.toml` - package version + aphrodite dependency ✅
- `plugins/aphrodite/plugin.yaml` - version field + install_message ✅
- `plugins/aphrodite/pyproject.toml` - version (if file exists) ✅
- `plugins/aphrodite/__init__.py` - docstring version (if present) ✅
- `plugins/aphrodite/_core/config.py` - BIN_VERSION + PLUGIN_VERSION (if file exists) ✅

## Submodule Release Flow

`plugins/aphrodite` is a git submodule → separate repo `PlayForm/Aphrodite-Hermes`.
`auto-release.sh` handles the full cross-repo release:

1. **Bump** `plugin.yaml` (and other submodule files) via `sed`
2. **Commit** in submodule: `cd plugins/aphrodite && git commit -m "release: plugin vX.Y.Z"`
3. **Tag** in submodule: `git tag "vX.Y.Z"`
4. **Push** submodule branch + tag to `$SUBMODULE_REMOTE` (default: `Source`)
5. **Update parent pointer**: `git update-index --cacheinfo` to track new submodule SHA
   (needed because `.gitmodules` has `ignore = all`, which hides dirty submodules)
6. **Commit** parent pointer update: `chore: sync aphrodite submodule → plugin vX.Y.Z`

**Manually verify after release:**

- `README.md:~258` - example output version (non-critical, grep for `"version":"v`)

## Binary Symlink

```bash
ln -sf /path/to/repo/target/release/aphrodite ~/.hermes/aphrodite/aphrodite
```

## Release Notes - Content Standards

Every release MUST include: Summary, Changes, Infrastructure, What Ships, and Links.
See `.hermes/RELEASE-TEMPLATE.md` for the canonical template.

Anti-pattern (DO NOT): bare compare link with zero description.
30+ releases (v0.8.13-v0.8.43) currently ship with only:

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
One paragraph. What this release is. 2-3 sentences.

### Changes
- **Feature**: description
- **Fix**: description

### Infrastructure
- Build: `cargo build --release -p aphrodite` ✅
- Tests: `cargo test -p aphrodite` ✅ (NNN passed)
- Python: `ruff check` + `pyright` ✅
- Lint: `cargo clippy` ✅

### What Ships
Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
- 4 targets × 3 files = 12 artifacts total on a full release.

| Artifact pattern | Platform |
|----------|----------|
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target above |
| `SHA256SUMS-<target>.txt` | checksums for that target's two binaries |
| Plugin vA.B.C | Hermes (standalone repo) |

### Links
- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/...
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
EOF
# In normal operation Build.yml's Release job attaches every staged artifact
# automatically on tag push - this manual form is only for a from-source
# re-attach. Glob every staged file rather than naming 2 of the 12:
gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md \
  staging/aphrodite-* staging/libaphrodite_hermes-* staging/SHA256SUMS-*.txt
```

## Cross-Module Import Pitfall

Adding a new import in one module (e.g., `from .live import _is_live_tool`)
without defining the symbol in the target module silently kills the plugin at
session start. Always test the full import chain before releasing:
`python3 -c "import aphrodite"`.
