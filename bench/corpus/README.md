# Aphrodite `bench/corpus/` - CCR content-type coverage corpus

Deterministic, anonymized, current-era fixture corpus for the Aphrodite CCR
(content compression & retrieval) engine. It exists to **exercise content-type
coverage** of the headroom `content_detector` labels the binary routes on
(`json_array`, `source_code`, `search`, `build`, `diff`, `html`, `tabular`,
`structured_config`, `text`) plus a set of **pathological inputs** that regress
specific past crash classes. The fixtures are **synthetic samples of content
types - not benchmark measurements**: no ratio, latency, or savings numbers are
claimed anywhere in this directory (measurement happens in
`Maintain/scripts/bench/`, which consumes corpora like this one).

## Layout

| File | Role |
|---|---|
| `*.txt` / `*.json` / `*.rs` / `*.go` / `*.py` / `*.patch` | 15 fixture files (table below) |
| `gen_corpus.py` | Deterministic generator (seed `0xCC12`); rewrites every fixture in place |
| `metadata.json` | Schema `aphrodite-bench-corpus-metadata/1`: intended label + size + description per fixture; the authoritative label source |
| `verify_labels.py` | Verification: runs the real vendored detector over every fixture and compares detected vs intended label |

## Fixtures and intended labels

Intended labels come from `metadata.json` (the `detector_vocabulary` there is
the full detector label set). `†` marks the three pathological fixtures.

| Fixture | Intended label | Size | Description |
|---|---|---|---|
| `build_log.txt` | `build` | 30,982 B | Modern cargo build log (aphrodite v1.4.3 workspace, rustc 1.88): errors, warnings, failure summary, then a passing `cargo test` run |
| `code_go.go` | `source_code` | 10,626 B | Idiomatic modern Go (structs, error returns) for `struct_extract` benches |
| `code_python.py` | `source_code` | 22,538 B | Modern Python (annotations, classes) for `struct_extract` benches |
| `code_rust.rs` | `source_code` | 2,673 B | Rust payload copied from `crates/aphrodite/examples/bench_payloads/sample.rs` |
| `code_ts.ts` | `source_code` | 20,310 B | Modern TypeScript (interfaces, async/await, generics) for `struct_extract` benches |
| `deep_nested_20.json` | `json_array` | 2,215 B | JSON object nested 20 levels deep; stresses recursive parsing |
| `git_diff.patch` | `diff` | 11,024 B | Unified git diff touching current `crates/aphrodite/src/*.rs` paths |
| `interior_nul.json` † | `json_array` | 4,201 B | Valid JSON whose decoded strings contain interior NUL bytes, plus raw `0x00` trailing bytes |
| `marker_literal.txt` † | `text` | 10,100 B | Literal CCR marker-syntax documentation; must pass through byte-identical |
| `text_1kb.txt` | `text` | 1,024 B | 1 KiB deterministic plain-text blob |
| `text_10kb.txt` | `text` | 10,240 B | 10 KiB deterministic plain-text blob |
| `text_100kb.txt` | `text` | 102,400 B | 100 KiB deterministic plain-text blob |
| `text_500kb.txt` | `text` | 512,000 B | 500 KiB deterministic plain-text blob |
| `utf8_boundary.txt` † | `text` | 18,808 B | Pathological UTF-8: multibyte runs straddling 500-byte boundaries |
| `wide_5k_keys.json` | `json_array` | 219,037 B | JSON object with 5,000 top-level keys; stresses wide-map handling |
| `gen_corpus.py` | `source_code` | 14,892 B | The generator itself (listed in metadata; classified as source code) |

> **Measured behavior (verify_labels.py, vendored detector):** all three
> object-form JSON fixtures (`deep_nested_20.json`, `interior_nul.json`,
> `wide_5k_keys.json`) detect as `json_array` (conf 0.90) - the vendored
> Python detector identifies JSON by **parsing** (objects, arrays, and any
> nesting), not by a leading-`[` check. The one real mismatch in the corpus is
> `code_go.go`: it detects as `build` (conf 1.00) instead of `source_code`,
> because the case-insensitive `\bERROR|FAIL|...\b` log pattern matches the
> `error` in Go's `(string, error)` return idiom on nearly every function and
> log detection runs **before** code detection. That is a genuine classifier
> blind spot, not a fixture bug - this fixture exists to keep it visible.

## Pathological fixtures - crash-regression significance

Each pathological fixture is a regression class that has caused real crashes or
corruption in the CCR pipeline; the corpus keeps them so any future change that
re-introduces the class fails verification loudly.

### `utf8_boundary.txt` - multibyte straddling offsets (slice-panic class)

Built as `("a" + "€"*700 + "b" + "🜲"*500 + "é"*300) * 4`, so 2-, 3-, and 4-byte
codepoints deliberately **straddle byte offsets 500 / 1000 / 1500 / …**.
Any byte-indexed slicing (`&content[..n]`, `content[..500]`-style chunking, or
`len()`-based offset arithmetic) that lands mid-codepoint panics
(`byte index N is not a char boundary`) or corrupts output. The Rust
`content_detector.rs` HTML sampler already handles this via `is_char_boundary`;
this fixture guards the rest of the pipeline (compressor chunking, preview,
retrieval) against the same class.

### `marker_literal.txt` - embedded CCR markers (05-F1 class)

Contains literal marker syntax inside ordinary prose: full
`<<<CCR:40-hex|type|size>>>`, the short `[CCR:hex|type]` form, a Unicode glyph
form, and edge cases (`<<<CCR:` unclosed, `<<<CCR:>>>` empty, a nested prefix).
None of these hashes exist in any store. The regression significance: marker
*parsing/expansion* must never rewrite, substitute, or corrupt content that only
*mentions* marker syntax - and retrieval of a marker-shaped-but-nonexistent
hash must fail cleanly, not panic. Any component that treats the text as a
retrievable marker corrupts documentation; any component that panics on the
malformed edge cases reintroduces the 05-F1 crash class.

### `interior_nul.json` - interior NULs (03-F3 class)

Valid JSON whose decoded strings embed `\x00` characters (`"nul\x00inside"`,
200 rows of `row_N\x00tail`), plus raw `0x00` bytes appended after the document
(still valid UTF-8 - NUL is a legal codepoint). Regression significance:
NUL-terminated or `len()`-as-terminator logic (C-style string handling, byte
counting, truncation) trips on interior NULs - content gets silently truncated,
lengths disagree with the byte stream, or the parser rejects valid payloads.
Length-based decisions must treat NUL as an ordinary byte.

## Verification - `verify_labels.py`

```sh
python3 verify_labels.py          # table: detected vs intended per fixture + summary
python3 verify_labels.py --quiet  # table only
```

For every fixture it loads the **real classifier** the project uses - the
vendored pure-python headroom detector at
`vendor/headroom/headroom/transforms/content_detector.py`, imported via the
exact `sys.path.insert` path that
`Maintain/scripts/bench/benchmark-eval.py::classify` (used by
`benchmark-report.py`) uses - and prints one row per fixture:

```
fixture                  intended         detected         status
build_log.txt            build            build            OK
...
# summary: N match, M mismatch/error across K fixtures
```

- `OK` - detected equals intended.
- `MISMATCH` - detected differs from intended (currently: `code_go.go`
  detected as `build` - see the measured-behavior note in the fixture table).
- `ERROR` - the fixture could not be read/decoded, or the detector raised on
  it (a pathological input crashing the detector is a finding, not a script
  crash - the script always continues).
- If the detector is **not importable** (moved, deleted, or broken), the script
  prints `DETECTOR_UNAVAILABLE` with the reason and falls back to the raw
  first-line heuristic classifier mirrored from `benchmark-eval.py`'s fallback
  (same label vocabulary) - flagged so heuristic results are never mistaken for
  real classifier output.
- `metadata.json` missing → detect-only mode (intended shown as `unknown`).
- Exit code is `0` on completion even with mismatches: the script is a
  measurement, not a gate. It never writes and never commits.

## Regeneration - `gen_corpus.py`

```sh
python3 gen_corpus.py     # run from anywhere; rewrites fixtures next to the script
```

Deterministic (seeded `0xCC12`), so regeneration is byte-stable and diffs are
reviewable. `code_rust.rs` copies the live bench-payload example at
`crates/aphrodite/examples/bench_payloads/sample.rs` (falls back to the
canonical archive-era embedded copy if the example is absent). `interior_nul.json`
is written in binary mode on purpose. After regenerating, re-run
`verify_labels.py` to confirm labels still hold.

## Current verification status

Last run: `verify_labels.py` against the vendored headroom detector
(`vendor/headroom/headroom/transforms/content_detector.py`) - **15 of 16
labels match**. Detected vs intended per fixture:

| Fixture | Intended | Detected | Status |
|---|---|---|---|
| `build_log.txt` | `build` | `build` (1.00) | OK |
| `code_go.go` | `source_code` | `build` (1.00) | MISMATCH |
| `code_python.py` | `source_code` | `source_code` (1.00) | OK |
| `code_rust.rs` | `source_code` | `source_code` (0.89) | OK |
| `code_ts.ts` | `source_code` | `source_code` (1.00) | OK |
| `deep_nested_20.json` | `json_array` | `json_array` (0.90) | OK |
| `gen_corpus.py` | `source_code` | `source_code` (0.60) | OK |
| `git_diff.patch` | `diff` | `diff` (1.00) | OK |
| `interior_nul.json` | `json_array` | `json_array` (0.90) | OK |
| `marker_literal.txt` | `text` | `text` (0.50) | OK |
| `text_1kb.txt` / `text_10kb.txt` / `text_100kb.txt` / `text_500kb.txt` | `text` | `text` (0.50) | OK |
| `utf8_boundary.txt` | `text` | `text` (0.50) | OK |
| `wide_5k_keys.json` | `json_array` | `json_array` (0.90) | OK |

None of the three pathological fixtures crashes or misroutes: `utf8_boundary.txt`
and `marker_literal.txt` land in `text`, and `interior_nul.json` is still parsed
as JSON - NUL bytes are ordinary bytes to the detector. Re-run
`verify_labels.py` after any detector or corpus change to refresh this table.

## Anonymization / adaptation policy

- **Synthetic only, no personal identifiers**: no usernames, emails, tokens,
  secrets, API keys, or real machine-specific paths anywhere in the corpus.
- All paths are workspace-relative (`crates/aphrodite/src/...`) or generic
  placeholders (`/home/user/...`); the build log and diff reference **current**
  Development-branch file names and crate versions (aphrodite v1.4.3, rustc
  1.88).
- Content is **current-era**: modern Python annotations, TS async/generics,
  Go error returns - not legacy idioms - so compression behavior reflects
  today's agent tool output.
- Deterministic generation (fixed seed) keeps the corpus auditable: any
  accidental identifier or machine-specific path would be visible as a diff on
  regeneration and in review.