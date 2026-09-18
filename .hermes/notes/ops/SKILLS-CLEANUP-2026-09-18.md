# Skills Library Cleanup - 2026-09-18

Scope: `.hermes/skills/**` only (13 skill dirs). Branch `Development`, HEAD
79bfa38. Manual edits only; no commits, no pushes (tree left dirty with the
edits below, per instructions). Prettier gate: all touched `.md` verified
clean (`npx prettier --check`).

## Live facts verified during this cleanup (ground truth for refreshes)

| Fact               | Value                                                                                                                                                                                                                                                                                                                                       | Verified by                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| Binary version     | 1.4.6                                                                                                                                                                                                                                                                                                                                       | `plugins/aphrodite/BINARY_VERSION`          |
| Plugin version     | 2.1.4                                                                                                                                                                                                                                                                                                                                       | `plugins/aphrodite/plugin.yaml`             |
| Runtime home       | `~/.hermes/aphrodite/` (toml, binaries, ccr.db, directives, hotreload)                                                                                                                                                                                                                                                                      | `ls`                                        |
| Plugin shape       | pure loader: `__init__.py`, `_bindings.py`, `BINARY_VERSION`, `download.sh/ps1`, `layout_check.py`, `layout_schema.json`, `plugin.yaml`, `README.md`, `tests/` - **no `_core/`, no `_hooks/`, no `pyproject.toml`**                                                                                                                         | `ls plugins/aphrodite`                      |
| Toolchain          | repo pins `1.96.0` (`rust-toolchain.toml`); `nightly-2026-05-01-aarch64-apple-darwin` installed for rustfmt; no `~/.cargo/config.toml`                                                                                                                                                                                                      | `rustup toolchain list`                     |
| Submodules         | all clean, no `+`: `plugins/aphrodite` (v2.1.3-30-g47cb321), `vendor/headroom`, `vendor/rtk`; `.gitmodules` uses `ignore = dirty` (NOT `all`)                                                                                                                                                                                               | `git submodule status`, `cat .gitmodules`   |
| Auto-expand        | **vestigial**: `auto_expand`/`auto_expand_limit` serialized by the proxy but marked "have no consumer" (`crates/aphrodite/src/proxy.rs`); `APHRODITE_AUTO_EXPAND` / `APHRODITE_NO_AUTO_EXPAND` have zero consumers (plugin `__init__.py` has no AUTO_EXPAND hits)                                                                           | `grep` over plugin + crates                 |
| Engine knobs       | `APHRODITE_ENGINE_THRESHOLD_PCT/MIN_MSGS/PROTECT_FIRST/PROTECT_LAST` all live in `config_loader.rs` (defaults 45/8/2/5); live TOML sets `engine_threshold_pct = 100` (effectively off)                                                                                                                                                      | grep + `~/.hermes/aphrodite/aphrodite.toml` |
| Terminal threshold | 512 bytes (`terminal_threshold`)                                                                                                                                                                                                                                                                                                            | live TOML                                   |
| `aphrodite_test`   | `quick` = 1 sample (source_code), `full` = 3; proxies `:9797/:9798` currently down (accepted state)                                                                                                                                                                                                                                         | live run                                    |
| Referenced paths   | `Maintain/scripts/release/auto-release.sh`, `~/Developer/Maintain/Fn/Update/Cargo.sh`, `.hermes/release/RELEASE-TEMPLATE.md`, `crates/aphrodite/templates/__init__.py`, `Maintain/{check_ffi_contract.py,tests/test_check_ffi_contract.py}`, `crates/aphrodite-hermes/codegen/test_finalize_bindings.py`, `.hermes/release-notes` all exist | `test -e`                                   |
| Profile            | `~/.hermes/profiles/dev-aphrodite/plugins/aphrodite` symlinks to the repo; repo listed in `skills.trusted_project_dirs`                                                                                                                                                                                                                     | `ls -la`, config grep                       |

## Per-skill inventory (13)

| Skill                           | Ver   | Status                                                                   | Action                                                                                                                                                                                                                                                                               |
| ------------------------------- | ----- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `aphrodite-release-flow`        | 2.0.0 | canonical ceremony                                                       | + Related pointer to `aphrodite-release-workflow`                                                                                                                                                                                                                                    |
| `aphrodite-branch-release-flow` | 1.1.0 | DEPRECATED (retired 2026-09-18, banner already in place from prior wave) | none - banner + archive pointer present                                                                                                                                                                                                                                              |
| `aphrodite-release-workflow`    | 1.6.0 | refreshed                                                                | Submodule Release Flow rewritten (defer to `release-flow`; `.gitmodules ignore = dirty` corrected from stale `ignore = all`; retired `update-index --cacheinfo` flow marked retired); version locations annotated for pure-loader plugin (no `_core/config.py`, no `pyproject.toml`) |
| `aphrodite-upgrade-breakpoints` | 1.0.2 | CONSOLIDATED                                                             | content absorbed into `aphrodite-operations` + `aphrodite-cargo-upgrade`; in-place ARCHIVED banner + archive snapshot; description fixed to trigger-first                                                                                                                            |
| `aphrodite-v0.8.6-patterns`     | 1.2.0 | ARCHIVED (historical)                                                    | in-place ARCHIVED banner + archive snapshot; description fixed to trigger-first                                                                                                                                                                                                      |
| `aphrodite-operations`          | 1.1.0 | refreshed                                                                | Dual-Mode Rebuild: `_hooks/rebuild.py` + `_find_cargo_toml()` removed (pure-loader merge), `aphrodite_rebuild` = dylib version + proxy health; Standalone Plugin Repo Sync replaced with "Plugin Repo = the Submodule" (no more `cp` to a `$STANDALONE` copy)                        |
| `aphrodite-development-lessons` | 1.2.0 | refreshed                                                                | Version Bump Locations: stale 5-location set (`_core/config.py`, `pyproject.toml`) replaced with pointer to `release-flow`/`release-workflow` + pure-loader facts                                                                                                                    |
| `aphrodite-auto-expand-testing` | 2.2.0 | refreshed (rewrite)                                                      | whole protocol rewritten to verified reality: auto-expand is vestigial (no consumer), env knobs table (verified in `config_loader.rs`), live TOML defaults, retrieve-first rule; removed `APHRODITE_AUTO_EXPAND`/`NO_AUTO_EXPAND` false claims                                       |
| `aphrodite-benchmarking`        | 1.3.0 | refreshed                                                                | terminal threshold `>1KB` → `terminal_threshold` (live 512B); engine "450K tokens" → `engine_threshold_pct` (live 100 = off, default 45)                                                                                                                                             |
| `aphrodite-tool-testing`        | 1.1.0 | refreshed                                                                | Auto-Expand section: removed "with auto-expand on, outputs expand inline" (vestigial) - retrieve-first is unconditional                                                                                                                                                              |
| `aphrodite-hook-reference`      | 1.3.0 | refreshed                                                                | Reference Files list completed (added `hook-invocation-verification.md`, `session-discoveries-20260615.md`); ContextEngine section gained the `compression.context_engine` TOML runtime toggle                                                                                       |
| `aphrodite-testing-discipline`  | 1.0.0 | current                                                                  | none (BINARY_VERSION 1.4.6 and gates verified current)                                                                                                                                                                                                                               |
| `aphrodite-cargo-upgrade`       | 1.1.0 | current                                                                  | none (`~/Developer/Maintain/Fn/Update/Cargo.sh` verified; breakpoints for current dep set)                                                                                                                                                                                           |

## Consolidations (absorbed -> canonical)

| Absorbed                               | Canonical home                                                                                                    | Evidence                                                                                      |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `aphrodite-branch-release-flow` v1.1.0 | `aphrodite-release-flow` v2.0.0                                                                                   | DEPRECATED banner in place; snapshot in `ARCHIVE-2026-09-18/aphrodite-branch-release-flow/`   |
| `aphrodite-upgrade-breakpoints` v1.0.1 | `aphrodite-operations` (rebuild path, `--version`, standalone repo) + `aphrodite-cargo-upgrade` (dep breakpoints) | in-place CONSOLIDATED banner; snapshot in `ARCHIVE-2026-09-18/aphrodite-upgrade-breakpoints/` |
| `aphrodite-v0.8.6-patterns` v1.1.0     | `aphrodite-operations`, `aphrodite-development-lessons`, `aphrodite-cargo-upgrade`                                | in-place ARCHIVED banner; snapshot in `ARCHIVE-2026-09-18/aphrodite-v0.8.6-patterns/`         |

No skill was deleted; each absorbed skill keeps its SKILL.md in place with an
explicit banner and an archive snapshot (evidence rule).

## Prior wave (resumed state, already done)

The 3 snapshots under `.hermes/notes/ops/ARCHIVE-2026-09-18/` plus its
`README.md` (pointer table) and `MERGE-SUBMODULE.md` were placed by the
earlier rate-limited wave and are recorded here for completeness:
`aphrodite-branch-release-flow/`, `aphrodite-upgrade-breakpoints/`,
`aphrodite-v0.8.6-patterns/` - each a copy of the in-place SKILL.md at archive
time. The originals were left in place ("deprecated in place" per the archive
README), which this wave completed with the in-place banners.

## Description frontmatter (trigger convention)

All descriptions now follow "Use when <trigger>. <behavior>." - fixed in
`aphrodite-upgrade-breakpoints` and `aphrodite-v0.8.6-patterns` (were not
trigger-first); the other 11 already conformed.

## Notes / residual

- `.hermes/AGENTS.md` still lists all 13 skills (incl. the two archived-in-place
  ones) - untouched, out of scope; a future AGENTS.md pass could mark
  `aphrodite-upgrade-breakpoints` / `aphrodite-v0.8.6-patterns` as
  archival, but AGENTS.md edits are outside this wave's scope.
- No commits/pushes made; working tree contains only the skill edits above
  (verify with `git diff --stat .hermes/skills`).
- The `auto-expand` vestigial finding is a code-level fact (proxy.rs comment,
  zero env consumers) worth a follow-up cleanup of the dead TOML keys.
