# Content Type Taxonomy

Every payload that enters the compression pipeline is labeled with a content
type so thresholds and previews can adapt per type - errors stay visible in
context while verbose build and log output is compressed at the base
threshold. Classification is layered: a vendored base classifier assigns one
of seven coarse types, an Aphrodite-side semantic detector refines common
tool-output shapes, the Hermes bridge unwraps wrapper envelopes before either
runs, and an explicit caller-supplied type hint always wins. Across all
stages the pipeline can emit **30 distinct content types**.

The preview each type produces is documented in the
[Enriched Preview Catalog](../proxy/compression.md#enriched-preview-catalog).

## Classification Pipeline

| Stage               | Where                                  | Role                                                                                                                                                                    |
| ------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Base classifier     | `headroom-core` content detector       | Coarse type: JSON array, source code, search results, build output, diff, HTML, plain text. Confidence-gated, first match wins                                          |
| Semantic refinement | `crates/aphrodite/src/preview.rs`      | Refines common shapes (test output, git status/log, ls, grep, tables, markdown, YAML, XML, CSV) that the base classifier flattens to `text`                             |
| Proxy heuristics    | `crates/aphrodite/src/proxy.rs`        | The standalone proxy path's own ladder: JSON envelopes, per-language code, errors, build, linter, diff, git, log                                                        |
| Envelope unwrap     | `crates/aphrodite-hermes/src/tools.rs` | The Hermes plugin path: unwraps `{output, exit_code}`, `{diff}`, `{error}`, `{total_count, matches}`, `{content, total_lines}` envelopes before classifying the payload |

The proxy path classifies raw pushed content; the Hermes plugin path
unwraps envelopes first, then runs the same base classifier and semantic
detector on the extracted payload. Both paths share the semantic detector, so
a `git status` or `ls -l` result is tagged the same way whether it arrives
through the proxy or through a Hermes tool call.

## Base Classifier (7 Types)

The vendored `headroom-core` content detector is regex-based and
confidence-gated: each candidate type must clear a confidence floor, and the
first candidate to clear its floor wins. JSON objects deliberately do not
match - only arrays do - so an object falls through to `text`.

| Type          | Detection signal                                                            | Confidence floor          |
| ------------- | --------------------------------------------------------------------------- | ------------------------- |
| `json_array`  | Strict JSON parse of an array (`[`-prefixed)                                | none (strongest signal)   |
| `diff`        | `diff --git`, `diff --cc`, `--- a/`, `@@ -A,B +C,D @@`, `@@@` hunk headers  | 0.7                       |
| `html`        | `<!DOCTYPE html` / `<html` / structural tags                                | 0.7                       |
| `search`      | `path:line:` prefix on a majority of non-blank lines                        | 0.6                       |
| `build`       | Log-level markers, timestamps, test verdicts, `npm ERR!` / `cargo error`    | 0.5                       |
| `source_code` | Per-language pattern votes (python, javascript, typescript, go, rust, java) | 0.5                       |
| `text`        | Fallback                                                                    | 0.5 (0.0 for empty input) |

## Semantic Refinement (15 Types)

`detect_semantic_type` runs after the base classifier and before the loose
first-line heuristics. Detection is deliberately conservative - line-prefix
patterns and majority votes - so ordinary prose is never mis-tagged.

| Order | Type           | Detection signal                                                                                                  |
| ----- | -------------- | ----------------------------------------------------------------------------------------------------------------- |
| 1     | `json`         | Strict parse of an object or array, unless the object is a Hermes wrapper envelope                                |
| 2     | `test`         | `test result:`, `=== RUN`, `--- PASS:`, `N passed` / `N failed`, `running N tests`                                |
| 3     | `diff`         | `diff --git`, `---` + `+++` header pair, or `@@` hunk with change lines                                           |
| 4     | `code`         | Strong signature line (`fn main(`, `def x(`, `struct X`, `#include`, shebang) or two or more statement-line votes |
| 5     | `table`        | Two or more `                                                                                                     | `-prefixed lines with a separator row |
| 6     | `markdown`     | Two or more heading lines plus structure or body prose                                                            |
| 7     | `yaml`         | Three or more top-level lowercase `key: value` lines                                                              |
| 8     | `html` / `xml` | `<!DOCTYPE html` / `<html` open, or `<` open with `</` close                                                      |
| 9     | `csv`          | Two or more rows with an identical comma-field count (no `, ` comma-space)                                        |
| 10    | `build`        | Two or more cargo/rustc verb or error lines (`Compiling `, `Finished `, `error[`, `warning:`)                     |
| 11    | `git`          | Porcelain status codes on a majority of lines (`M `, `A `, `??`, `UU`)                                            |
| 12    | `gitlog`       | `commit <hash>` blocks                                                                                            |
| 13    | `grep`         | `path:line:match` on a majority of lines                                                                          |
| 14    | `ls`           | `ls -l` mode strings, or a majority of bare path-like tokens                                                      |

A JSON object is excluded from `json` when any of its keys matches the
Hermes wrapper envelope guard (`output`, `exit_code`, `diff`, `error`,
`success`, `total_count`, `matches`, `matches_text`, `content`,
`total_lines`, `result`, `message`, `found`, `preview`, `name`,
`description`): those objects are wrappers whose payload lives inside, and
the bridge unwraps them instead of previewing the key list.

## Proxy Heuristics

The standalone proxy path classifies raw content with its own ladder, running
the semantic detector between the code step and the error step:

1. JSON: `{`- or `[`-prefixed. Invalid JSON → `text`; contains `exit_code` or
   the quoted key `"status"` → `tool_output`; otherwise `json`
2. Code (more than 3 lines): `code_rust` (fn/impl/struct/enum plus `->`, `&`,
   or `use`), `code_python` (def plus import/class/from/self.), `code_go`
   (func/package plus `import (`), `code_js` (function/const/=> plus
   import/export), generic `code`
3. Semantic detector (the 15 refined shapes above)
4. Error: first line contains `error`/`Error`/`ERROR`/`Traceback`/`panic`, or
   starts with `thread '` → `error`
5. Build output: first line starts with `Compiling`, `running`, or `test`, or
   contains `Finished` → `build_output`
6. Linter: first line starts with `error[E`, `error:`, `warning[`, `warning:`,
   or contains `mypy`, `clippy`, `eslint`, `tsc` → `linter`
7. Diff: first line starts with `diff --git`, `@@ -`, `+++`, or `---` → `diff`
8. Git: first line starts with `commit ` or `On branch ` → `git`
9. Log: a line starts with `[` and contains a log level, or starts with a
   digit and is ISO/syslog-like → `log`
10. Fallback → `text`

## Envelope Unwrap (Hermes Bridge)

Before classification, the Hermes bridge extracts the meaningful payload from
wrapper envelopes. The stored content is always the original bytes - the
extracted payload is used only for classification and preview:

| Envelope                                | Extracted payload                          | Classified as                                                                                                                                                                                            |
| --------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `{output, exit_code}`                   | `output`                                   | `terminal` if it contains `exit code:` or `Error:`; `build_error` if it contains `error[` or `error: could not`; `build_output` if it is cargo/test verb output; otherwise the base classifier's verdict |
| `{success, diff}` / `{error}`           | `diff` / error message                     | Base classifier on the diff, or `text` for error messages                                                                                                                                                |
| `{total_count, matches / matches_text}` | One `path:line:content` row per real match | `search`                                                                                                                                                                                                 |
| `{content, total_lines}`                | `content`                                  | Base classifier's verdict                                                                                                                                                                                |

A caller-supplied `type` hint (from the tool schema) overrides the detected
type. For JSON payloads the preview is built from the full stored content,
never from an unwrap fragment.

## Complete Registry (30 Types)

| Type           | Produced by            | Threshold | Preview arm                                        |
| -------------- | ---------------------- | --------- | -------------------------------------------------- |
| `json`         | Semantic, proxy        | ×1        | `[json:{keys} {ln}L \| {first keys}]`              |
| `json_array`   | Base                   | ×1        | `[json:{items} items {ln}L]`                       |
| `source_code`  | Base                   | ×code     | `[code:{fns} fns {first fn} {ln}L]`                |
| `code`         | Semantic, proxy        | ×code     | code arm                                           |
| `code_rust`    | Proxy                  | ×code     | code arm                                           |
| `code_python`  | Proxy                  | ×code     | code arm                                           |
| `code_go`      | Proxy                  | ×code     | code arm                                           |
| `code_js`      | Proxy                  | ×code     | code arm                                           |
| `test`         | Semantic               | ×1        | `[test:{pass} pass {fail} fail {ignored} ignored]` |
| `diff`         | Base, semantic, proxy  | ×2        | `[diff:{files} {ln}L \| {file} +more]`             |
| `build`        | Base, semantic         | ×1        | `[build:{err}E {warn}W {ln}L]`                     |
| `build_output` | Proxy, bridge          | BASE      | build family                                       |
| `build_error`  | Bridge                 | BASE      | build family                                       |
| `linter`       | Proxy                  | BASE      | `[lint:{ln}L {size}B \| {first issue}]`            |
| `error`        | Proxy                  | ×8        | `[error:{ln}L {size}B \| {first error line}]`      |
| `log`          | Proxy                  | BASE      | `[log:{ln}L {size}B \| {error or tail}]`           |
| `terminal`     | Bridge                 | ×1        | `[terminal:{lines} exit={exit}]`                   |
| `search`       | Base, bridge           | ×1        | `[grep:{files} matches {ln}L \| {first loc}]`      |
| `grep`         | Semantic               | ×1        | `[grep:{files} hits {ln}L \| {first loc}]`         |
| `git`          | Semantic, proxy        | ×2        | `[git:{mod}M {add}A \| {file} +more]`              |
| `gitlog`       | Semantic               | ×1        | `[gitlog:{commits} commits {first} {last}]`        |
| `ls`           | Semantic               | ×1        | `[ls:{files} files {dirs} dirs \| {extensions}]`   |
| `table`        | Semantic               | ×1        | `[table:{items} rows {ln}L]`                       |
| `markdown`     | Semantic               | ×1        | `[markdown:{ln}L {size}B \| {first heading}]`      |
| `yaml`         | Semantic               | ×1        | `[yaml:{keys} keys {ln}L \| {first key}]`          |
| `xml`          | Semantic               | ×1        | `[xml:{ln}L {size}B \| {root}]`                    |
| `csv`          | Semantic               | ×1        | `[csv:{rows} rows {ln}L]`                          |
| `html`         | Base, semantic         | ×1        | `[html:{title} {ln}L]`                             |
| `tool_output`  | Proxy                  | ×1        | Proxy-only label (envelope guess)                  |
| `text`         | Every stage's fallback | ×2        | `[text:{ln}L {size}B \| {first line}]`             |

## Threshold Groups

The per-type threshold decides how many bytes of a type survive in context
before compression. Base is the floor for all types.

| Group                                 | Types                                                    | Multiplier                      |
| ------------------------------------- | -------------------------------------------------------- | ------------------------------- |
| Noisy (BASE, excluded from auto-tune) | `linter`, `build_output`, `log`                          | base, immediately - never tuned |
| Error                                 | `error`                                                  | ×8                              |
| Code                                  | `code_rust`, `code_python`, `code_go`, `code_js`, `code` | × code multiplier               |
| Tracked                               | `diff`, `git`, `text`                                    | ×2                              |
| Default                               | `tool_output`, `json`, everything else                   | ×1                              |

The code multiplier is configurable (`compression.code_multiplier` or
`APHRODITE_CODE_MULTIPLIER`) and defaults to 3.0. Auto-tune adjusts the base
from the historical compression-ratio EMA: ratio above 20 → ×2.0, ratio
below 3 → ×0.5, otherwise ×1.0. The noisy group is exempt - build, lint, and
log output stays fully visible because a coding session needs to read it.

## Detection Examples

### JSON envelope (proxy path)

```json
{ "output": "Compiling aphrodite v1.4.6", "exit_code": 0 }
```

→ type=`tool_output` (contains `exit_code`), threshold ×1

### Rust code (proxy path)

```rust
pub fn main() -> Result<()> {
    let app = AppState::new();
}
```

→ type=`code_rust`, threshold ×code multiplier

### Build output (proxy path)

```text
   Compiling aphrodite v1.4.6
   Compiling headroom-core v1.0.0
    Finished release [optimized] target(s) in 12.34s
```

→ type=`build_output`, threshold BASE (fully visible, never auto-tuned)

### Linter output (proxy path)

```text
error[E0308]: mismatched types
  --> src/proxy.rs:841:5
```

→ type=`linter` (matches `error[E` before the generic error check), threshold BASE

### Test output (semantic detector)

```text
test result: ok. 3 passed; 0 failed; 0 ignored
```

→ type=`test`, threshold ×1

### Terminal output (Hermes bridge)

```json
{ "output": "error: could not compile `aphrodite`", "exit_code": 101 }
```

→ unwrapped to the message, type=`build_error` (contains `error: could not`),
threshold BASE
