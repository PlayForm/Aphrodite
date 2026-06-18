---
name: aphrodite-operations
description: Operational patterns for working with aphrodite - engine compression
    handling, prefetch workflow, dual-mode rebuild, standalone repo sync.
version: 1.0.0
---

# Aphrodite Operations

Day-to-day operational patterns when working inside an aphrodite-compressed
session.

## Engine Compression + Prefetch Workflow

When the context engine compresses tool output into CCR markers, do NOT fight it
by re-reading the same file with different offsets or tools. The engine
compresses every read. Instead:

1. **Plan reads ahead** - use `aphrodite_prefetch(paths=[...])` to read files in
   background. Markers return instantly; content loads concurrently.
2. **Retrieve, don't re-read** - when you get `<<<CCR:hash|type|size>>>`, use
   `aphrodite_retrieve(hash)`. Don't call `read_file` again with different
   offsets.
3. **Write terminal output to files** - `command > /tmp/out.txt 2>&1` then
   prefetch/retrieve.
4. **Do other work while waiting** - dispatch prefetches, proceed with
   independent tasks, check `aphrodite_prefetch_status` for readiness.

**Anti-pattern**: calling `read_file` 3+ times on the same file with different
offsets, each time getting a compressed marker. The engine compresses every
read.

## Dual-Mode Rebuild (`_rebuild_handler`)

The `aphrodite_rebuild` tool supports two modes:

- **Dev mode**: if `Cargo.toml` exists in a parent directory, builds from Rust
  source via `cargo build --release`.
- **User mode**: if no Cargo workspace found (standalone install), re-downloads
  the binary from GitHub Releases via `_download_binary()`.

The `_find_cargo_toml()` helper walks up to 6 directories from the rebuild
module to locate the workspace root.

## Standalone Plugin Repo

End users install from
[Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes) - a lightweight
repo containing only Python plugin files (no binary, no monorepo). The binary is
auto-downloaded from
[GitHub Releases](https://github.com/PlayForm/Aphrodite/releases) on first
session start.

After releasing in the monorepo, sync files to the standalone repo:

```bash
cp plugins/aphrodite/_core/config.py $STANDALONE/_core/config.py
cp plugins/aphrodite/plugin.yaml $STANDALONE/plugin.yaml
cp plugins/aphrodite/pyproject.toml $STANDALONE/pyproject.toml
cp plugins/aphrodite/__init__.py $STANDALONE/__init__.py
cp plugins/aphrodite/_hooks/rebuild.py $STANDALONE/_hooks/rebuild.py
cd $STANDALONE && git add -A && git commit -m "sync: v$NEW" && git push
```

## `--version` Flag

The Rust binary's `--version` flag is only parsed by clap when `Cli::parse()` is
called (no `aphrodite.toml` exists). Added early handling in `main()` to
intercept `--version`/`-V` before config loading, ensuring it works regardless
of config state.

## Pitfalls

- `_rebuild_handler` repo path: from `_hooks/rebuild.py`, use
  `_find_cargo_toml()` which walks up, not hardcoded `dirname()` chains that
  break when directory structure changes.
- `_check_binary_version()` calls `[BINARY, "--version"]` - ensure the binary
  actually supports it (fixed in v0.8.6).
- Plugin-shipped skills are read-only via `skill_manage` - create profile-level
  skills for operational patterns.
