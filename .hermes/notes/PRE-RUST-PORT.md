# PRE-RUST PORT - re-implement features lost in the Python→Rust migration

Goal: port the pre-Rust plugin feature set (lost in `18de22d9` unified-binary
rewrite) back into the Rust engine on Development. Implement only; never
commit (auto-committer sweeps). Test at the end (`cargo test -p aphrodite`,
clippy).

## Source of truth (read-only reference worktree)

`/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite-Pre-Rust`
(detached at `0f24056b`, parent of the migration commit; plugin submodule at
`f8d4aee` = PlayForm/Aphrodite-Hermes). Toggle commits there freely; never
edit it.

| Feature | Pre-Rust source | Lost surface | Status today |
| ------- | --------------- | ------------ | ------------ |
| TOML preview/marker/prompt templates | `_core/template.py` (`_render_template`, `_render_marker_tmpl`, `_render_prompt_tmpl`) | preview builders, marker format, prompt rendering | hardcoded per-arm; no `templates` reader |
| Auto-expand | `_resolve.py` (`_resolve_recursive`), `auto_expand` keys | resolve/proxy | key parses, no consumer |
| Context engine | `_engine.py` (`AphroditeContextEngine`, `should_compress`, `compress`) | session archiving | `engine_*` status-only |
| Live containers | `_hooks/live.py`, `_core/template.py` (`APHRODITE_LIVE_CONTAINER`) | streaming read_file/terminal via CCR | env var unwired |
| Prompt knobs | `_core/config.py` (`retrieve_guidance`, `ccr_marker_hint`, `catalog_intent_hints`) | flow rendering | parsed, no consumer |
| Rhai scripting | `crates/aphrodite/src/scripting.rs` (pre-Rust tree) | scripting module + Cargo feature | module deleted |
| center module | `crates/aphrodite/src/center.rs` (pre-Rust tree) | settings store/center | module deleted |
| Recursion depth / request-body caps | env vars (`APHRODITE_RECURSIVE_DEPTH`, `APHRODITE_MAX_REQUEST_BODY_SIZE`) | resolve + proxy | unwired, hardcoded |

## Wave plan (pairs of 2 agents per task, disjoint file ownership)

1. **Foundation (parent):** `TemplatesConfig` structs (preview/marker/prompts/
   reverse) in `config/proxy.rs` + `config_loader.rs` parse + example keys.
2. **Task 1 - templates pair: DONE (auto-committed c4f7079c + 1016fe6c, 2026-10-13).**
   A = preview template rendering (templates.rs NEW + preview/builders consumption) - 4 tests;
   B = marker block + prompt templates (marker/format.rs + flow.rs) - 22 tests, 465 total green.
   Parent integration: moved TemplatesConfig to state/mod.rs (ungated; the config module is
   feature-gated and broke the no-proxy aphrodite-hermes build); re-export kept at config.
3. **Task 2 - auto-expand pair: DONE (2026-10-13).**
   A = proxy-side inline resolution (proxy.rs + resolve/recursive.rs resolve_inline) - 10 tests;
   B = dylib/hook-side (hooks.rs + flow.rs auto_expand_guidance + hermes bridge auto_expanded arm) - 14 tests.
   487 total green; rustfmt clean. Parent integration gate pending re-run.
4. **Task 3 - context engine pair:** session/state archiving wired to
   `engine_*` keys (middle-message compression, head/tail protection).
4. **Task 3 - context engine pair: DONE (2026-10-13).**
   A = session.rs engine_maybe_archive (threshold gates, protect 2/5, middle JSON
   packed, BLAKE3 hash, inline store, engine_offload prompt) - 8 tests;
   B = flow.rs trigger + engine_offload injection - 4 tests. NOTE: engine now
   LIVE - with shipped defaults (threshold 30, enabled) it archives at 30% fill.
5. **Task 4 - live containers pair: DONE (2026-10-13).**
   A = prefetch.rs/poll_worker.rs container status + ETA (7 tests);
   B = hooks.rs/retrieve.rs instant <<<CCR:hash|live|size>>> marker + chunked
   progress retrieval (9 tests). 506 total green; default off = byte-identical.
6. **Task 5 - scripting + caps pair:** Rhai `scripting.rs` (restore from the
   pre-Rust worktree crates/aphrodite/src/scripting.rs + its Cargo feature,
   rhai dep ALLOWED pinned + optional) + recursion-depth/body-size config
   consumers. (center.rs NOT restored - deliberately dead per report 01-T6.)
7. **Task 6 - prompt knobs: DONE (absorbed by Task 1 sa-1):** retrieve_guidance,
   ccr_marker_hint, catalog_intent_hints wired with tests.

## Standing brief rules (verbatim in every child context)

- Repo root: `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`.
- Pre-Rust reference: `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/
  Aphrodite-Pre-Rust` (read-only - do NOT edit).
- House style: tabs, `rustfmt`, no semver-range deps, defensive non-happy-path
  (warn + degrade, never panic).
- Do NOT run `git commit`; an external auto-committer may run - ignore it and
  report it. Do NOT push, tag, or rebase.
- Ownership: touch ONLY the files named in your brief.
- Verify with `cargo check -p aphrodite` (or the named target) before
  reporting; a child's green claim is re-verified by the parent on disk.