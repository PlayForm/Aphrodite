# DOC-REWRITE-SUPER-2026-09-18 - part 2: aphrodite-hermes bridge + crate docs (S3)

Scope: `crates/aphrodite-hermes/src/{tools,directives,schemas,lib}.rs`,
`crates/aphrodite-hermes/src/bin/setup-helper.rs`, `crates/aphrodite-hermes/build.rs`,
`crates/aphrodite/README.md`, `crates/aphrodite-hermes/README.md`,
`aphrodite.toml.example` (comments only). Branch Development, HEAD 66d8fa1.
All edits are comment/doc-only; zero behavior change. Nothing committed or staged.

## (a) Per-file rewrite summary - 6 files touched

1. **crates/aphrodite-hermes/src/lib.rs** - module doc: dropped the stale
   "skill registration" claim (the bridge crate has no skills module; the Python
   plugin registers skills - `plugins/aphrodite/__init__.py::register_skill`),
   fixed the malformed diagram (two `└─` glyphs on sibling branches), added
   `chain_split` to the core-crate dependency list. Extended the Issue #11 WS4
   comment with the `preview_max_chars` precedence (env > TOML > default 120;
   absent key = unlimited). Fixed a `doc_lazy_continuation` clippy lint in the
   `aphrodite_hermes_materialize_directives` doc comment (continuation line of
   the `- always` list item now indented).
2. **crates/aphrodite-hermes/src/tools.rs** - `unwrap_hermes_result` doc:
   "sees '{' and returns json_array" was stale - headroom classifies `{` JSON
   objects as `text`; `json_array` only for `[` array shapes (confirmed by the
   in-repo ISSUE-11 test comment and `preview.rs::detect_type` wrapping the
   headroom classifier).
3. **crates/aphrodite-hermes/src/directives.rs** - module doc: "the binary is
   the provider" → "the core crate is the provider (the embedded set ships
   inside this dylib)"; the builtin set lives in the core crate
   (`builtin_directives/*.md`), not in the binary.
4. **crates/aphrodite-hermes/src/schemas.rs** - read fully; module doc and test
   comments verified accurate against handlers; no changes needed.
5. **crates/aphrodite-hermes/src/bin/setup-helper.rs** - read fully; module doc
   accurate (cargo install ships only the bin target; dylib via `aphrodite
setup` or full build); no changes needed.
6. **crates/aphrodite-hermes/build.rs** - read fully; FFI-codegen contract,
   graceful-degradation and copy-on-change comments verified against the build
   script; no changes needed.
7. **crates/aphrodite/README.md** - intro: `libaphrodite.dylib` is NOT loaded
   by the Hermes plugin (verified: plugin dlopens only `libaphrodite_hermes`
    - `__init__.py` and `setup.rs::copy_dylibs`, which treats the core dylib as
      best-effort "for external" consumers); rewrote the C ABI paragraph (core
      exports form the core C ABI; plugin `_bindings.py` is generated from the
      bridge header, not from core exports); src tree gained `chain_split.rs`,
      `flow.rs`, `poll_worker.rs` (21 files now, matches `src/`); pipeline diagram
      "15 tok preview, not 500 tok raw" → honest-preview wording; Install adds the
      `aphrodite setup` bootstrap step.
8. **crates/aphrodite-hermes/README.md** - diagram: "Python **init**.py (145
   lines)" → "(thin loader)" (file is 1257 lines now); diagram bottom corrected
   (core engine is linked as rlib, no separate `libaphrodite.dylib` load);
   removed the stale "Skill registration" bullet and `skills.rs` from the src
   tree (crate has no skills module); tree now lists `directives.rs`,
   `bin/setup-helper.rs`, `build.rs`; hook count 5 → 6; Install rewritten to the
   real clone-and-setup flow: `cargo install --path crates/aphrodite` +
   `cargo build --release -p aphrodite-hermes` → `aphrodite setup` (runtime home
   `~/.hermes/aphrodite/`: binaries/, directives/, hotreload/, ccr.db) → manual
   plugin symlink (verified live on this machine:
   `~/.hermes/plugins/aphrodite -> <repo>/plugins/aphrodite`); Dependencies
   gains the cbindgen/ctypesgen FFI-codegen note.
9. **aphrodite.toml.example** - comments only, structure/keys/defaults
   untouched: `preview_max_chars` comment now documents env > TOML > default 120
   (absent key = unlimited); `chain_split` comment now names SEG_MARKER segments.

## (b) Code-vs-doc mismatches (fix would need a code change - NOT changed)

1. **crates/aphrodite-hermes/src/tools.rs:583** - `aphrodite_rebuild` hint
   string: `rebuild via cargo build --release -p aphrodite`. Wrong crate: the
   plugin loads `libaphrodite_hermes.dylib`, built by `-p aphrodite-hermes`;
   `-p aphrodite` builds the core binary/cdylib. Hint should be
   `cargo build --release -p aphrodite-hermes` (then re-run `aphrodite setup`
   to refresh `binaries/`). Behavioral string; left as-is.
2. **crates/aphrodite-hermes/src/schemas.rs - `schema_stats()` description** -
   the documented `Returns {...}` omits the four chain_split telemetry keys the
   handler actually emits (`chain_split_min_segments`, `chain_split_events`,
   `chain_split_produced`, `chain_split_retrieved` - tools.rs:365). Likely
   deliberate ("Tier 1 teaching loop telemetry ... never rendered into the LLM's
   conversational view", tools.rs comment) but the schema doc is incomplete
   relative to the handler. Model-facing string; left as-is.

## (c) Stale items fixed

- "Skill registration" as a bridge-crate feature (lib.rs module doc, hermes
  README bullet + `skills.rs` in the src tree) - skills are registered by the
  Python plugin.
- `libaphrodite.dylib` described as the Hermes plugin's loaded dylib (both
  READMEs) - plugin loads only `libaphrodite_hermes.dylib`.
- "Python **init**.py (145 lines)" - file is 1257 lines; replaced with a
  non-numeric "(thin loader)".
- "5 hooks" → 6 hooks (hermes README src tree; `aphrodite_hermes_get_hooks`
  returns 6 names).
- Core src tree missing `chain_split.rs`, `flow.rs`, `poll_worker.rs`.
- "sees '{' and returns json_array" (tools.rs doc) - objects classify as text.
- "15 tok preview, not 500 tok raw" (core README diagram) - stale preview
  marketing; honest-preview wording.
- `preview_max_chars` / `chain_split` example comments - now carry precedence /
  SEG_MARKER detail.

## (d) Lints left as findings

1. **crates/aphrodite-hermes/src/tools.rs:613** - `clippy::type_complexity`
   ("very complex type used") on the test table type
   `Vec<(&str, serde_json::Value, Option<(&str, &str)>)>` in
   `test_unwrap_hermes_result_table`. Test-code lint, only fires with
   `--all-targets`; fix needs a type alias (code change) - left as-is.
2. **Pre-existing, out of scope** - plain `cargo clippy -- -D warnings`
   (without `--no-deps`) fails compiling the vendored `aphrodite-headroom-core`
   under clippy; unrelated to this task's comment edits (verified with
   `--no-deps`). The `empty_line_after_doc_comments` lint: regex scan of all 6
   in-scope .rs files returns zero hits - nothing to fix.

## Verification (real outputs)

- `rustup run nightly-2026-05-01 cargo fmt --all -- --check` → exit 0 (clean).
- `rustup run nightly-2026-05-01 cargo check -p aphrodite -p aphrodite-hermes`
  → exit 0 (Finished dev profile; only pre-existing Cargo.toml semver-metadata
  warning and the normal "header unchanged" build.rs skip warning).
- `rustup run nightly-2026-05-01 cargo clippy -p aphrodite-hermes --no-deps -- -D warnings`
  → exit 0 (lib target clean, matching the quality-gate baseline).
- `node node_modules/.bin/prettier --check crates/aphrodite/README.md
crates/aphrodite-hermes/README.md` → all files use Prettier code style.
- `git diff --stat`: 6 files in scope (115 insertions / 81 deletions across the
  13-file working diff; the other 7 .hermes/*.md files in the diff are
  concurrent S3 subagent edits, untouched here).
