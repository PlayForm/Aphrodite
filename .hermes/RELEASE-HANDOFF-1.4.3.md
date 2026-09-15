# Release Handoff - Aphrodite v1.4.3 / Plugin v2.1.3

Status: **ready to release** - everything is prepared on `Development`; the
release ceremony (sync to `Current` + tags + GitHub release) is the only
remaining step. Follow `aphrodite-branch-release-flow` skill.

## Branch state (both repos on Development)

- Parent HEAD: `5effa73` - bench suite committed; 7 Rust files + 1 new file
  (chain_split.rs) are **UNCOMMITTED working-tree changes** (the chain-split
  feature). Version bump already committed in `5d8c09a`.
- Submodule: `plugins/aphrodite` at `350e8c2` (test: perf probe) - clean,
  on Development. plugin.yaml already `2.1.3`.
- Versions everywhere: binary **1.4.3**, plugin **2.1.3**, BINARY_VERSION
  `1.4.3`.

## What's committed (release-relevant)

| Commit | Content |
|---|---|
| `5d8c09a` | bump v1.4.3 + real-corpus benchmark tooling (benchmark-report.py) |
| `8afe8e0` | release notes `Maintain/release-notes-v1.4.3.md` (full: Summary/Changes/Infrastructure/What Ships/Links), CHANGELOG moved to root |
| `db12c86` | draft release notes |
| `48abe46` | gitignore/prettierignore refinements |
| `5effa73` | **standalone CCR benchmark suite** - bench/corpus (16 files), bench/compression (crate, 6/6 crash tests), bench/proxy, bench/agents, bench/conversational (5 tasks + migrated 2026-07-20 run) |

## UNCOMMITTED - the chain-split feature (must be committed before release)

New `chain_split.rs` + wiring in 7 files: `hooks.rs` (split marked output in
transform_terminal_output), `lib.rs` (module + replacement_from chain_summary),
`state.rs` + `config_loader.rs` (`chain_split_enabled`, env
`APHRODITE_CHAIN_SPLIT`, TOML `[compression] chain_split`, default true),
`aphrodite-hermes/src/lib.rs` (pre_tool_call rewrite of chained commands).

**Verified**: 6 unit tests pass, full suite 393 tests 0 failures, clippy
`-D warnings` clean, live FFI test confirms rewrite + split. The feature is
ACTIVE in this session via hot-reload (dylib rebuilt 22:31, generations
.28507.5/.69010.0 in ~/.hermes/aphrodite/hotreload/).

**KNOWN ISSUE to fix before/after release**: the segment marker
`__APHRODITE_SEG__` lands in **redirected stdout** (e.g. `cmd > file`) -
observed when my own chained commands wrote markers into files. Harmless for
interactive output, but a real side effect for file-redirected chains.
Consider stderr/side-stream markers or post-processing before shipping.

## Gates (all green, verified this session)

- clippy `-D warnings` ✅ · release build ✅ · cargo test 393 ✅ · plugin
  pytest 22+3 ✅ · cargo deny advisories ✅ (rustls 0.23.45, lru 0.18.4)
- Fix-layer reverification: all 6 issues/PRs complete and correct ✅

## Release ceremony (per `aphrodite-branch-release-flow` skill)

1. **Commit the chain-split feature** on Development (7 modified + 1 new).
2. Prepare on Development (already done: bumps, notes, gates).
3. Sync plugin first: S-Dev → S-Current (squash), commit
   `release: sync v2.1.3`, tag `v2.1.3` on S-Current, push.
4. Sync parent: clean tree → checkout Current → squash Development →
   restore protected paths (.gitmodules, .github/workflows, plugins/aphrodite
   gitlink) → float submodule to S-Current tip → verify → commit
   `release: sync v1.4.3 from Development` → push → tag `Aphrodite/v1.4.3`
   on Current → push tag.
5. GitHub release from the tag (notes via --notes-file; notes already
   drafted at `Maintain/release-notes-v1.4.3.md` - review it, it may need a
   chain-split feature bullet added).
6. Return working copy to Development (I10).
7. Optionally: benchmark measurement of chain-split (compression-aware task
   in corpus) before tagging, to include real numbers in notes.

## Known observations worth reporting in the release

- bench proxy run found `wide_5k_keys.json` round-trip NOT byte-identical
  (`{` → `[` re-serialization by JSON minify) - first real finding from the
  new bench suite.
- `code_go.go` classifier blind spot (detected `build` due to `\bERROR\b`
  log heuristic) - documented in bench/corpus README.