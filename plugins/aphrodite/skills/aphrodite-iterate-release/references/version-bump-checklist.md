# Version Bump Checklist

Every release must update ALL 6 locations before building + tagging. Missing any causes download 404s or version mismatches.

## Locations

| # | File | Key | Example |
|---|------|-----|---------|
| 1 | `plugins/aphrodite/__init__.py` | `BIN_VERSION` | `"v0.5.10"` |
| 2 | `plugins/aphrodite/__init__.py` | `PLUGIN_VERSION` | `"1.19.0"` |
| 3 | `plugins/aphrodite/__init__.py` | Docstring line 1 | `aphrodite v1.19.0` |
| 4 | `plugins/aphrodite/plugin.yaml` | `version:` field | `1.19.0` |
| 5 | `plugins/aphrodite/plugin.yaml` | `install_message:` version string | `v1.19.0` |
| 6 | `crates/aphrodite/Cargo.toml` | `version =` | `"0.5.10"` |

## Version Convention

- `BIN_VERSION` and `Cargo.toml version` track the Rust binary: `0.5.X`
- `PLUGIN_VERSION` and `plugin.yaml version` track the Python plugin: `1.X.0`
- Both increment independently — binary version patches, plugin version minors
- Always bump on every change — no skipped versions

## Auto-Increment Pattern

After each release, the next patch version auto-increments:
- Binary: `v0.5.9` → `v0.5.10`
- Plugin: `1.18.0` → `1.19.0`

Use `git tag` to find the last release: `git tag --sort=-v:refname | head -1`

## Release Commands (Quick Reference)

```bash
# Build
cargo build --release -p aphrodite

# Copy binary
cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
chmod 755 ~/.hermes/aphrodite/aphrodite

# Tag and push
git tag -f -m "vX.Y.Z: summary" vX.Y.Z
git push aphrodite vX.Y.Z --force

# Create GitHub release (delete existing first if forced tag)
gh release delete vX.Y.Z --repo PlayForm/Aphrodite --yes
gh release create vX.Y.Z \
  --repo PlayForm/Aphrodite \
  --title "vX.Y.Z - summary" \
  --notes "detailed notes" \
  ~/.hermes/aphrodite/aphrodite
```

## Pitfalls

- Release binary must be copied BEFORE tag push (otherwise release has stale binary)
- `gh release create` fails if tag already has a release — delete first
- `git tag -f` without `-m` fails on annotated tags — always include `-m "message"`
- The Rust binary embeds version at compile time — must rebuild after bumping Cargo.toml
