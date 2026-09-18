# REFACTOR-PLAN-1.5.0 - Preview/Detection Layer: Declarative Detector Pipeline

**Scope:** `crates/aphrodite/src/preview.rs` (2117 lines) - the whole preview/detection
layer - restructured for 1.5.0 into a declarative detector pipeline with a reverse-
taxonomy file tree. Distills the IDEAS of the split.md rewrite (declarative detector
pipeline, typed pre-computed input, `Option::or_else` dispatcher, regex elimination,
reverse-taxonomy tree) and adapts them to the ACTUAL Aphrodite code. Original
adaptation - nothing transcribed.

**Status:** plan only. No code touched, nothing committed.

---

## 1. Ideas → Aphrodite adaptation map

| split.md idea                                         | Aphrodite reality (preview.rs)                                                                                                                                                                                                          | 1.5.0 adaptation                                                                                                                                   |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Declarative detector pipeline (`fn is_X(ls) -> bool`) | `detect_semantic_type` is a 212-line imperative `if`-chain; 14 shape arms                                                                                                                                                               | One file per shape in `detectors/`, each `fn detect(inp: &Input) -> bool` (the xml detector returns `Option<&'static str>`)                        |
| Typed input pre-computed once                         | Rebuilds `lines`/`non_empty` Vecs, then re-scans `content.lines()` inside the csv/build/git/grep/ls arms and in every builder                                                                                                           | `preview/input.rs::Input<'a>` built once per blob, shared by detectors AND builders                                                                |
| `or_else` dispatcher                                  | Early-return `if` chain; priority = arm order                                                                                                                                                                                           | `None.or_else(                                                                                                                                     |     | …)` chain with byte-identical priority order |
| Regex elimination                                     | 5 `LazyLock<Regex>` statics remain (ERROR_LINE_RE, FAILED_COUNT_RE, LINT_RE, DUR_RE, SEARCH_RE); the CODE_STRONG/CODE_VOTE regexes split.md assumed are ALREADY gone (is_code_strong_line/is_code_vote_line are starts_with/structural) | Table §6: all 5 statics → starts_with/splitn/token checks, one static per commit, regex kept as a test-only oracle during the swap                 |
| Reverse-taxonomy file tree (leaf = most specific)     | 1 file, 4 collapsed domains (detect / build / line-predicate / state)                                                                                                                                                                   | `preview/` module tree: `detectors/`, `builders/`, `line/`, `text/`, `state.rs` (§4)                                                               |
| (uncovered by split.md) proxy parallel layer          | `proxy.rs::proxy_detect_content_type` (145 lines) + `proxy_build_preview` (135 lines) duplicate the pipeline                                                                                                                            | §7: collapse into one `resolve_effective_type` + `build_preview`; proxy heuristics move into the pipeline (lang.rs, log/search/terminal detectors) |

Naming adaptation: the tree uses `detect` (not split.md's `is_X`) because the folder
already answers "what kind" - `detectors/test.rs`, not `detectors/is_test.rs`.

---

## 2. Current inventory (what moves where)

`preview.rs` today, by domain:

- **Entry points (pub):** `detect_type` (headroom wrapper), `detect_semantic_type`,
  `build_preview`, `set_preview_max_chars`, `preview_max_chars`, (pub(crate))
  `preview_cap_test_guard`. `lib.rs:528-529` re-exports: `pub mod preview; pub use
preview::{build_preview, detect_type};` - the facade must keep these exact names.
- **Detect arms (if-chain):** json (envelope-guarded, runs FIRST), test, diff, code
  (strong + vote), table, markdown, yaml, html, xml, csv, build, git, gitlog, grep, ls.
- **Line predicates (16):** is_envelope_json_object, is_md_heading, is_md_structure,
  is_yaml_key_line, git_status_code, is_grep_line, is_path_line, is_running_tests_line,
  is_test_result_line, has_number_before, has_number_after, is_fn_style_sig,
  is_type_decl, is_include_directive, is_code_strong_line, is_let_assign,
  is_use_statement, is_from_import, is_code_vote_line.
- **Line predicates (WS2 family):** is_error_line (+ERROR_LINE_RE), is_warning_line,
  is_failure_line (+FAILED_COUNT_RE), is_lint_line (+LINT_RE).
- **State:** PREVIEW_MAX_CHARS AtomicU32 + accessors + test guard.
- **Builders (15):** build_git_status_preview, build_gitlog_preview, build_ls_preview,
  build_test_preview (+DUR_RE), build_grep_preview, build_json_preview,
  build_search_preview (+SEARCH_RE), build_table_preview, build_markdown_preview,
  build_yaml_preview, build_xml_preview, build_csv_preview, build_html_preview,
  num_before, apply_preview_cap, first_meaningful_line, sample_long_line.
- **Tests:** one `mod tests` (28 tests) with battery pins - the frozen contract.

Call sites that consume the layer (all must keep compiling):

| Call site                                                       | Uses                                                                                             |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `crates/aphrodite/src/lib.rs:287`                               | `crate::build_preview(&t, &content)` (classify ABI path)                                         |
| `crates/aphrodite/src/hooks.rs:185-189, 197, 314, 387-391, 400` | detect upgrade blocks (2 near-duplicates) + build_preview (tool, terminal, chain-split segments) |
| `crates/aphrodite/src/proxy.rs:1408, 1946-2081`                 | detect_semantic_type inside proxy_detect_content_type; proxy_build_preview parallel builder      |
| `crates/aphrodite-hermes/src/tools.rs:252-275, 520-521`         | compress_into hint-wins + build_preview; reclassify detect_type + build_preview                  |

---

## 3. (a) Target file tree

`preview.rs` → `crates/aphrodite/src/preview/` (directory module; `lib.rs` untouched).
Phase 1 uses `git mv preview.rs preview/mod.rs` to keep history/blame.

```
crates/aphrodite/src/preview/
├── mod.rs              facade: pub mod decls + pub use of the 6 public fns (lib.rs contract)
├── detect.rs           detect_type (headroom wrapper) + detect_semantic_type or_else chain
├── type.rs             resolve_effective_type - the hint-wins contract (§5)   [Phase 5]
├── input.rs            pub(crate) struct Input<'a> + new/count/any/majority   (§4)
├── lang.rs             detect_language → code_rust/python/go/js/ts             [Phase 6]
├── state.rs            PREVIEW_MAX_CHARS atomic + set/get + preview_cap_test_guard
├── tests.rs            #[cfg(test)] the 28 existing tests verbatim + per-swap tests
│
├── detectors/          one file per SHAPE (17)
│   ├── mod.rs          pub mod decls only
│   ├── json.rs         strict parse + envelope guard (envelope predicate lives in text/)
│   ├── test.rs         raw-marker contains-checks + line/test.rs predicates
│   ├── diff.rs         diff --git | ---/+++ pair | @@ hunk + delta
│   ├── code.rs         strong-signature OR ≥2 statement votes (line/code.rs)
│   ├── table.rs        ≥2 pipe rows + separator row
│   ├── markdown.rs     ≥2 headings + structure/body guard (line/md_*.rs)
│   ├── yaml.rs         ≥3 top-level key lines (line/yaml_key.rs)
│   ├── xml.rs          → Option<&'static str>: "html" | "xml" | None (only non-bool detector)
│   ├── csv.rs          ≥2 rows, uniform comma-field count, no ", "
│   ├── build.rs        ≥2 verbs+errs (line/error.rs, line/warning.rs)
│   ├── git.rs          status-code majority (line/git_status.rs)
│   ├── gitlog.rs       commit-hash blocks
│   ├── grep.rs         grep-line majority (line/grep.rs)
│   ├── ls.rs           mode-byte ≥2 OR path-line majority (line/path.rs)
│   ├── search.rs       path:line: majority (stub → Phase 6)
│   ├── log.rs          log-marker/timestamp rules lifted from proxy.rs:1462-1478 (stub → Phase 6)
│   └── terminal.rs     exit-code/prompt rules (stub → Phase 6)
│
├── builders/           one file per PREVIEW ARM (22)
│   ├── mod.rs
│   ├── build.rs diff.rs git.rs gitlog.rs ls.rs test.rs grep.rs code.rs
│   ├── html.rs table.rs markdown.rs yaml.rs xml.rs csv.rs json.rs search.rs
│   ├── terminal.rs error.rs lint.rs log.rs generic.rs cap.rs
│
├── line/               primitive per-line predicates (13)
│   ├── mod.rs
│   ├── error.rs warning.rs failure.rs lint.rs grep.rs path.rs git_status.rs
│   ├── md_heading.rs md_structure.rs yaml_key.rs test.rs code.rs
│
└── text/               &str utilities (4)
    ├── mod.rs
    ├── first_meaningful.rs sample_long.rs num_before.rs envelope_json.rs
```

Line-predicate residency note: `is_grep_line`/`git_status_code`/`is_md_heading`/
`is_yaml_key_line` are consumed by BOTH a detector and a builder - they live in
`line/` exactly once; `num_before`/`first_meaningful_line`/`sample_long_line`/
`envelope_json` live in `text/`. No duplicated helpers.

---

## 4. (b) Typed input - pre-computed once

`preview/input.rs` - zero-copy view built ONCE per blob; no detector or builder ever
re-runs `content.lines().collect()`:

```rust
/// Pre-computed view of a content blob, shared by every detector and every
/// preview arm. Built once; predicate scans over `non_empty` are the only
/// per-detector cost.
pub(crate) struct Input<'a> {
    raw: &'a str,             // original content
    trimmed: &'a str,         // raw.trim_start()
    non_empty: Vec<&'a str>,  // lines().map(trim_end).filter(|l| !l.trim().is_empty()) - order kept
    total: usize,             // raw.lines().count()   → the `NL` in every [type:NL …] arm
    n: usize,                 // non_empty.len()        → majority denominators
    bytes: usize,             // raw.len()              → the `NB` in error/lint/log/generic arms
    chars: usize,             // raw.chars().count()    → char-boundary stats (cap decisions)
    first: Option<&'a str>,   // first non-empty line (trim_end'd)
    last: Option<&'a str>,    // last non-empty line (trim_end'd) - tail-fallback arms
    shebang: Option<&'a str>, // first line starting with "#!" (code-strong + shell-comment guard)
    bracket_balance: i32,     // '{'/'['/'(' minus '}'/']'/')' over non_empty (reserved)
    digit_lines: usize,       // lines containing ≥1 ASCII digit (reserved)
}

impl<'a> Input<'a> {
    fn new(content: &'a str) -> Option<Self>;   // None when non_empty is empty (mirrors current early-return)
    fn count(&self, pred: impl Fn(&str) -> bool) -> usize;
    fn any(&self, pred: impl Fn(&str) -> bool) -> bool;
    fn majority(&self, pred: impl Fn(&str) -> bool, min: usize) -> bool; // count ≥ min && count*2 ≥ n
}
```

Field → consumer mapping (current code that each field replaces):

| Field             | Replaces (preview.rs)                                                                                                                                            | Used by                                                     |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `raw`             | `content.contains(...)` test markers (74-84), xml `content.contains("</")` (157), terminal exit-code probes                                                      | detectors/test.rs, xml.rs; builders/terminal.rs             |
| `trimmed`         | `content.trim_start()` html/xml open checks (153-154)                                                                                                            | detectors/xml.rs                                            |
| `non_empty`       | the `lines`+`non_empty` Vecs (39-45) and every re-scan inside arms (csv 163, build 176-199, git 204, gitlog 210-220, grep 223, ls 230-247)                       | all detectors                                               |
| `total` / `bytes` | `content.lines().count()` + `content.len()` recomputed in build_preview (684-685) and every builder                                                              | builders (line/byte counts)                                 |
| `first` / `last`  | `first_meaningful_line` re-scan (313-326), terminal/error/lint/log fallback tails (876, 897, 911, 926)                                                           | builders/terminal.rs, error.rs, lint.rs, log.rs, generic.rs |
| `shebang`         | `t.starts_with("#!")` in is_code_strong_line (507) - hoisted so the markdown guard test (`test_shell_comments_are_not_misdetected_as_markdown`) stays one lookup | detectors/code.rs                                           |
| `bracket_balance` | nothing today - reserved for a future xml/json structural guard; NOT consumed in 1.5.0 (YAGNI guard)                                                             | -                                                           |
| `digit_lines`     | nothing today - reserved cheap gate for detectors/test.rs; NOT consumed in 1.5.0                                                                                 | -                                                           |

Exact-threshold rules stay with the detectors (do NOT force every arm through
`majority`): git = `count ≥ 2 && count*2 ≥ n`; grep = same; ls-path = `count ≥ 3 &&
count*2 ≥ n`; yaml = `count ≥ 3`; build = `verbs+errs ≥ 2`; code = `any strong ||
votes ≥ 2`; markdown = `heads ≥ 2 && (structure ≥ 1 || body ≥ 1)`.

---

## 5. (c) The or_else dispatcher + ordering + hint-wins contract

`preview/detect.rs` - the entire dispatch, order = current arm order (priority IS the
chain position; short-circuit evaluation preserves exact semantics):

```rust
/// Order is the contract: identical to the current if-chain (json FIRST so a
/// JSON payload can never be hijacked by a marker substring inside its string
/// values). search/log/terminal are stubs returning None until Phase 6.
pub fn detect_semantic_type(content: &str) -> Option<&'static str> {
    let inp = Input::new(content)?;
    None
        .or_else(|| detectors::json::detect(&inp).then_some("json"))
        .or_else(|| detectors::test::detect(&inp).then_some("test"))
        .or_else(|| detectors::diff::detect(&inp).then_some("diff"))
        .or_else(|| detectors::code::detect(&inp).then_some("code"))
        .or_else(|| detectors::table::detect(&inp).then_some("table"))
        .or_else(|| detectors::markdown::detect(&inp).then_some("markdown"))
        .or_else(|| detectors::yaml::detect(&inp).then_some("yaml"))
        .or_else(|| detectors::xml::detect(&inp))                       // "html"/"xml"/None
        .or_else(|| detectors::csv::detect(&inp).then_some("csv"))
        .or_else(|| detectors::build::detect(&inp).then_some("build"))
        .or_else(|| detectors::git::detect(&inp).then_some("git"))
        .or_else(|| detectors::gitlog::detect(&inp).then_some("gitlog"))
        .or_else(|| detectors::grep::detect(&inp).then_some("grep"))
        .or_else(|| detectors::ls::detect(&inp).then_some("ls"))
        .or_else(|| detectors::search::detect(&inp).then_some("search"))   // Phase 6
        .or_else(|| detectors::log::detect(&inp).then_some("log"))         // Phase 6
        .or_else(|| detectors::terminal::detect(&inp).then_some("terminal")) // Phase 6
}
```

Adding a shape = one chain line + one `detectors/<shape>.rs` file. Reordering a
priority = moving one line.

**Hint-wins contract** - currently inlined in `build_preview` (702-709) and
duplicated in `hooks.rs` (185-189, 387-391) and `tools.rs` compress_into (252-272).
1.5.0 extracts it to `preview/type.rs`:

```rust
/// Single source of truth for type resolution:
/// 1. explicit non-generic hint wins (never upgraded);
/// 2. generic buckets ("text" | "terminal" | "log" | "" | "plain" | "tool_result")
///    → detect_semantic_type(content).unwrap_or(hint);
/// 3. "build_output" | "build_error" → upgrade to "test" ONLY when the run is
///    clean (no failure line); a failing run keeps the build arm (WS2 pin);
/// 4. (Phase 6) the terminal exit-code override (hooks.rs:379-381) folds in here
///    as an explicit terminal-path rule - it must NOT become a universal
///    detector, or every "Error:" line would classify terminal.
pub(crate) fn resolve_effective_type(hint: &str, inp: &Input<'_>) -> Cow<'static, str>
```

`build_preview(type_str, content)` keeps its ABI signature; internally it builds
`Input::new(content)` once, resolves the effective type, and dispatches to
`builders::dispatch(&effective, &inp)`. `tools.rs` compress_into calls
`resolve_effective_type` for the hint-vs-unwrap precedence instead of its inline
if/else; the Hermes-envelope unwrap itself STAYS in the bridge (pinned by the
27-row `test_unwrap_hermes_result_table`).

---

## 6. (d) Regex-elimination table

All five statics die; `regex` stays in Cargo.toml because `marker.rs:162` HASH_RE
(and its test regex at 246) remain - converting the marker parser is out of scope
(§9). Detection and preview become 100% regex-free.

| #   | Static (preview.rs)   | Pattern                                      | Consumer                           | Fixed replacement (starts_with / splitn / token scan)                                                                                                                                                                                                                        |
| --- | --------------------- | -------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | ERROR_LINE_RE (638)   | `\b\w+(?:Error\|Exception):`                 | is_error_line (635)                | token-before-colon scan: for each `:` in the line, walk back over the `\w` run; match when that run ends `Error` or equals `Exception` and its first char is line-start or preceded by a non-alnum char. No hardcoded exception list; keeps ValueError:/KeyError:/Exception: |
| 2   | FAILED_COUNT_RE (664) | `(?i)[1-9]\d*\s+failed`                      | is_failure_line (661)              | `num_before_kw(t, "failed") >= 1` - extend text/num_before.rs with a casefold-aware find (bare `FAILED` is already caught by the `contains("FAILED")` check above it; `0 failed` excluded by `>= 1`)                                                                         |
| 3   | LINT_RE (674)         | `^[^\s:]+:\d+:\d+:\|(?:^\|\s)[EWF]\d{3,4}\b` | is_lint_line (671)                 | two structural checks: (a) `splitn(4, ':')` → tok1 non-empty without space/colon, tok2 all digits, tok3 all digits; (b) `split_whitespace().any(                                                                                                                             | tok | 4 ≤ len ≤ 5 && tok[0] ∈ {E,W,F} && rest all digits)` |
| 4   | DUR_RE (1236)         | `(\d+\.\d+s\|\d+ms)`                         | build_test_preview dur (1175-1177) | token scan, first match wins (same order as `captures`): token ends `s` with `digits '.' digits` prefix, or ends `ms` with all-digit prefix                                                                                                                                  |
| 5   | SEARCH_RE (1282)      | `^[^\s:]+:\d+:`                              | build_search_preview (1291)        | `is_search_line(line)`: `splitn(3, ':')` → path non-empty no-space, middle non-empty all digits, third segment exists. This is `is_grep_line` minus the file-ish path check → share the splitn core in line/grep.rs                                                          |

Swap discipline (Phase 4): one static per commit; the regex is kept in a
`#[cfg(test)]` const as an ORACLE that cross-checks the new predicate against every
battery fixture, then deleted. The two code regexes split.md assumes exist
(CODE_STRONG_RE / CODE_VOTE_RE) are already gone in Aphrodite - `is_code_strong_line`
/ `is_code_vote_line` are starts_with/structural today; no work needed there.

---

## 7. (e) Merging the parallel arms into one pipeline

Today three parallel classifiers + two preview builders exist. 1.5.0 collapses them
onto: `detect_semantic_type` (pipeline) + `resolve_effective_type` (contract) +
`build_preview` (single builder).

**proxy.rs - `proxy_detect_content_type` (1336-1480) fate:**

| proxy block                                         | lines     | 1.5.0 fate                                                                                                                                                                                                                                                                                                                            |
| --------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JSON `{`/`[` + exit_code/status → tool_output/json  | 1340-1350 | deleted; json detector + envelope guard cover it; `"tool_output"` becomes an alias arm on builders/json.rs (additive)                                                                                                                                                                                                                 |
| per-language code heuristics (rust/python/go/js/ts) | 1353-1399 | moved to `preview/lang.rs::detect_language(&Input) -> Option<&'static str>`, consulted when shape == "code"; rules re-expressed line-based (rust: fn/pub fn/async fn/impl/struct/enum lines AND `->`/`&`/`use`; python: `def ` AND import/class/from/self.; go: func/package AND `import (`; js: function/const/=> AND import/export) |
| semantic override                                   | 1408-1410 | already the pipeline - unchanged after §5                                                                                                                                                                                                                                                                                             |
| error first-line                                    | 1413-1421 | deleted; classifier/hint supply "error"; line/error.rs already line-based                                                                                                                                                                                                                                                             |
| build first-line                                    | 1424-1431 | deleted; build detector                                                                                                                                                                                                                                                                                                               |
| linter first-line                                   | 1434-1445 | deleted; line/lint.rs                                                                                                                                                                                                                                                                                                                 |
| diff first-line                                     | 1448-1454 | deleted; diff detector                                                                                                                                                                                                                                                                                                                |
| git first-line (commit/`On branch`)                 | 1457-1459 | `commit ` → gitlog detector; `On branch ` long-form → git detector extension (Phase 6, behind a battery test - proxy currently classifies it "git", core would say text: a regression if dropped)                                                                                                                                     |
| log markers + timestamp lines                       | 1462-1478 | moved verbatim into `detectors/log.rs` (Phase 6)                                                                                                                                                                                                                                                                                      |
| `text` fallback                                     | 1479      | stays                                                                                                                                                                                                                                                                                                                                 |

**proxy.rs - `proxy_build_preview` (1946-2081) fate: deleted.** Every arm is a
strict subset of the core builder: code → core code arm (struct_extract structure
map + first signature, richer than 2 raw sigs); error → `[error:NL NB | msg]`;
diff → `[diff:NF +N/-N NL | files]`; json → `[json:Nkeys …]`; build → honest E/W
tallies; default → first-meaningful + head/tail sample. The parity pre-check
(1954-1960) dies with the builder - ALL types route to core. LLM-visible marker
text converges on core formats; proxy pin tests update in Phase 5 and the change is
called out in the CHANGELOG.

**hooks.rs - two near-identical classify blocks (185-189 and 387-391) become one
`resolve_effective_type` call.** The terminal exit-code override (379-381) stays a
terminal-path rule (contract rule 4), NOT a universal detector. The chain-split
per-segment path (314) is untouched - it already calls `build_preview`.

**tools.rs** - `compress_into` (252-272) delegates hint-vs-unwrap precedence to
`resolve_effective_type`; `aphrodite_reclassify` (520-521) is already correct and
stays. `lib.rs:287` classify path already calls `build_preview` - unchanged.

Result: one detection pipeline, one preview builder, one type-resolution contract,
four thin call sites.

---

## 8. (f) Migration order - tests green at every step

Green criterion at each phase: `cargo test -p aphrodite` and
`cargo test -p aphrodite-hermes` fully pass; battery (per `.hermes/notes/issue11/
ISSUE-11-PREVIEW-BATTERY.md`) re-run at Phase 6.

| Phase                      | Work                                                                                                                                                                                                                                                        | Green proof                           |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| 0 Baseline                 | `git mv preview.rs preview/mod.rs` (history preserved); run the suite; freeze the 28 preview tests + battery pins as the contract                                                                                                                           | identical pass list                   |
| 1 Pure extraction          | Move helpers VERBATIM into line/, text/, builders/, detectors/, state.rs; mod.rs re-exports the 6 pub fns; visibility → pub(crate) where cross-module. Zero logic edits                                                                                     | full suite                            |
| 2 Typed input + pipeline   | Add input.rs; reimplement detect_semantic_type as the or_else chain; detector bodies = current arm blocks reading `inp.non_empty`; search/log/terminal = stubs returning None; add an ordering test (battery fixtures through the chain == Phase 0 outputs) | suite + ordering test                 |
| 3 Builders split           | build_preview arms → builders/*.rs; arms switch to `inp.total`/`inp.bytes`/`inp.first`/`inp.last` (identical values); apply_preview_cap → builders/cap.rs                                                                                                   | full suite                            |
| 4 Regex elimination        | one commit per static (§6 table), oracle cross-check per swap, statics deleted; pattern/ dir (if created in Phase 1) deleted when empty                                                                                                                     | suite + oracle tests                  |
| 5 Single pipeline          | type.rs resolve_effective_type; rewire hooks.rs (both blocks), tools.rs compress_into, proxy.rs (delete proxy_detect_content_type + proxy_build_preview, add lang-agnostic arms aliases); update proxy pin tests to core formats                            | full workspace suite + battery re-run |
| 6 New detectors + language | fill search/log/terminal detectors (rules from proxy/hooks, line-based, with battery tests); lang.rs detect_language; git long-form (`On branch`) extension; code_rust etc. converge on core arm                                                            | suite + battery re-run                |
| 7 Cleanup                  | delete preview.rs shim if kept; `cargo fmt`; prettier on docs; update issue11/INDEX.md with this plan; CHANGELOG 1.5.0 entry (proxy preview-format convergence note)                                                                                        | full workspace suite + fmt            |

---

## 9. (g) Risk register

| Risk                                                                                                                                                                          | L   | I   | Phase | Mitigation                                                                               |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --- | --- | ----- | ---------------------------------------------------------------------------------------- |
| Lines re-cut drifts on empty/whitespace/NUL/multibyte input (battery pins test_preview__\_never_panics_)                                                                      | M   | H   | 2     | detector bodies moved verbatim; full pathological-input battery run per phase            |
| `(?i)` casefold drift in FAILED_COUNT_RE ("Failed" middle-case)                                                                                                               | M   | M   | 4     | casefold-aware num_before_kw + oracle cross-check                                        |
| LINT_RE word-boundary vs token split (E### glued to punctuation)                                                                                                              | M   | M   | 4     | token scan + oracle; table-driven lint fixtures                                          |
| DUR_RE first-match ordering (multiple durations on one line)                                                                                                                  | L   | L   | 4     | token scan iterates lines in same order; oracle                                          |
| or_else chain order regression vs if-chain                                                                                                                                    | L   | H   | 2     | ordering test over all battery fixtures                                                  |
| `On branch`/`git status` long-form regresses to text when proxy classifier dies                                                                                               | M   | M   | 5-6   | git detector extension + battery test                                                    |
| Proxy preview format convergence changes LLM-visible markers (error/diff/json/code)                                                                                           | M   | M   | 5     | CHANGELOG call-out; pin tests updated deliberately; battery after-run documents the diff |
| tools.rs hint-wins rework breaks the 27-row unwrap table                                                                                                                      | M   | H   | 5     | unwrap stays in bridge; only the type-precedence delegates; table re-run                 |
| ABI surface drift (`pub use preview::{build_preview, detect_type}`; hooks/proxy call `crate::preview::detect_semantic_type`; config_loader/engine call set_preview_max_chars) | L   | H   | 1     | mod.rs re-exports the exact 6 names; compile-fail test or grep check in CI               |
| PREVIEW_MAX_CHARS race returns (cap_guard lost in the split)                                                                                                                  | M   | M   | 3     | cap_guard + all 28 tests move together into tests.rs                                     |
| Chain-split per-segment path (hooks.rs:314) regresses                                                                                                                         | L   | M   | 5     | untouched call site; suite covers                                                        |
| `regex` dependency cannot be dropped (marker.rs HASH_RE)                                                                                                                      | -   | L   | 4/7   | documented expectation; no Cargo.toml churn in this plan                                 |
| Over-engineered Input fields (bracket_balance/digit_lines unused)                                                                                                             | M   | L   | 2     | fields reserved but unconsumed; removed if the pipeline lands without needing them       |

---

## 10. Out of scope (explicitly)

- `marker.rs` HASH_RE parser conversion (separate 1.5.x candidate - it is a marker
  parser, not detection).
- `struct_extract.rs` - already the code-structure source for the code arm.
- `chain_split.rs` - segment_error_hint and segment splitting untouched.
- `headroom` vendor classifier - the fork boundary stays (detection upgrades in
  Aphrodite's own layer only).
- `proxy.rs` marker formatting (`format_ccr_output`/`smart_marker`) - consumers of
  the preview string, not the pipeline.
