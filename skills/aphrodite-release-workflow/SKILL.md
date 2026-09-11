---
name: aphrodite-release-workflow
description: Use when releasing Aphrodite. Pre-release verification gates,
    auto-release, version sync, submodule flow, release notes, commit/sync.
version: 1.5.0
platforms: [macos]
tags: [aphrodite, release, cargo, crates-io, submodule, release-notes]
---

# Aphrodite Release Workflow

## When to Use

- Cutting a new Aphrodite binary or plugin release, or verifying one.
- Publishing crates to crates.io, syncing the standalone plugin repo, or
  writing release notes.

## Pre-Release Verification (mandatory — four gates)

Missing symbols silently kill the plugin — `Failed to load plugin` with no
error. The release ALSO fails in CI on four gates that a local `cargo build`
does not catch. Run all of them before tagging; every one has burned a release:

```bash
cd /path/to/Aphrodite
python3 -c "import sys; sys.path.insert(0, 'plugins'); import aphrodite; print('OK:', aphrodite.__doc__[:60])"
ruff check plugins/aphrodite/ Maintain/scripts/ crates/
npx pyright plugins/aphrodite/
cargo check -p aphrodite
cargo clippy -p aphrodite --lib -- -D warnings   # GATE 1
cargo audit                                     # GATE 3 (or: cargo deny check advisories)
```

1. **GATE 1 — clippy `-D warnings`**: CI compiles with `-D warnings`; a green
   local build still fails release. Recurring trap: `clippy::useless_conversion`
   on a redundant `.into_iter()` — remove the `.into_iter()`.
2. **GATE 2 — ruff on `plugins/aphrodite/`**: AGENTS.md requires 0 errors.
   Recurring violations: `SIM105` (`try/except/pass` → `contextlib.suppress`)
   and `F401` unused imports. Fix in plugin source + tests before releasing.
3. **GATE 3 — cargo audit / unmaintained advisory**: `Check.yml` fails on
   advisories. `cgmath` (unmaintained) enters only through the experimental
   `s2` crates (`crates/s2-probe`, `crates/s2-navigate`) which are NOT shipped.
   Keep `s2` an `optional` dep behind the `navigation` feature (off by default)
   AND excluded from `workspace.members`. Verify with
   `cargo tree -p aphrodite -i cgmath` — must return nothing.
4. **GATE 4 — version discipline**: never re-tag a released version to fix it;
   cut a new version.

If any gate fails, fix BEFORE releasing. A broken plugin means zero tools, no
compression, no context engine.

## Auto-Release

```bash
GIT_EDITOR=true Maintain/scripts/release/auto-release.sh "descriptive message"
```

Handles: stage → commit → bump version → cargo build → cargo test → tag → push
to the `Source` remote.

## crates.io Publishing (Publish.yml only)

Never run `cargo publish` locally — crates.io versions are immutable; a version
burned by a bad publish can never be reused. `.github/workflows/Publish.yml`
publishes `aphrodite-headroom-core` → `aphrodite` → `aphrodite-hermes` in
dependency order, but ONLY on `workflow_dispatch` with `publish_crates: true`;
a plain `Aphrodite/v*` tag push triggers only Build.yml's GitHub Release
artifacts, not crates.io. Trigger deliberately once the tagged artifacts are
verified:

```bash
gh workflow run Publish -f publish_crates=true
```

Dispatch-name note: the file is `Publish.yml` but it dispatches as `Publish`
(no extension). `cargo publish` needs `CARGO_REGISTRY_TOKEN` (a CI secret) —
never run it locally without that token.

### Headroom core crate — the gitlink trap

`vendor/headroom` is a git submodule (`PlayForm/Headroom.git`, branch
`Current`). Its publishable crate is `crates/headroom-core/Cargo.toml`,
published under the package name `aphrodite-headroom-core` (sibling crates
reference it via `package = "aphrodite-headroom-core"`). Publish.yml's
`Publish-Headroom-Core` job runs `working-directory: vendor/headroom` and is a
hard `needs:` prerequisite for `Publish-Aphrodite`.

Never assume CI publishes the locally checked-out submodule HEAD — CI checks
out the RECORDED GITLINK commit in the parent repo. After renaming/reverting/
bumping the headroom crate, always update and push the parent gitlink so CI
publishes the new state: `git add vendor/headroom && git commit -m "..." &&
git push Source Current`. Otherwise CI publishes the previously recorded
commit's tree.

Verify before triggering (read-only): check the crates.io index —
`https://index.crates.io/ap/hr/aphrodite-headroom-core` (path = first 2 / next
2 chars of the crate name). `404` = not published (CI will attempt it); `200`
containing `"vers":"X.Y.Z"` = that version is live (CI skips it). Never
re-publish an immutable version.

See `references/headroom-publish.md` for the rename procedure and checklist.

## Version Sync (two independent tracks)

- **Binary version** — Rust crates; must match across Cargo.toml files.
- **Plugin version** — Hermes plugin, lives in the `plugins/aphrodite`
  submodule.

Never trust a stale number in this document — read the live value from
`crates/aphrodite/Cargo.toml` (binary) and `plugins/aphrodite/plugin.yaml`
(plugin) before bumping. `auto-release.sh` reads both itself via
version-pattern seds, so a stale doc number can never misdirect a release.

**Binary version locations** (monorepo — bump together):

1. `crates/aphrodite/Cargo.toml` — `version`
2. `crates/aphrodite-hermes/Cargo.toml` — package `version` + the
   `aphrodite = { ..., version }` dependency
3. `plugins/aphrodite/BINARY_VERSION` — plain text, read by `download.sh`
4. `package.json` — `"version"`

**Plugin version locations** (submodule `plugins/aphrodite/`):

5. `plugin.yaml` — `version` + the `install_message` block
6. `pyproject.toml` — `version` (if the file exists)
7. `__init__.py` — docstring version (if present)
8. `_core/config.py` — `BIN_VERSION` + `PLUGIN_VERSION` constants (if exists)

**Documentation**: `README.md` — release badge, plugin badge, and the example
health output `"version":"v<bin>"`.

## Submodule Release Flow

`plugins/aphrodite` is a git submodule → separate repo `PlayForm/Aphrodite-Hermes`.
`auto-release.sh` handles the full cross-repo release:

1. Bump `plugin.yaml` (and other submodule files) via sed
2. Commit in submodule: `cd plugins/aphrodite && git commit -m "release: plugin vX.Y.Z"`
3. Tag in submodule: `git tag "vX.Y.Z"`
4. Push submodule branch + tag to `$SUBMODULE_REMOTE` (default: `Source`)
5. Update parent pointer with `git update-index --cacheinfo` (needed because
   `.gitmodules` has `ignore = all`, which hides dirty submodules)
6. Commit parent pointer: `chore: sync aphrodite submodule → plugin vX.Y.Z`

Manually verify after release: grep README example output for `"version":"v`
(non-critical).

## Binary Symlink (dev)

```bash
ln -sf /path/to/repo/target/release/aphrodite ~/.hermes/aphrodite/aphrodite
```

## Release Notes — Content Standards

Every release MUST include: Summary, Changes, Infrastructure, What Ships, and
Links. Canonical template: `.hermes/RELEASE-TEMPLATE.md` (defines **Live** vs
**Retrospective** modes).

- Drafts live in `.plans/release-notes/` (`vNEXT-draft.md`,
  `headroom-fork-vNEXT-draft.md`). Stage `Maintain/release-notes-vX.Y.Z.md`
  explicitly — verify it is actually committed; it can drop out between
  `git add` and commit.
- **Live mode** (cutting the release now) requires a real `### Infrastructure`
  section with commands you actually ran. **Retrospective** (rewriting an
  already-shipped release) replaces Infrastructure with `### Verification`
  describing what was analyzed (commit range, diffstat) — never re-test.
- Draft placeholders (`{PENDING}` in Infrastructure, `{VERSION}` /
  `{PLUGIN_VERSION}` in title/compare link, `DO NOT PUBLISH` header) are
  by-design for drafts — never publish a note still containing `{PENDING}`.
- Headroom-fork notes are retrospective and separate from the binary notes;
  they use the fork's `aphrodite-vX.Y.Z` tag scheme and the
  `aphrodite-headroom-core` package name.
- Never ship a bare compare link with zero description.
- Never use backticks with `gh release create --notes` — the shell interprets
  them as command substitution. Always `--notes-file` with a heredoc:

```bash
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**

## Aphrodite vX.Y.Z 💋 Plugin vA.B.C

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
(4 targets × 3 files = 12 artifacts).

| Artifact pattern | Platform |
|------------------|----------|
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target |
| `SHA256SUMS-<target>.txt` | checksums for that target's two binaries |
| Plugin vA.B.C | Hermes (standalone repo) |

### Links
- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/...
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
EOF
# Normal tag pushes auto-attach every staged artifact; the glob form below is
# only for a from-source re-attach. Glob every staged file, never name 2 of 12:
gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md \
  staging/aphrodite-* staging/libaphrodite_hermes-* staging/SHA256SUMS-*.txt
```

## Commit & Sync Workflow

- **Commit:** `git gcommit-hermes` (LLM-generated message; alias → the local
  `Save` binary), `git gcommit`, or `git ecommit` (empty message).
- **Sync:** `git sync` — alias for
  `git pull --no-edit --allow-unrelated-histories; git push --recurse-submodules=on-demand`.

Reliability caveats:

- Never assume `gcommit-hermes` landed the commit — its `Save` backend calls an
  LLM provider; when the provider is unavailable the command exits with the
  change still staged. It also excludes `vendor` (and other dirs) from its diff
  view, so a submodule-pointer-only change can report "No staged changes
  found" while staged. Always verify with `git log -1` / `git status` and fall
  back to a plain `git commit -m "..."` if it didn't land.
- For submodule-pointer updates, `git add <submodule-path>` explicitly and
  confirm with `git diff --cached` before committing.

## Cross-Module Import Pitfall

Never add a cross-module import of a symbol without defining it in the target
module — `from .live import _is_live_tool` with no `_is_live_tool` in `.live`
silently kills the plugin at session start. Always test the full import chain
before releasing: `python3 -c "import aphrodite"`.