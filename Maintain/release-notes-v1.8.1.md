**[Compare Aphrodite/v1.8.0...Aphrodite/v1.8.1](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/v1.8.1)**

## Aphrodite v1.8.1 💋 Plugin v2.3.1

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

### Changes

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

No engine, store, proxy or FFI behaviour changes beyond the two above - the
`1.8.0` retention caps, the index-driven sweep and the headroom fork resync are
carried over untouched. `aphrodite-headroom-core` stays at `0.2.0` (no fork
delta in this release), and the plugin is unchanged at `v2.3.1`: its README,
`plugin.yaml` wording and loader message already landed on `Current` with
`1.8.0`, so this ceremony only moves the parent.

### Infrastructure

- Version track: `crates/aphrodite` + `crates/aphrodite-hermes` + the
  `aphrodite-hermes` dependency pin + `package.json` → `1.8.1` in one commit
  (`c8f6e310`), README release badge → v1.8.1. `plugin.yaml` stays `2.3.1` -
  the plugin carries no delta in this release - and `BINARY_VERSION` moves to
  `1.8.1` only after the release assets exist: it names a release, so it is set
  last.
- crates.io availability before tagging: `aphrodite` `max_version` 1.8.0,
  `aphrodite-hermes` `max_version` 1.8.0 → `1.8.1` free on both.
- Gates on the release tree (`Current` at `c8f6e310`): `cargo check
  --workspace` ✅ · `cargo build --release -p aphrodite -p aphrodite-hermes` ✅
  1m42s on `aarch64-apple-darwin` · `cargo fmt --all -- --check` ✅ ·
  `cargo clippy -p aphrodite -p aphrodite-hermes -- -D warnings` ✅ (the CI
  invocation; `--all-targets` additionally lints `examples/` and the test
  modules that CI never lints, and reports 16 pre-existing lints in files this
  release does not touch).
- Tests: `cargo test -p aphrodite --lib` ✅ 500 passed / 0 failed / 1 ignored ·
  `cargo test -p aphrodite-hermes --lib` ✅ 61 passed / 0 failed. One transient
  failure was observed on the first run of the battery (499 passed / 1 failed)
  and did not reproduce across 16 subsequent runs; the test name was not
  captured, so it is recorded as a pre-existing flake risk rather than a
  blocker - the test code is byte-identical to `Development`'s.
- Transplant audit: 41 files, +370/−135, one commit (`809da98f`). Held back and
  verified unchanged: `.gitmodules`, `.github/workflows/*`,
  `.github/Update.md`, the `plugins/aphrodite` gitlink, and
  `crates/aphrodite/Cargo.toml` (whose only delta was `Current`'s `lru 0.19.0` /
  `rhai =1.27.0` pins - crossing it would have reverted them). `README.md` was
  the only file edited on both sides; `git merge-file` resolved it clean and
  kept `Current`'s inclusivity-emoji contribution heading.
- Known pre-existing reds, not introduced here: `Maintain/Check.sh` on `Current`
  calls `cargo ci-hooks` (`Maintain/dynamic_hooks/`), which the previous
  promotion also held back from this line; `Maintain/CheckParity.py` compares
  against a `Check.yml` that is deliberately test-free on this branch.
- Still outstanding at tag time: the `Aphrodite/v1.8.1` tag, the `Build.yml`
  4-target matrix and its 12 assets, `Publish.yml` via `workflow_run`, and the
  `BINARY_VERSION` bump to `1.8.1`.

### What Ships

`Build.yml`'s 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.1                                                        | Hermes (standalone repo, unchanged)      |

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.8.0...Aphrodite/v1.8.1
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
