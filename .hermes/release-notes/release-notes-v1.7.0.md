**[Compare Aphrodite/v1.6.6...Aphrodite/v1.7.0](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.6...Aphrodite/v1.7.0)**

## Aphrodite 1.7.0 💋 Plugin v2.3.0

### Summary

1.7.0 completes the pre-Rust feature port: every feature that lived only
in the pre-Rust prototype now runs in the Rust engine - TOML templates,
auto-expand, the load-bearing context engine, live containers, the Rhai
scripting engine, and the prompt knobs, plus the recursion-depth and
request-body caps. The shipped config moves to the priced-retrieval posture
(lower thresholds, `preview_max_chars 0`, `[lazy, lazy-eval, foresight]`
directives) so the defaults match the doctrine the docs teach.

### Changes

- **Feature (TOML templates)**: `[templates.preview.*]` / `[templates.marker]` /
  `[templates.prompts]` / `[templates.reverse]` are now real configuration -
  per-content-type format strings with a documented variable set (`{type}`,
  `{ln}`, `{size}`, `{fns}`, `{sigs}`, ...), resolved per type as
  `family[ctype] → reverse[ctype] → family[_default] → hardcoded builder`; the
  family is selected by `[previews] model_family` (default `"code_first"`).
  Unknown variables degrade to empty; absent blocks fall back to the
  hardcoded builders.
- **Feature (auto-expand)**: `[compression] auto_expand` / `auto_expand_limit`
  are wired - just-stored tool output resolves inline (byte-capped, `0` =
  unlimited) instead of emitting a CCR marker; the knob is hot-reloadable
  with the threshold set.
- **Feature (context engine)**: `engine_threshold_pct` /
  `engine_protect_first` / `engine_protect_last` / `engine_min_msgs` are
  load-bearing - the engine archives the middle turns into one compact CCR
  entry and replaces them with a single marker (head + marker + tail), then
  injects the rendered `engine_offload` prompt; the archive is in-place but
  recoverable via `aphrodite_retrieve`. `0` = never, `100+` = off.
- **Feature (live containers)**: `APHRODITE_LIVE_CONTAINER` streaming mode -
  large tool results get instant CCR markers with the `live` type and
  progressive chunked retrieval with bytes-so-far / ETA progress.
- **Feature (scripting)**: the `scripting` Cargo feature ports the pre-Rust
  Rhai micro-script engine.
- **Feature (prompt knobs)**: `retrieve_guidance` / `ccr_marker_hint` /
  `catalog_intent_hints` are wired - the per-turn `[retrieval: ...]` guidance
  line, the marker template's `{hint}`, and catalog intent hints; the marker
  format/hint re-applies on `/reload`.
- **Feature (caps)**: `APHRODITE_RECURSIVE_DEPTH` (nested marker expansion
  cap, default 5) and `APHRODITE_MAX_REQUEST_BODY_SIZE` (optional request-body
  cap; over-size bodies rejected with 413) are wired through resolve and the
  proxy.
- **Feature (debug)**: `APHRODITE_DEBUG` env-gated structured debug logger.
- **Fix (hermes bridge, shared-gateway lock wedge)**: the dylib
  (`crates/aphrodite-hermes`) now keeps a per-session state registry with
  bounded lock acquisition and fail-open degrade, so multi-session gateways
  (parent + subagents) no longer contend on one global state lock.
- **Chore (new shipped defaults)**: `tool_threshold_token` 256,
  `tool_threshold_cache` 2048, `terminal_threshold` 256, `inline_threshold`
  1024, `code_multiplier` 1.0, `engine_threshold_pct` 30, `preview_max_chars`
  0 (unlimited), `auto_reload` true, and `[directives] active =
["lazy", "lazy-eval", "foresight"]` - the priced-retrieval doctrine as the
  shipped posture.
- **Docs**: config, context-engine, and architecture docs refreshed for the
  ported keys; the site config surface updated to the new defaults.

### Infrastructure

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes` ✅ (host
  target aarch64-apple-darwin; binaries at `target/release/`)
- Check: `cargo check -p aphrodite -p aphrodite-hermes` ✅ (0 errors);
  `cargo clippy --lib -D warnings` ✅ clean
- Tests: `cargo test -p aphrodite --lib` ✅ 498 passed / 0 failed / 1
  ignored; `cargo test -p aphrodite-hermes --lib` ✅ 59 passed / 0 failed
- Lint/format: `cargo +nightly-2026-05-01 fmt --all -- --check` ✅ clean;
  `npx prettier --check` on owned docs ✅ clean; FFI contract ✅ 0
  violations / 0 warnings; drift-guard (plugin shim vs
  `crates/aphrodite/templates/__init__.py`) byte-identical ✅
- Version track: crates + pin + `package.json` 1.7.0; `plugin.yaml` 2.3.0;
  `BINARY_VERSION` bumped LAST, after the release assets exist; README
  badges v1.7.0 / v2.3.0

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts; `Finalize` enforces all 12).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.3.0                                                        | Hermes (standalone repo)                 |

`aphrodite-headroom-core` stays at 0.1.3; the fork holds 2 dependency-update
commits post-tag (fastembed→5, crate version refresh) - dep-only delta, no
fork version bump per the 1.6.4 precedent.

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.6...Aphrodite/v1.7.0
- **CHANGELOG.md**: [CHANGELOG.md](https://github.com/PlayForm/Aphrodite/blob/Current/CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
