**[Compare Aphrodite/v1.8.0...Aphrodite/v1.8.1](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/v1.8.1)**

## Aphrodite v1.8.1 💋 Plugin v2.3.2

### Summary

`1.8.1` is the release where the documentation catches up to the engine. The
docs tree had been advertising a pairing two releases old (`docs/README.md`
pinned "1.6.6 binary / 2.2.6 plugin"), and the retention work that shipped in
`1.8.0` - `store_max_entries`, `store_max_bytes`, the debounced index-driven
sweep, the `fill_pct` relabelling - was documented nowhere an operator would
look for it. Both are fixed here: the docs no longer own a version number, the
README gains the sections a first install actually needs, and sqlite /
prometheus / lifecycle / `aphrodite.toml.example` now describe the store caps
and their restart semantics.

Two engine-side consequences ride along. **Truncation markers are ASCII now** -
every preview cap and directive cap emits `...` instead of `U+2026`, with the
cap arithmetic adjusted so the marker still fits inside its character budget.
And the dev-only `aphrodite_debug` tool is **gated at compile time** instead of
being stripped from the release tree by hand at staging: a release dylib
registers exactly the 13 declared production tools, a debug build registers 14,
and the catalog test asserts whichever profile it was compiled under.

One dependency had to move for the release to be shippable: `libsqlite3-sys`
0.38.2 bundles a SQLite affected by RUSTSEC-2026-0459 (CVE-2026-119375), so the
owned headroom fork bumps `rusqlite` `0.40.2 → 0.41` (`aphrodite-headroom-core`
`0.2.0 → 0.2.1`, bundled SQLite 3.54.1). No API reaches the parent.

### Changes

- **Fix (dependency security)**: `rusqlite` `0.40.2 → 0.41` in
  `vendor/headroom/crates/headroom-core`, pulling `libsqlite3-sys` `0.39.1` /
  bundled SQLite 3.54.1. Closes RUSTSEC-2026-0459 (CVE-2026-119375): a database
  opened together with a sibling `-wal` whose page size differs could return
  uninitialized page-cache bytes to ordinary queries. `CcrStore`,
  `SqliteCcrStore` and the retention surface are unchanged, so the parent pin
  moves with no code delta.
- **Fix (lint, floating nightly)**: five `clippy::unnecessary_to_owned` hits from
  `.contains(&x.to_string())` in `directives/action.rs`, `hooks.rs`,
  `resolve/recursive.rs` and `proxy.rs`, surfaced when CI's floating nightly
  moved to clippy 0.1.102. Rewritten as `iter().any(|i| i == x)`.
- **Fix (tool catalog)**: `aphrodite_debug` is behind
  `#[cfg(debug_assertions)]` in `crates/aphrodite-hermes/src/{schemas,tools,debug}.rs`
  instead of being deleted from the release tree during staging. The staging
  strip was a manual step that had to be repeated every ceremony and left
  `Current`'s source diverging from `Development`'s; the compiler now enforces
  the same contract. `hermes plugins validate` rule 6 (13 declared tools) holds
  for a release dylib without editing the tree.
- **Chore (preview + directive truncation)**: `U+2026` → `...` in
  `preview/builders/cap.rs`, the per-type preview builders (`gitlog`, `grep`,
  `html`, `ls`, `search`), the directive builders/loader, and the default marker
  hint. `apply_preview_cap` keeps the closing `]` by taking `max_chars - 4` and
  appending `...]`; the directive cap keeps `MAX_DIRECTIVE_CHARS` (+3) and
  `MAX_COMBINED_CHARS` (+4). Tests updated to the new character counts.
- **Docs (README)**: new sections - LLM provider configuration, Verify it's
  running, See It Work, Session Lifecycle, Directives, Features; corrected
  tuning defaults (`engine_threshold_pct` 30, `terminal_threshold` 256,
  `code_multiplier` 1.0, `retrieve_guidance = "minimal"`, `ccr_marker_hint`
  off); `aphrodite.toml.example` documents the retention caps, the
  `fill_pct`-is-not-occupancy caveat, and the ASCII hint strings.
- **Docs (tree)**: `docs/README.md` and `docs/architecture/README.md` no longer
  pin a binary/plugin version (they point at the manifests and the newest
  changelog entry); `docs/ccr/content-types.md` links retargeted to
  `docs/classification/content-types.md`; the `table` classifier row's escaped
  pipe fixed; the preview-shape table refreshed; `docs/ccr/backends/sqlite.md`
  gains "Retention: Space Caps", `docs/metrics/prometheus.md` gains "Retention
  and Eviction Telemetry", and the backend expiry table in
  `docs/ccr/lifecycle.md` names the space caps.
- **Chore**: `.gitignore` ignores `*.mp4`; `.githooks/{pre-commit,post-checkout}`
  whitespace-normalized (`>/dev/null` → `> /dev/null`).

### What this release does not include

No engine, store, proxy or FFI behaviour changes beyond the three above - the
`1.8.0` retention caps, the index-driven sweep and the headroom fork's
compression/detection code are carried over untouched; the fork moves only its
`rusqlite` constraint. The plugin does move this cycle: its `install_message`
carried a stale `rhai pinned 1.25.1`, the tag's `Finalize` regenerated its
in-tree `SHA256SUMS.txt`, and `BINARY_VERSION` had to name `1.8.1` - three
commits past `v2.3.1`, so the plugin version advances to `v2.3.2` and is tagged
alongside this binary, the way `v2.3.1` paired with `BINARY_VERSION 1.8.0`.

### Infrastructure

- Version track: `crates/aphrodite` + `crates/aphrodite-hermes` + the
  `aphrodite-hermes` dependency pin + `package.json` → `1.8.1`, README release
  badge → v1.8.1; `aphrodite-headroom-core` `0.2.0 → 0.2.1` (fork) and the parent
  pin `crates/aphrodite/Cargo.toml` :55 moved with it. `plugin.yaml` and its
  `install_message` header move `2.3.1 → 2.3.2` with the plugin README badge,
  and `BINARY_VERSION` moves to `1.8.1` only after the release assets exist: it
  names a release, so it is set last.
- crates.io availability before tagging: `aphrodite` `max_version` 1.8.0,
  `aphrodite-hermes` `max_version` 1.8.0, `aphrodite-headroom-core` `max_version`
  0.2.0 → `1.8.1` / `1.8.1` / `0.2.1` free.
- Toolchain: the local nightly was `1.102.0-nightly (8eb778d04 2026-11-12)` /
  clippy 0.1.101 while CI resolves the newest; refreshed with `rustup toolchain
  install nightly --profile minimal --force` → `5f4effd57 2026-11-13` /
  clippy 0.1.102, which reproduced CI's five `clippy::unnecessary_to_owned`
  errors locally. No formatting drift from the bump (`cargo fmt --all --
  --check` clean).
- Gates on the final release tree: `cargo check --workspace --release` ✅ 37.52s
  · `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ 29.15s ·
  `cargo fmt --all -- --check` ✅ · `cargo clippy -p aphrodite -p
  aphrodite-hermes -- -D warnings` ✅ (the CI invocation) · `cargo deny check` ✅
  `advisories ok, bans ok, licenses ok, sources ok` (was red on four advisories
  before the `rusqlite`/`rustls` resolution). `--all-targets` additionally
  builds targets CI never builds and reports 16 pre-existing lints in `examples/`
  and test modules this release does not touch.
- Tests: `cargo test -p aphrodite --lib` ✅ 500 passed / 0 failed / 1 ignored ·
  `cargo test -p aphrodite --bins` ✅ 16 passed / 0 failed ·
  `cargo test -p aphrodite-hermes --lib` ✅ 61 passed / 0 failed. One transient
  failure was observed on the first run of an earlier battery (499 passed / 1
  failed) and did not reproduce across 16 subsequent runs; the test name was not
  captured, so it is recorded as a pre-existing flake risk rather than a
  blocker - the test code is byte-identical to `Development`'s.
- Transplant audit: 41 files, +370/−135, one commit (`809da98f`). Held back and
  verified unchanged: `.gitmodules`, `.github/workflows/*`,
  `.github/Update.md`, the `plugins/aphrodite` gitlink, and
  `crates/aphrodite/Cargo.toml` (held back from the transplant because its only
  Development delta was `Current`'s `lru 0.19.0` / `rhai =1.27.0` pins - crossing
  it would have reverted them; it is touched afterwards only to move the
  headroom-core pin). `README.md` was the only file edited on both sides;
  `git merge-file` resolved it clean and kept `Current`'s inclusivity-emoji
  contribution heading.
- Fork cycle: `vendor/headroom` `177a4f41 → c8029afb` (`f9556f13` is the
  version-carrying bump commit). `RELEASE-CYCLE.md` records 0.2.1 with the
  `aphrodite-v0.12.0` fork tag still to cut; `CHANGELOG.md` gains the 0.2.1
  entry and corrects the 0.2.0 row, which still claimed "not published" although
  crates.io has served 0.2.0 since `Aphrodite/v1.8.0`.
- Known pre-existing reds, not introduced here: `Maintain/Check.sh` on `Current`
  calls `cargo ci-hooks` (`Maintain/dynamic_hooks/`), which the previous
  promotion also held back from this line; `Maintain/CheckParity.py` compares
  against a `Check.yml` that is deliberately test-free on this branch; and
  `vendor/headroom/crates/headroom-core/benches/ccr_store.rs:61` implements
  `CcrStore::put` as `-> bool` against a unit-returning trait, so the fork's
  bench target does not compile - no gate builds benches.
- Release chain: `Aphrodite/v1.8.1` (signed, cut on `1f7d0e23`) drove `Build.yml`
  - `Release`, all four matrix legs and `Finalize` succeeded; the release carries
  the full 12-asset matrix. `Publish.yml` followed on `workflow_run`: `aphrodite`
  1.8.1, `aphrodite-hermes` 1.8.1 and `aphrodite-headroom-core` 0.2.1 are all
  live on crates.io (the last gated on `published == 'false'`, since 0.2.0 was
  already there).
- `Bump plugin submodule gitlink` failed inside `Publish.yml` and is expected to:
  it commits on the tag's detached HEAD and pushes to `Current`, which by then
  already carried `51343c1f` - the manual float of that same gitlink. Its outcome
  is in place (`Current` -> `8c09607`, the child commit whose `BINARY_VERSION`
  1.8.1 matches its in-tree `SHA256SUMS.txt`), so the job is redundant rather
  than broken; re-running it only rebuilds the same behind-tip commit.
- Still outstanding: the `aphrodite-v0.12.0` fork tag - the crate itself is
  published as 0.2.1.

### What Ships

`Build.yml`'s 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.2                                                        | Hermes (standalone repo, `v2.3.2` tag)   |
| `aphrodite-headroom-core` 0.2.1                                      | crates.io, by `Publish-Headroom-Core`    |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/v1.8.1
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
