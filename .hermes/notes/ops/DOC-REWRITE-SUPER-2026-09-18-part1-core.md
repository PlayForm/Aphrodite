# DOC-REWRITE-SUPER-2026-09-18 - part 1 (core crate) findings

Task: re-check and rewrite every code comment in the Aphrodite core Rust crate (branch
Development), comment-only edits, per S3 instructions. Scope: all 21 files in
`crates/aphrodite/src/*.rs`, `crates/aphrodite/build.rs`, the 5 `bench_*` examples, and the 6
`builtin_directives/*.md` runtime-directive prose files (conservative). Repo root:
`/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`.

Repo state observed during the run: branch `Development`; HEAD moved mid-session from `66d8fa1`
to `2f805c0d` (a concurrent coordinator commit captured this run's edits - see "Repository
state note" below). Binary version verified on the compile line: `aphrodite v1.4.6`. Toolchain
`nightly-2026-05-01`. Working tree clean at start for the in-scope files.

## (a) Per-file rewrite summary and files-touched count

All 33 in-scope files were read in full. **4 files were edited (5 comment patches total).**
The remaining 29 files were verified accurate against the 1.4.6 code - no comment changes
needed. Terminology consistency checked across every file: CCR / `<<<CCR:hash|type|size>>>`
markers, retrieve-first rule, chain-split teaching loop (Tier 1 = consequence attribution on
successful segment resolve; Tier 3 = per-segment error hints), honest previews (Issue #11),
`preview_max_chars` wiring (env > TOML > default 120, absent key = unlimited). No stale
versions, dates, or test counts were found in any comment (test-fixture strings like
`Compiling aphrodite v0.5.0` / `v1.3.3` / `v1.4.3` inside bench/preview fixtures are code data,
not comments - left untouched).

Edits made:

1. `crates/aphrodite/src/directives.rs` (1 patch)
    - L80-81: the `load_directives` doc listed the builtin-fallback set as `focus`,
      `foresight`, `ccr-handling`, `cleanup`, `explore`, and `lazy` - omitting `lazy-eval`.
      `builtin_directives()` embeds 7 files (the 7 `.md` files, verified); the list is now
      complete. (Same fix as the correct 7-name list already present at L18-19.)

2. `crates/aphrodite/src/marker.rs` (1 patch)
    - L142: doc said the HASH_RE matches "all four marker delimiter families" but the regex
      (L160-162) has three opener forms (`<<<`, `[`, U+2AF7) / three closers (`>>>`, `]`,
      U+2AF8), and the doc itself enumerates three families. Corrected to "all three".
    - Note: the same file also carries 3 em-dash→hyphen comment normalizations (L48, L433,
      L448) that are in HEAD `2f805c0d` but were not made by this run (see repository-state
      note below). Comment-only, in scope, harmless.

3. `crates/aphrodite/src/state.rs` (1 patch)
    - L113-117: `chain_split_enabled` doc said "Default true" with no nuance. The struct
      `Default` is indeed `true`, but `config_loader::apply_compression` resolves the shipped
      config default to `false` (`get_bool("APHRODITE_CHAIN_SPLIT", "compression",
"chain_split", false)` - "Default OFF for release"). Clarified the comment to state both
      levels (struct default true; shipped config default false, opt-in per session). This is
      the documented product behavior (config key `chain_split`, default false).

4. `crates/aphrodite/src/config_loader.rs` (2 patches)
    - L277-281: the active-directive seeding comment claimed the `focus + foresight + lazy`
      default applies only "if the TOML list is empty AND we fell back to builtins". The code
      (L284-289) seeds whenever the TOML `active` list resolves empty AND any directives are
      loaded, from disk or builtins (`active_directives.is_empty() && !directives.is_empty()`).
      Reworded to match the code.
    - L426-427: test comment claimed "the fallback seeds focus + foresight as defaults"; the
      temp dir for that test only contains `focus.md`, so only `focus` is seeded (the code
      pushes whichever of `[focus, foresight, lazy]` exists in the loaded set). Corrected.

Unchanged after full review (accurate as-is): `chain_split.rs`, `setup.rs`, `poll_worker.rs`,
`prefetch.rs`, `retrieve.rs`, `stage2.rs`, `main.rs`, `hooks.rs`, `catalog.rs`, `preview.rs`,
`lib.rs`, `config.rs`, `resolve.rs`, `struct_extract.rs`, `proxy.rs`, `session.rs`, `flow.rs`,
`build.rs`, all 5 `bench_*` examples, and all 6 `builtin_directives/*.md` files. The 6 `.md`
files are runtime directive prose shipped with the binary; conservative review found no
factual/terminology errors, stale versions, numbers, or paths - no edits (meaning, tone, and
structure preserved).

## (b) Code-vs-doc mismatches recorded (fix would need a code change - code NOT changed)

None found. The closest candidate was `state.rs` `chain_split_enabled` struct default `true`
vs the shipped config default `false`; that is intentional layering (struct default is always
overridden by `apply_compression`), so it was resolved as a comment clarification, not a code
mismatch. No comment anywhere in scope contradicts behavior the code actually has; the
"four vs three delimiter families" and the seeding-comment issues were comment-only errors
and were fixed as comments (section (a)).

## (c) Stale items fixed

- `directives.rs` L80-81: incomplete builtin-directive fallback list (missing `lazy-eval`).
- `marker.rs` L142: wrong delimiter-family count ("four" → "three").
- `config_loader.rs` L277-281: seeding comment described an older builtins-only fallback path.
- `config_loader.rs` L426-427: test comment mis-stated which directives get seeded (only
  `focus` exists in that fixture).
- No stale version numbers, dates, or test counts were found in comments (the 2026-09-18
  quality-gate numbers - 406 passed / 52 passed / 23-23 / 13-13 - are not cited in any
  in-scope comment).

## (d) Lints left as findings (code changes would be required - NOT fixed, out of scope)

1. `crates/aphrodite` clippy status: `cargo clippy -p aphrodite -- -D warnings` fails to
   compile because of a pre-existing `question_mark` lint ("this block may be rewritten with
   the `?` operator") in the vendored dependency `aphrodite-headroom-core`
   (`vendor/headroom/crates/headroom-core`, around its `get`/expiry block, lines ~212-216).
   The lint is in `vendor/**` - explicitly out of this task's scope and on the never-touch
   list. The aphrodite crate itself produced zero clippy findings; the required verification
   (`fmt --check`, `cargo check -p aphrodite`) passes. A clean `-D warnings` gate therefore
   requires excluding/exempting the vendor crate (e.g. `--workspace --exclude` or a targeted
   `#[allow]` in vendor code) - a code change, recorded here per rule 4.
2. `clippy::empty_line_after_doc_comments`: no violations found. The 26
   doc-comment-then-blank-line patterns in scope are all module-level `//!` docs followed by a
   blank line before `use` imports - idiomatic Rust, not flagged by the lint (the 2026-09-18
   quality gate ran clippy `-D warnings` clean on the aphrodite crate).
3. Pre-existing code-idiom lints inside scope that are not comment-fixable and were NOT
   touched (per rule 4): none were surfaced by this run's clippy pass on the aphrodite crate
   (the crate compiled with zero clippy output before the vendor failure aborted the build).
   `lib.rs`'s `#![allow(clippy::not_unsafe_ptr_arg_deref)]` is intentional and documented
   in-file (FFI boundary report pending).

## Repository state note (for the coordinator)

- HEAD moved from `66d8fa1` to `2f805c0d` during this run, via a commit made by the
  coordinator/concurrent process (this run made no `git` writes - no commit, no stage, no
  push). The commit's diff within this task's scope is exactly: the 5 comment patches listed
  in section (a) plus 3 em-dash→hyphen comment normalizations in `marker.rs` (L48, L433,
  L448) that this run did not author (likely a parallel/previous pass on the shared tree).
  Nothing else in scope changed (`git diff 66d8fa1..HEAD -- crates/aphrodite/src/ build.rs
examples/` shows only the 4 files above). Blob hashes of all 4 edited files on disk equal
  HEAD's - the working tree is in sync with the commit.
- `git status` at write time shows one unrelated in-flight modification,
  `crates/aphrodite-hermes/src/lib.rs` (` M`), not touched by this run (out of scope).

## Verification (real outputs)

- `rustup run nightly-2026-05-01 cargo fmt --all -- --check` → exit 0 (clean).
- `rustup run nightly-2026-05-01 cargo check -p aphrodite` → exit 0
  (`Finished dev profile [unoptimized + debuginfo] target(s) in 13.87s`; compile line shows
  `Checking aphrodite v1.4.6`).
- `rustup run nightly-2026-05-01 cargo clippy -p aphrodite -- -D warnings` → exit 101, blocked
  by the pre-existing vendor-crate `question_mark` lint (see (d)1); zero findings from the
  aphrodite crate.
- Prettier: no `.md` files were edited (the 6 builtin directives were verified and left
  untouched); this findings file is written prettier-clean (tabs, width 100, proseWrap
  preserve) and checked with `npx prettier --check`.
