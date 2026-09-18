# SPLIT-ADAPTATION - Crate Landscape, Declarative Detection, Reverse Taxonomy

**Scope:** research distillation of the split.md rewrite (regex-crate landscape,
declarative detector pipeline, reverse-taxonomy file tree) into the Aphrodite
detection layer's context. Original adaptation - nothing transcribed; the
source document stays out of the repo. Companion plan:
`ops/REFACTOR-PLAN-1.5.0.md` (the concrete 1.5.0 tree + per-swap steps).

**Status:** research record. No code touched, nothing committed.

---

## 1. Crate landscape - the per-dimension winners

The survey asked which Rust regex crate is "most advanced" for parsing
structured generated source / tool output. The answer depends on the dimension
being measured - no single crate wins them all:

| Dimension               | Winner                   | Why                                                                                                                                              |
| ----------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Feature completeness    | `fancy-regex`            | Full PCRE-class surface layered on the `regex` engine: lookaheads, lookbehinds, backreferences, atomic groups - everything Python's `re` can do. |
| Performance ceiling     | `regex` + `aho-corasick` | Linear-time DFA, SIMD `memchr` fast paths, purpose-built multi-pattern search.                                                                   |
| Compile-time patterns   | `lazy_regex`             | Patterns verified and compiled at `cargo build` via proc macro; zero runtime `unwrap()` on `Regex::new()`.                                       |
| Syntax power            | `onig`                   | Full Oniguruma: named captures, possessive quantifiers, Unicode properties, recursive patterns - at the cost of a C library.                     |
| Streaming / large input | `regex` `find_iter`      | Constant memory regardless of input size; `fancy-regex` can blow the stack on deep backtracking.                                                 |

### The verdict for Aphrodite

For this codebase the winning "crate" is the standard library, not any regex
engine. Detection classifies structured tool output - cargo `test result:`
lines, `diff --git` headers, git status codes, grep `path:line:match` rows -
whose shapes are known fixed strings. `starts_with` / `contains` / `bytes()`
cover every one of them, so the most advanced engine is the one never invoked.

- `onig` / `pcre2` are rejected on build-graph grounds: they link a native C
  library, which conflicts with the lean dylib goal of the plugin binary. Most
  feature-rich, wrong here for exactly that reason.
- `lazy_regex` is the only genuinely attractive upgrade (compile-time
  verification beats a `LazyLock` that could panic on first use) - and it
  becomes moot once detection stops using regexes entirely.
- `aho-corasick` is worth knowing about, not adopting: already a transitive
  dependency of `regex`, it can compile the four test markers into a
  single-pass automaton replacing six `.contains()` scans. Optional - for
  KB-sized blobs the architectural clarity of the pipeline matters more than
  the micro-optimization.

### Why no library does this

Every candidate operates in a different problem domain:

| Crate               | What it actually does        | Why it mismatches                            |
| ------------------- | ---------------------------- | -------------------------------------------- |
| `magika`            | AI-based file-type detection | Filesystem files, not in-memory string blobs |
| `mimetype-detector` | Magic bytes / file headers   | Binary sniffing, not semantic text shape     |
| `file-identify`     | Extensions + shebangs        | Filename metadata, not content lines         |
| `mime-sniffer`      | Chromium-style byte sniffer  | Binary content, not structured tool output   |
| `string-patterns`   | Regex sugar                  | Still regex under the hood                   |

No crate knows cargo's `test result:` format or ripgrep's `path:line:match`
shape. Semantic classification of tool output is domain-specific logic that
belongs in this codebase - the `or_else` pipeline is the zero-dependency answer.

## 2. Declarative detector pipeline - the blueprint

The transformation: a flat imperative `if`-chain with scattered re-scans of
`content.lines()` becomes a pipeline of pure, named detectors over one
pre-computed view of the blob. Three rules carry the whole design.

### Rule 1 - pre-compute the input once, typed

A zero-copy view built once at function entry; every detector and every builder
shares it, and nothing ever re-runs `content.lines().collect()`:

```rust
/// Pre-computed view of a content blob. Built once; per-detector cost is a
/// predicate scan over `non_empty` and nothing else.
pub(crate) struct Input<'a> {
    raw: &'a str,             // original content - raw contains-checks
    trimmed: &'a str,         // raw.trim_start() - html/xml open probes
    non_empty: Vec<&'a str>,  // lines, trim_end'd, non-blank, order kept
    n: usize,                 // non_empty.len() - majority denominators
    first: Option<&'a str>,   // first non-empty line
}

impl<'a> Input<'a> {
    fn new(content: &'a str) -> Option<Self>;      // None when non_empty is empty
    fn count(&self, pred: impl Fn(&str) -> bool) -> usize;
    fn any(&self, pred: impl Fn(&str) -> bool) -> bool;
    fn majority(&self, pred: impl Fn(&str) -> bool) -> bool; // c >= 2 && c*2 >= n
}
```

The concrete Aphrodite field set (total/bytes/first/last/shebang and their
consumer mapping) is worked out in `REFACTOR-PLAN-1.5.0.md` §4 - this doc keeps
the principle.

### Rule 2 - one pure function per shape

Each `if` block becomes a single-responsibility `fn detect(&Input) -> bool`.
One function per shape, never one mega-function. The single exception is the
html/xml detector, which returns `Option<&'static str>` because it dispatches
to two different labels.

```rust
fn detect_json(&Input) -> bool    { /* strict parse, envelope-guarded */ }
fn detect_test(&Input) -> bool    { /* raw contains + line predicates */ }
fn detect_code(&Input) -> bool    { /* strong signature OR >= 2 statement votes */ }
fn detect_xml(&Input) -> Option<&'static str>  { /* "html" | "xml" | None */ }
```

### Rule 3 - the dispatcher is a chain, priority is position

`None.or_else(...)` short-circuits exactly like the original early-return
chain, so priority order maps 1:1 onto chain position:

```rust
pub fn detect_semantic_type(content: &str) -> Option<&'static str> {
    let inp = Input::new(content)?;
    None
        .or_else(|| detect_json(&inp).then_some("json"))
        .or_else(|| detect_test(&inp).then_some("test"))
        .or_else(|| detect_code(&inp).then_some("code"))
        .or_else(|| detect_xml(&inp))                       // "html"/"xml"/None
        // ... remaining shapes in current arm order
}
```

Adding a shape is a one-line addition; reordering priorities is a one-line
move. Order is the contract - json stays first so a JSON payload can never be
hijacked by a marker substring inside its string values.

### String ops over regex, and the one justified regex

The test detector's four `LazyLock<Regex>` statics exist only because the old
code ran regexes over the whole `content` string. Against `Input`, every
pattern becomes a `starts_with` / `contains` / `bytes()` check: raw
contains-checks for `test result:`, `=== RUN `, `--- FAIL:` / `--- PASS:`
first (fastest, no line iteration), then line predicates for pytest `N passed`,
jest `Tests:`, and cargo `running N tests` / `test ... ok` shapes.

The strong-code marker is likewise a pure prefix chain - `fn `, `pub struct `,
`impl `, `def `, `class `, `func `, `package `, `#!`, `#include` - zero regex,
no `LazyLock`, same semantics, faster because no DFA construction is possible.

One regex survives, and only one: the multi-language statement vote in the code
detector (`use x::y;` / `let x =` / `import` / `from ... import` / `return` /
`println!` / `print(` / `echo`). Alternation across seven languages is the one
place a regex is the cleanest expression; everywhere else it is noise.

Repo-state divergence to note: Aphrodite is ahead of the source's assumptions -
`CODE_STRONG_RE` / `CODE_VOTE_RE` are already gone (the current
`is_code_strong_line` / `is_code_vote_line` are `starts_with` / structural
today). The real elimination targets are the five remaining statics
(`ERROR_LINE_RE`, `FAILED_COUNT_RE`, `LINT_RE`, `DUR_RE`, `SEARCH_RE`), one
static per commit, with the regex kept as a test-only oracle during the swap -
see `REFACTOR-PLAN-1.5.0.md` §6.

## 3. Reverse taxonomy - the atomic file tree

### The inversion principle

PlayForm's structure is reverse categorical: a path reads right-to-left, the
leaf name is the most specific identity, and each ancestor is a progressively
broader container. `Detect/Test.rs` reads "detector of test output"; the folder
answers _what kind of thing_, the file answers _of what_.

Two consequences fall out:

- **Kind lives in the folder, subject in the leaf.** `Detect/Test.rs`, not
  `Detect/IsTest.rs` - the `Is` prefix encodes predicate shape that the folder
  already states. The same leaf word recurs at multiple levels on purpose:
  `Detect/Test.rs` and `Preview/Test.rs` are different kinds sharing a subject,
  exactly as PlayForm uses `Summary` as both a `Fn/` file and a `Struct/`
  namespace.
- **Compiled pattern state is its own kind.** `Struct/Pattern/` groups the
  remaining `LazyLock<Regex>` statics under a namespace that says "compiled
  pattern state", distinct from data structs - the same principle as grouping
  config state under `Struct/Config/`.

### The conventions that make the tree work

- PascalCase filenames and module names; `#![allow(non_snake_case)]` once, at
  the crate root only.
- Every public function is named `Fn` inside its file - the module path _is_
  the name.
- A `.rs` file and a same-named folder always coexist when a function has
  sub-functions; `mod.rs` contains only `pub mod` declarations.
- No inline helpers: every helper, however small, gets its own file.
- Domain boundaries are explicit: Detect (shape classifiers), Preview (output
  formatters), Line (per-line primitive predicates, shared by both), Text
  (&str utilities), State (atomics + compiled patterns).

## 4. Workflow guidance - what survives as practice

The source's operational core distills to four rules, each scoped to one
domain per session:

1. **Split only, never refactor.** No logic renames, no signature changes, no
   behavior drift - files move and re-export, nothing else.
2. **One domain per session, starting with Preview.** It has no dependencies on
   the other domains: builders call helpers via `super::` until later sessions,
   so session 1 compiles and tests before the detectors are touched at all.
3. **Never touch code outside the scoped domain** - the boundary list is part
   of the prompt.
4. **Output one file per fenced block, path-labeled**, each file independently
   compilable, sub-modules called as `Entry::X::Fn(...)`.

## 5. The full file-tree adaptation - preview.rs as PlayForm structure

The source's exact file-tree suggestion, adapted to the current Aphrodite
`preview.rs` (2117 lines: detection + builders + line predicates + the cap
statics). The PlayForm shape: `Source/` is the crate root, `Library.rs` is
`lib.rs` holding only `mod` declarations, `Fn/` holds every function one
PascalCase file each, `Struct/` holds every struct/static, and a `.rs` file
with a same-named folder coexist whenever a function has sub-functions.

```
crates/aphrodite/src/
├── preview/                        ← new module root (replaces preview.rs)
│   ├── mod.rs                      ← pub mod Detect; pub mod Preview; pub mod Line; pub mod Text; pub mod State;
│   ├── Detect.rs                   ← pub fn Fn(content) -> Option<&'static str>  (the or_else pipeline)
│   ├── Detect/
│   │   ├── mod.rs                  ← pub mod Input; pub mod IsJson; pub mod IsTest; pub mod IsDiff; …
│   │   ├── Input.rs                ← pub struct Input (typed, pre-computed once) + new/count/any/majority
│   │   ├── IsJson.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsTest.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsDiff.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsCode.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsTable.rs              ← pub fn Fn(inp: &Input) -> bool   (markdown tables)
│   │   ├── IsMarkdown.rs           ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsYaml.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsHtmlOrXml.rs          ← pub fn Fn(inp: &Input) -> Option<&'static str>
│   │   ├── IsCsv.rs                ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsBuild.rs              ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsGitStatus.rs          ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsGitLog.rs             ← pub fn Fn(inp: &Input) -> bool
│   │   ├── IsGrep.rs               ← pub fn Fn(inp: &Input) -> bool
│   │   └── IsLs.rs                 ← pub fn Fn(inp: &Input) -> bool
│   ├── Preview.rs                  ← pub fn Fn(type_str, content) -> String  (build_preview)
│   ├── Preview/
│   │   ├── mod.rs                  ← pub mod GitStatus; pub mod GitLog; pub mod Ls; …
│   │   ├── GitStatus.rs            ← pub fn Fn(content, lines) -> String
│   │   ├── GitLog.rs               ← pub fn Fn(content, lines) -> String
│   │   ├── Ls.rs                   ← pub fn Fn(content, lines) -> String
│   │   ├── Test.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Grep.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Html.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Table.rs                ← pub fn Fn(content, lines) -> String
│   │   ├── Markdown.rs             ← pub fn Fn(content, lines) -> String
│   │   ├── Yaml.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Xml.rs                  ← pub fn Fn(content, lines) -> String
│   │   ├── Csv.rs                  ← pub fn Fn(content, lines) -> String
│   │   ├── Json.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Search.rs               ← pub fn Fn(content, lines) -> String
│   │   ├── Error.rs                ← pub fn Fn(content, lines) -> String
│   │   ├── Lint.rs                 ← pub fn Fn(content, lines) -> String
│   │   ├── Log.rs                  ← pub fn Fn(content, lines) -> String
│   │   ├── Terminal.rs             ← pub fn Fn(content, lines) -> String
│   │   └── Diff.rs                 ← pub fn Fn(content, lines) -> String
│   ├── Line/                       ← per-line primitive predicates, shared by Detect + Preview
│   │   ├── mod.rs
│   │   ├── IsErrorLine.rs          ← fn Fn(line) -> bool
│   │   ├── IsWarningLine.rs        ← fn Fn(line) -> bool
│   │   ├── IsFailureLine.rs        ← fn Fn(line) -> bool
│   │   ├── IsLintLine.rs           ← fn Fn(line) -> bool
│   │   ├── IsGrepLine.rs           ← fn Fn(line) -> bool
│   │   ├── IsPathLine.rs           ← fn Fn(line) -> bool
│   │   ├── IsMdHeading.rs          ← fn Fn(line) -> bool
│   │   ├── IsMdStructure.rs        ← fn Fn(line) -> bool
│   │   ├── IsYamlKeyLine.rs        ← fn Fn(line) -> bool
│   │   ├── GitStatusCode.rs        ← fn Fn(line) -> Option<&str>
│   │   ├── FirstMeaningfulLine.rs  ← fn Fn(content) -> Option<String>
│   │   ├── SampleLongLine.rs       ← fn Fn(line) -> String
│   │   └── NumBefore.rs            ← fn Fn(line, keyword) -> usize
│   ├── Text/                       ← &str utilities
│   │   ├── mod.rs
│   │   └── ApplyPreviewCap.rs      ← pub fn Fn(preview, max_chars) -> String
│   └── State/
│       ├── mod.rs                  ← pub mod PreviewMaxChars; pub mod ErrorLineRe; …
│       ├── PreviewMaxChars.rs      ← pub static PREVIEW_MAX_CHARS: AtomicU32  + Fn() -> u32 + set Fn
│       ├── ErrorLineRe.rs          ← static ERROR_LINE_RE: LazyLock<Regex>
│       ├── FailedCountRe.rs        ← static FAILED_COUNT_RE: LazyLock<Regex>
│       ├── LintRe.rs               ← static LINT_RE: LazyLock<Regex>
│       ├── DurRe.rs                ← static DUR_RE: LazyLock<Regex>
│       └── SearchRe.rs             ← static SEARCH_RE: LazyLock<Regex>
└── lib.rs                          ← pub mod preview; (contract unchanged: pub use preview::{build_preview, detect_type})
```

### The mod.rs pattern

Every `mod.rs` does exactly one thing - re-export its siblings:

```rust
// preview/Detect/mod.rs
pub mod Input;
pub mod IsJson;
pub mod IsTest;
pub mod IsDiff;
pub mod IsCode;
pub mod IsTable;
pub mod IsMarkdown;
pub mod IsYaml;
pub mod IsHtmlOrXml;
pub mod IsCsv;
pub mod IsBuild;
pub mod IsGitStatus;
pub mod IsGitLog;
pub mod IsGrep;
pub mod IsLs;
```

And `Detect.rs` itself becomes purely the pipeline:

```rust
pub mod Entry; // submodule declarations live in the .rs file, not mod.rs

use Entry::Input::Struct as Input;

pub fn Fn(content: &str) -> Option<&'static str> {
    let inp = Input::new(content)?;
    None
        .or_else(|| Entry::IsJson::Fn(&inp).then_some("json"))
        .or_else(|| Entry::IsTest::Fn(&inp).then_some("test"))
        .or_else(|| Entry::IsDiff::Fn(&inp).then_some("diff"))
        .or_else(|| Entry::IsCode::Fn(&inp).then_some("code"))
        // …remaining shapes in current arm order…
        .or_else(|| Entry::IsHtmlOrXml::Fn(&inp))
}
```

### Key conventions (verbatim rules to give a refactor agent)

- PascalCase filenames and module names - `IsTest.rs`, not `is_test.rs`.
- Every public function is named `Fn` inside its file - the module path _is_
  the name (`preview::Detect::Entry::IsTest::Fn`).
- `#![allow(non_snake_case)]` at the crate root - required because Rust warns
  on PascalCase fn/module names by default.
- Statics/globals live in `State/` (or `Struct/`) - never inline in function
  files.
- A `.rs` file and same-named folder always coexist when a function has
  sub-functions: `Detect.rs` declares the fn and `pub mod` entries; `Detect/`
  holds the sub-files.
- No inline helpers - every helper, however small, gets its own file in
  `Fn/` (or the domain-specific `Line/`/`Text/`).

### The exact scope boundary to state in a prompt

"Split only, never refactor: no logic renames, no signature changes, no
behavior drift. One domain per session, starting with Preview. Never touch
code outside the scoped domain. Output one file per fenced block,
path-labeled, each file independently compilable, sub-modules called as
`Entry::X::Fn(...)`."

### The exact output format to request

One fenced block per file, each opening fence labeled with the full target
path (`preview/Detect/IsJson.rs`), the file body complete and compilable in
isolation, nothing else in the block. Verify each phase with the repo's
test suite before the next session starts.

### Session order to use

1. `preview/State/` + `preview/Text/` (no dependencies - the cap + string
   utils move first, tests green).
2. `preview/Line/` (per-line predicates - depend only on &str).
3. `preview/Detect/` (the pipeline + Input + Is* shapes).
4. `preview/Preview/` (the builders, largest surface).
5. `lib.rs` re-export swap + dead-file removal.
Each session ends with `cargo test -p aphrodite` green before the next
begins.

## 6. Distilled vs omitted

Distilled into this record: the per-dimension crate conclusions and the
Aphrodite verdict (no new dependencies), the no-library-exists reasoning, the
three-rule declarative pipeline, the string-ops-over-regex principle with the
one justified regex, the reverse-taxonomy principle and its conventions, and
the session-sequencing practice.

Omitted on purpose: the source's verbatim prompt templates (their substance is
the four workflow rules above), the full 40+ file tree listings (superseded by
the real Aphrodite mapping in `REFACTOR-PLAN-1.5.0.md` §3), the Cargo.toml
dependency block (the verdict is no new deps), per-crate source links, and all
literal code blocks - every sketch here is re-derived. Nothing from the source
file was placed in the repo.
