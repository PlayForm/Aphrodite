# aphrodite-bench-compression

Standalone micro-benchmark + crash-regression crate for the aphrodite
compression pipeline. Migrated and adapted from the archived
`.bench/compression/` crate; **not a member of the root workspace** (own
empty `[workspace]` table) so `cargo` commands run here never touch the
product workspace's `Cargo.lock` or member set.

Targets **aphrodite 1.4.3** (`crates/aphrodite`) via a path dependency, plus
the vendored `aphrodite-headroom-core` (`vendor/headroom/crates/headroom-core`)
for `compute_key`. Corpus fixtures are resolved relative to
`bench/corpus/` (the sibling fixture twin set).

## Run instructions

All commands run from this directory (`bench/compression/`):

```sh
cargo build            # compile crate + path deps (aphrodite rlib, headroom-core)
cargo test             # crash-regression tests over the pathological corpus
cargo bench -- --quick # criterion stage benches, reduced sampling (recorded baseline)
cargo bench            # full criterion run (longer)
```

> Note: this crate's `[workspace]` opts it out of the root workspace -
> `cargo build` at the repo root does **not** build this crate, and CI does
> not reference it. Build/test/bench here only.

## Stage benches (`benches/stages.rs`, criterion group `stages`)

| Group             | Measures                                                                  |
|-------------------|---------------------------------------------------------------------------|
| `classify`        | `detect_type` content-type detection over every corpus fixture            |
| `stage2`          | `stage2::compress_stage2` semantic reduction per reducible type (log, patch, json, rust, text control) |
| `struct_extract`  | `struct_extract::extract_code_structure` per language (rust/python/go/ts) |
| `marker`          | `marker::ccr_marker` generation, `marker::parse_preview`, `marker::extract_hashes` on a 10 kB marker-shaped doc |
| `resolve`         | `resolve::expand` over nested marker chains, 1-5 levels deep               |
| `store`           | `compute_key` hashing, inline-store `put`/`get` (64-entry LRU scan on `get`) for 1 kB/100 kB/500 kB text |
| `pathological`    | full stage set (`run_all_stages`) over the crash-regression fixtures - a panic anywhere aborts the run |

`run_all_stages` (in `src/lib.rs`) exercises the non-proxy pipeline: type
detection → preview build → stage-2 compression → code-structure extraction →
hash extraction → key computation → marker generation → preview parse, and
returns a fingerprint of every stage's output.

## Crash-regression classes (`tests/crash_regression.rs`)

The three pathological fixtures in `bench/corpus/` each target a historical
failure class:

| Fixture               | Class    | Failure mode guarded against                                        |
|-----------------------|----------|---------------------------------------------------------------------|
| `utf8_boundary.txt`   | wave-1   | UTF-8 slice panic: 500-byte chunk offsets landing mid-codepoint (19 of 37 offsets straddle a boundary) |
| `marker_literal.txt`  | 05-F1    | Literal-marker corruption: stored text merely *containing* `<<<CCR:...>>>` shapes must round-trip byte-identical through recursive expansion |
| `interior_nul.json`   | 03-F3    | Interior NULs: the raw inline-store path must hold/return NUL-bearing content unmodified |

Tests:
1. `no_stage_panics_on_any_corpus_fixture` - full stage set over **every**
   corpus fixture under `catch_unwind`.
2. `pathological_fixtures_are_present_and_flagged` - the three classes are
   present and flagged pathological.
3. `utf8_boundary_fixture_straddles_500_byte_marks` - pins the fixture's
   "teeth" (offsets really land mid-codepoint).
4. `expand_preserves_literal_markers_byte_identical` - 05-F1 round-trip.
5. `interior_nul_content_round_trips_through_inline_store` - 03-F3 round-trip.
6. `filter_lines_survives_pathological_corpus` - `resolve::filter_lines`
   survives all pathological fixtures.

## Adaptation notes (archive → this copy)

- **Location**: migrated from the archived `.bench/compression/` crate
  (read-only archive) → `bench/compression/` on the Development branch of
  the PlayForm/Aphrodite repo.
- **API**: verified against aphrodite **1.4.3** `crates/aphrodite/src` - all
  consumed entry points are unchanged from the archive version:
  `detect_type`/`build_preview` (re-exported from `preview` at crate root),
  `stage2::compress_stage2(content, &ty) -> Option<String>`,
  `struct_extract::extract_code_structure(content, language) -> HashMap`,
  `marker::ccr_marker(hash, ty, size, preview, headroom_budget, meta, center)`,
  `marker::parse_preview(line) -> Option<String>`,
  `marker::extract_hashes(text) -> Vec<String>`,
  `state::AphroditeState` + `inline_store_put/get`,
  `resolve::{expand, resolve_one, filter_lines}`,
  `headroom-core::ccr::compute_key(&[u8]) -> String`.
  No source-level adaptation was required; the crate compiles against the
  current rlib as-is.
- **Corpus**: fixtures now resolve to the repo's `bench/corpus/` twin set
  (identical names; `../corpus` relative to `CARGO_MANIFEST_DIR`), instead of
  the archive's `.bench/corpus/`. Verified: all 18 fixtures UTF-8-clean;
  `utf8_boundary.txt` straddles 19/37 500-byte offsets;
  `interior_nul.json` carries 2 NUL bytes; `marker_literal.txt` contains
  CCR-shaped text (10.1 kB).
- **Corpus loader**: the twin corpus also contains `README.md`,
  `verify_labels.py`, and a `__pycache__/` directory (left by
  `gen_corpus.py`); `src/lib.rs` now skips non-fixture files and any
  subdirectory so the corpus load stays fixture-only (the archive loader
  only skipped `gen_corpus.py`).
- **Manifest**: package renamed `aphrodite-bench-compression` (same as
  archive), own empty `[workspace]`, path deps `../../crates/aphrodite` +
  `../../vendor/headroom/crates/headroom-core` (package
  `aphrodite-headroom-core`), `serde_json` feature set replicated from the
  root workspace (`preserve_order`, `arbitrary_precision`, `raw_value`) for
  feature unification with headroom-core's smart-crusher anchor matching.
- **Anonymization**: no personal paths/IDs anywhere; all corpus references
  are relative.
- **Constraints honored**: archive untouched; `crates/aphrodite/**`, root
  `Cargo.toml`, and `vendor/**` not modified. Nothing committed or pushed -
  note: the repo auto-committer may sweep untracked files, so this crate may
  be auto-committed to the Development branch by an external process, not by
  this migration.