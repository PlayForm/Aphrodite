# DISCREPANCY-PAIR3-B-2026-09-19 - docs/install/ verified-stale findings

Append-only log for PAIR-3-B (docs/install/ rewrite). Format:
`- [date] file: claim X stale; source says Y (file:line)`.

- [2026-09-19] docs/install/macos-linux.md: "Refuses to run twice unless you pass `--force`" stale; source says only the `aphrodite.toml` write is `--force`-gated, while the binary, dylibs, `plugin.yaml`, and `__init__.py` shim are always overwritten on re-run (crates/aphrodite/src/setup.rs:92, 511, 578).
- [2026-09-19] docs/install/macos-linux.md: "Finds and copies both dylibs ... errors out naming the missing one if none are found" stale; source says `libaphrodite` is best-effort (warn + continue) and only `libaphrodite_hermes` aborts setup; local search also falls back to a SHA-256-verified GitHub-release download (crates/aphrodite/src/setup.rs:329-356).
- [2026-09-19] docs/install/macos-linux.md: "if missing, run `download.sh` yourself first" stale; source says the plugin auto-runs `download.sh` on registration when the binary or dylib is missing (plugins/aphrodite/**init**.py:793, 922).
- [2026-09-19] docs/install/macos-linux.md: "What changes" tree placed the binary at `~/.hermes/aphrodite/aphrodite`; source says binaries live in `~/.hermes/aphrodite/binaries/` (plugins/aphrodite/layout_schema.json:46-60, plugins/aphrodite/**init**.py:37).
- [2026-09-19] docs/install/windows.md: manual walkthrough placed binaries in `Aphrodite-Hermes\binaries\`; source says `download.ps1` and the loader resolve `%USERPROFILE%\.hermes\aphrodite\binaries` (plugins/aphrodite/download.ps1:27, plugins/aphrodite/**init**.py:37).
- [2026-09-19] docs/install/README.md + troubleshooting.md: config links used anchor `#api-key-resolution-chain`; source heading is `## API key resolution` -> `#api-key-resolution` (docs/config/aphrodite-toml.md:185).
- [2026-09-19] docs/install/troubleshooting.md: claimed `DEEPSEEK_API_KEY`/`HEADROOM_DEEPSEEK_KEY` env fallbacks; source says only `APHRODITE_API_KEY` is probed (crates/aphrodite/src/config.rs:302-314).
- [2026-09-19] docs/install/troubleshooting.md: `aphrodite_test` linked as `#7-aphrodite_test`; source section is `## 8. aphrodite_test` -> `#8-aphrodite_test` (docs/tool-relay/tools.md:202).
- [2026-09-19] docs/install/troubleshooting.md: claimed the only lever against auto-launch was removing the binary / pointing `APHRODITE_BINARY_PATH` at a nonexistent file; source says `APHRODITE_NO_AUTO_LAUNCH=1` skips auto-launch (plugins/aphrodite/**init**.py:914).
- [2026-09-19] docs/install/*.md: all cross-links used `tree/Development`; default branch is `Current` - replaced with relative links (verified: wave-1 docs/architecture + docs/ccr use relative links, no tree/ links).
