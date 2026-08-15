**[Compare Aphrodite/v1.3.9...Aphrodite/v1.4.0](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.3.9...Aphrodite/v1.4.0)**

## Aphrodite 1.4.0 💋 Plugin v2.1.0

### Summary

A packaging, runtime-cache-lifecycle, and configuration-defaults release that
resolves every issue reported against the official Aphrodite 1.3.9 Linux x86_64
proxy and Hermes dylib. The core CCR compression engine is unchanged - all
fixes are in the release/update path, the hot-reload cache, directive
packaging, the plugin manifest, and proxy TOML defaults.

### Changes

- **Fix (feedback #1):** `BINARY_VERSION` no longer holds a stray `1|1.3.9`
  line-number artifact (it now contains exactly `1.3.9`), and `download.sh`
  validates the version against a strict semver pattern before building a
  release URL, so a malformed value can no longer break the update path.
- **Fix (feedback #2):** replaced the broken absolute `directives` symlink
  (`-> /Users/nikola/.../Aphrodite/directives`) with a real, portable
  `directives/` directory shipped in the plugin repo. The directive loader now
  resolves from `~/.hermes/aphrodite/directives` (Aphrodite namespace, no
  collision with other tools) and falls back to built-in directives when the
  directory is missing or unreadable.
- **Fix (feedback #3):** hot-reload dylib copies now live in
  `~/.hermes/aphrodite/hotreload` (outside the plugin tree, so
  `hermes plugins doctor` never stages/copies them), dead-PID copies are
  reaped on startup, each live process keeps only its newest generation, and an
  `atexit` sweep removes the current process's own copy - bounded storage even
  across many terminated processes (~19 GB → ~10 MB in the report).
- **Fix (feedback #4):** `plugin.yaml` now lists `pre_tool_call` under
  `provides_hooks`, and `aphrodite_navigate` was removed from `provides_tools`
  because the released dylib does not register it (the `navigation` feature is
  currently unbuildable) - the manifest now matches runtime registration.
- **Fix (feedback #5):** `MultiConfig.proxies` defaults to an empty list when
  the `[proxies]` table is absent, so a hook-only configuration no longer fails
  with `missing field 'proxies'`.
- **Chore:** added a Python regression test for bounded hot-reload storage, and
  regenerated the embedded plugin shim template to match the live `__init__.py`.

### Infrastructure

- Build: `cargo build --release -p aphrodite` ✅
- Tests: `cargo test -p aphrodite` ✅ (325 passed, 0 failed)
- Python tests: `python3.11 tests/test_hotreload_cleanup.py` ✅ (4 passed)
- Note: the plugin uses PEP 604 (`X | None`) syntax, so the Python test suite
  requires Python >= 3.10 (CI must use 3.10+, not 3.9).

### What Ships

| Artifact | Platform |
|----------|----------|
| `aphrodite-aarch64-apple-darwin` | macOS ARM64 |
| `aphrodite-x86_64-apple-darwin` | macOS Intel |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64 |
| `aphrodite-x86_64-pc-windows-msvc` | Windows x86_64 |
| Plugin v2.1.0 | Hermes (standalone repo `Aphrodite-Hermes`) |

> **Always list all four targets above - do NOT trim from a live asset count.**
> `Build.yml`'s matrix always produces the full set, and the `Finalize` job
> fails the release if any platform is missing.

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.3.9...Aphrodite/v1.4.0
- **CHANGELOG.md**: [Maintain/CHANGELOG.md](Maintain/CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
