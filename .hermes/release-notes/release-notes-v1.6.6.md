**[Compare Aphrodite/v1.6.5...Aphrodite/v1.6.6](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.5...Aphrodite/v1.6.6)**

## Aphrodite 1.6.6 💋 Plugin v2.2.6

### Summary

1.6.6 is the second `hermes-agent` catalog-review release (PR 118488, review
5441199534), hardening the two paths the reviewer flagged: `aphrodite_prefetch`
now refuses sensitive paths outright (`.env`, `auth.json`, `~/.ssh`, Hermes
home, non-file paths) and `aphrodite setup` never touches the Hermes plugin
directory - it is report-only and points at the download scripts. The dylib
arm joins the strict in-tree-checksum policy (missing sums = hard refusal),
the context engine is re-described as opt-in with a `requires_hermes` floor,
and config + docs are re-audited 1:1 against the parsers. Binary
`1.6.5 → 1.6.6`, plugin `2.2.5 → 2.2.6`.

### Changes

- **Fix (prefetch, path guard)**: `ReadOutcome::Refused` + `refuse_reason`
  on `.env`, `auth.json`, `~/.ssh`, Hermes-home paths, and non-file paths -
  `aphrodite_prefetch` never reads what the reviewer flagged. 10/10 tests.
- **Fix (setup, plugin-dir guard)**: the catalog-install guard makes the
  Hermes-home plugin directory report-only - no symlink removal/create, no
  writes, no deletes; the notice points at `bash download.sh` /
  `pwsh download.ps1`.
- **Fix (setup, dylib checksums)**: the dylib arm hard-refuses (`Err`) when
  `SHA256SUMS.txt` is missing, lacks a target block, or lacks an asset entry -
  the mandatory-sums policy now covers both binaries.
- **Fix (setup, hints)**: the shim carries the explicit download-script
  hints; `plugins/aphrodite/__init__.py` and
  `crates/aphrodite/templates/__init__.py` stay byte-identical (drift-guard).
- **Feature (plugin, opt-in context engine)**: description corrected to
  "opt-in context engine" - registration is `APHRODITE_CONTEXT_ENGINE=1`
  (TOML `compression.context_engine` defaults true), and `requires_hermes:
  ">=0.20.2"` (the first Hermes release with `pre_tool_call` `modify`).
- **Feature (plugin, Disclosure)**: `plugin.yaml` and the parent `write.rs`
  description disclose the `pre_tool_call` behavior (auto-backgrounds long
  terminal/process commands, rewrites chained terminal commands when enabled).
- **Chore (repo, config 1:1 audit)**: both TOMLs mutually consistent and 1:1
  with the parsers; dead `[templates.*]` subtree and `compression.prefetch`
  removed; `chain_split_min/max_segments` and `defaults.api_url/model` added;
  unwired keys annotated.
- **Docs**: 21 doc files fixed (thresholds, 13 tools, 30-type classifier, no
  hot-reload claims, explicit `bash download.sh` install, endpoints table,
  version stamps 1.6.5 → 1.6.6).

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ (fresh
  binaries at `target/release/`, `--version` reports v1.6.6)
- Check: `cargo check -p aphrodite -p aphrodite-hermes` ✅ (0 errors, proves
  crates + dep pin moved together)
- Tests: `cargo test -p aphrodite --lib` ✅ 410 passed / 0 failed / 1 ignored
  (network-gated); config 16/16, setup 6/6, prefetch 10/10
- Lint/format: `cargo +nightly-2026-05-01 fmt --all -- --check` ✅;
  drift-guard byte-identical ✅; `ruff check plugins/aphrodite/` ✅;
  `npx prettier --check` on owned docs ✅
- Version track: crates + pin + `package.json` 1.6.6; `plugin.yaml` 2.2.6;
  `BINARY_VERSION` 1.6.6 (bumped LAST, after assets); README badges
  v1.6.6 / v2.2.6

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.6                                                        | Hermes (standalone repo)                 |

`aphrodite-headroom-core` stays at 0.1.3 (live since 2026-09-20); the fork
holds 2 dependency-update commits post-0.1.3 (fastembed→5, crate version
refresh) - dep-only delta, no fork version bump in this release per the 1.6.4
precedent (the fork publish gate is unreachable under the workflow_run chain).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.5...Aphrodite/v1.6.6
- **CHANGELOG.md**: [CHANGELOG.md](https://github.com/PlayForm/Aphrodite/blob/Current/CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite