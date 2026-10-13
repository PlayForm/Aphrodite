> **Draft** - fork release note for the {VERSION} release cycle; `{VERSION}`
> stays a placeholder until the version sibling cuts the bump. The fork itself
> ships no new crate version in this cycle (dep-only delta, per the 1.6.4
> precedent); this note tracks the fork's position in the {VERSION} release.

## Headroom Fork: aphrodite-headroom-core 0.1.3 (unchanged) · fork tag aphrodite-v0.10.0 (unchanged)

### Summary

The PlayForm/Headroom fork does not bump in the {VERSION} release:
`aphrodite-headroom-core` stays at 0.1.3 (live since 2026-09-20) and the fork
tag stays at `aphrodite-v0.10.0`. The fork holds two dependency-update
commits post-tag - fastembed→5 and a crate-version refresh - a dep-only
delta, no fork-side feature work. The main {VERSION} release (the Rust-engine
port) consumes the vendored core where it matters: the context engine keys
its archives with `headroom_core::ccr::compute_key` (BLAKE3), and the
inline/CCR store paths ride the same vendored machinery.

### Changes

- **Chore (deps, fastembed)**: `fix(dependencies): update fastembed version
to 5 in Cargo.toml` (`fc58ee39`).
- **Chore (deps, refresh)**: `chore(dependencies): update various crate
versions in Cargo.toml` (`41e3602b`).
- **Feature (main release, {VERSION})** - the release this fork note
  accompanies; see `vNEXT-draft.md` for the full main note:
    - TOML templates (`[templates.preview.*]` / `[templates.marker]` /
      `[templates.prompts]` / `[templates.reverse]`) are live config.
    - Auto-expand: `auto_expand` / `auto_expand_limit` wired.
    - Context engine: `engine_*` keys load-bearing (middle-turn archiving +
      `engine_offload` prompt).
    - Live containers: `APHRODITE_LIVE_CONTAINER` streaming mode.
    - Rhai scripting: `scripting` Cargo feature.
    - Prompt knobs: `retrieve_guidance` / `ccr_marker_hint` /
      `catalog_intent_hints` wired; caps `APHRODITE_RECURSIVE_DEPTH` +
      `APHRODITE_MAX_REQUEST_BODY_SIZE`.
    - New shipped defaults: `tool_threshold_token` 256, `tool_threshold_cache`
      2048, `terminal_threshold` 256, `inline_threshold` 1024,
      `code_multiplier` 1.0, `engine_threshold_pct` 30, `preview_max_chars` 0,
      `auto_reload` true, directives `[lazy, lazy-eval, foresight]`.

### Verification

- Fork range analyzed: `aphrodite-v0.10.0..HEAD` on `vendor/headroom` = 2
  commits (`fc58ee39`, `41e3602b`), both `Cargo.toml` dependency updates.
- Diffstat: 2 files changed (`Cargo.toml` in both commits), dependency pins
  only - no source, docs, or workflow changes.

### What Ships

- No fork artifact in this cycle: `aphrodite-headroom-core` stays at 0.1.3 on
  crates.io; fork tag `aphrodite-v0.10.0` unchanged. The main {VERSION}
  release ships the 4-target binary matrix (see `vNEXT-draft.md`).

### Links

- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **Fork tag compare**: https://github.com/PlayForm/Headroom/compare/aphrodite-v0.9.4...aphrodite-v0.10.0
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core
