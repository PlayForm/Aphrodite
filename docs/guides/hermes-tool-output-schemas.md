# Hermes Tool Output Schemas

Every Hermes tool returns output with its own shape: unified diffs, JSON
envelopes, grep listings, markdown, console logs. Aphrodite classifies each
result into a content type, builds a compact preview, and - when the result
is above the compression threshold and eligible - replaces it with a CCR
marker. This page maps the tool surface: what each tool returns, how that
output is classified, and how much compression typically helps.

The catalog below covers 43 tool-output shapes: 31 Hermes tools, 10 Aphrodite
meta-tools, and two special entries. The classification pipeline and its 30
content types are documented in [Content Type
Taxonomy](../classification/content-types.md); the exact preview strings are
in the [Enriched Preview
Catalog](../proxy/compression.md#enriched-preview-catalog).

## Classification Pipeline

Classification runs in stages, and the first match wins at each stage:

| Stage               | Role                                                                                                                                                                              |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Envelope unwrap     | The Hermes bridge extracts the payload from wrapper envelopes (`{output, exit_code}`, `{diff}`, `{error}`, `{total_count, matches}`, `{content, total_lines}`) before classifying |
| Base classifier     | Confidence-gated regex detection: JSON arrays, diff, HTML, search results, build output, source code, text                                                                        |
| Semantic refinement | Conservative shape detection for git status/log, grep, ls, test output, tables, markdown, YAML, XML, CSV                                                                          |
| Caller hint         | An explicit `type` hint on the call overrides the detected type                                                                                                                   |

The stored content is always the original bytes - the extracted payload is
used only for classification and preview. A caller-supplied type hint always
wins. See [Content Type
Taxonomy](../classification/content-types.md#classification-pipeline) for
the full detection order and signals.

## Threshold Groups

Each content type has a threshold multiplier that decides how many bytes
survive in context before compression:

| Group                          | Types                                                    | Multiplier                      |
| ------------------------------ | -------------------------------------------------------- | ------------------------------- |
| Noisy (base, never auto-tuned) | `linter`, `build_output`, `log`                          | base - always fully visible     |
| Error                          | `error`                                                  | ×8                              |
| Code                           | `code`, `code_rust`, `code_python`, `code_go`, `code_js` | × code multiplier (default 3.0) |
| Tracked                        | `diff`, `git`, `text`                                    | ×2                              |
| Default                        | `tool_output`, `json`, everything else                   | ×1                              |

The base itself is auto-tuned from the historical compression-ratio EMA
(ratio above 20 → ×2.0, below 3 → ×0.5); the noisy group is exempt so build,
lint, and log output stays readable during a coding session. See [Content
Type Taxonomy](../classification/content-types.md#threshold-groups) for the
full registry.

## Tool-by-Tool Reference

### Hermes tools

| Tool                 | Typical output shape                                              | Classification                                                                 | Compression benefit                         |
| -------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------- |
| `read_file`          | `LINE_NUM \| CONTENT` lines, optional hint footer                 | `text`, reclassified `code_*` when a language is detected, `json` if parseable | High - large files get structure previews   |
| `write_file`         | JSON `{status, path, bytes, syntax_errors}` or plain confirmation | `json` / `text` / `error` (syntax errors)                                      | Low - usually short                         |
| `patch`              | Unified diff (`---`/`+++`/`@@`) or confirmation                   | `diff` / `text`                                                                | Medium - diffs compress well                |
| `search_files`       | `path:line:content` rows, file-only, or count mode                | `search` / `grep` (file:line pattern) / `text`                                 | Medium - large result sets                  |
| `terminal`           | stdout/stderr plus exit code                                      | `terminal` / `build_output` / `build_error` / `error` / `text`                 | Very high - verbose output                  |
| `process`            | JSON list, poll delta, paginated log, or confirmation             | `json` / `text` / `terminal` (exit code in output)                             | Medium - logs verbose, poll deltas short    |
| `execute_code`       | stdout plus optional stderr and result                            | `error` (traceback) / `terminal` / `code` / `text`                             | Medium - errors need visibility             |
| `cronjob`            | JSON object or array of jobs                                      | `json` / `json_array`                                                          | Low - short scheduling info                 |
| `delegate_task`      | Narrative summary with task results                               | `text` (reclassified on JSON, error, or diff patterns)                         | High - delegate outputs can be very verbose |
| `session_search`     | JSON `{total_count, query, results}`                              | `search` / `json_array` / `json`                                               | Medium - scales with session length         |
| `memory`             | Confirmation or `{memories: [...]}`                               | `json` / `text`                                                                | Low - short snippets                        |
| `skill_view`         | Markdown with YAML frontmatter                                    | `text` / `markdown` / `json`                                                   | Medium - skills can be 100+ lines           |
| `skill_manage`       | Confirmation or JSON `{action, name, path}`                       | `json` / `text`                                                                | Low - short                                 |
| `skills_list`        | Pipe-delimited table or JSON array                                | `table` / `json_array`                                                         | Medium - many skills make long tables       |
| `clarify`            | Short question text                                               | `text`                                                                         | Very low                                    |
| `vision_analyze`     | Description text or JSON `{description, objects, text}`           | `json` / `text` / `error`                                                      | Medium                                      |
| `computer_use`       | Screenshot description plus action result                         | `json` / `text` / `error`                                                      | Low-medium                                  |
| `browser_navigate`   | JSON `{url, title, status}` or confirmation                       | `json` / `text`                                                                | Low - short                                 |
| `browser_snapshot`   | Large accessibility tree / DOM array                              | `json_array` / `json` / `table`                                                | Very high - trees can be 10 KB+             |
| `browser_click`      | Structured result or narrative confirmation                       | `json` / `text`                                                                | Low-medium - includes snapshot delta        |
| `browser_type`       | Structured result or confirmation                                 | `json` / `text`                                                                | Low - very short                            |
| `browser_scroll`     | Structured result or plain text                                   | `json` / `text`                                                                | Low - very short                            |
| `browser_console`    | Log entries `[{level, message, timestamp}]`                       | `log` / `json_array` / `error`                                                 | High - console logs verbose                 |
| `browser_back`       | Structured result or plain text                                   | `json` / `text`                                                                | Low - very short                            |
| `browser_vision`     | Description text or JSON                                          | `json` / `text` / `error`                                                      | Medium - longer than `vision_analyze`       |
| `browser_get_images` | JSON array of image URLs/descriptions                             | `json_array`                                                                   | Low-medium                                  |
| `browser_press`      | Structured result or plain text                                   | `json` / `text`                                                                | Low - very short                            |
| `image_generate`     | JSON `{image}` or `MEDIA:path`                                    | `json` / `text`                                                                | Low - short                                 |
| `text_to_speech`     | `MEDIA:path` audio reference plus confirmation                    | `json` / `text`                                                                | Low - short                                 |
| `web_search`         | JSON array of results `[{title, url, snippet}]`                   | `search` / `json_array` / `json`                                               | High - snippets verbose                     |
| `todo`               | JSON array, table, or markdown list                               | `json_array` / `table` / `text`                                                | Low-medium                                  |

### Aphrodite meta-tools

The plugin ships thirteen `aphrodite_*` tools; the ten below are the ones the
output catalog tracks. Meta-tool output is auto-expanded inline - never
compressed - so the agent always sees it (the remaining three, `directive`,
`prefetch`, and `prefetch_status`, return short status text classified as
`json`/`text`).

| Tool                   | Typical output shape                                      | Classification                                    |
| ---------------------- | --------------------------------------------------------- | ------------------------------------------------- |
| `aphrodite_catalog`    | Markdown table with hash, type, size, and preview columns | `table` - auto-expanded                           |
| `aphrodite_compress`   | Programmatic CCR create confirmation / JSON               | `json` - auto-expanded                            |
| `aphrodite_diff`       | Structured per-turn compression summary                   | `text` - auto-expanded                            |
| `aphrodite_files`      | Grouped file listing                                      | `text` - auto-expanded                            |
| `aphrodite_rebuild`    | Build lines or structured response                        | `build_output` / `error` / `json` - auto-expanded |
| `aphrodite_retrieve`   | Original uncompressed content with markers resolved       | The original content's own type - auto-expanded   |
| `aphrodite_search`     | Catalog search results                                    | `text` / `search` - auto-expanded                 |
| `aphrodite_stats`      | Structured key-value status                               | `text` - auto-expanded                            |
| `aphrodite_test`       | Smoke-test results                                        | `json` / `build_output` - auto-expanded           |
| `aphrodite_reclassify` | Reclassification result                                   | `json` - auto-expanded                            |

### Special entries

| Entry             | What it is                                                                                | Behavior                                                         |
| ----------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `CCR_MARKER`      | Output that is already a CCR marker (`<<<CCR:hash\|type\|size>>>`)                        | Detected by marker pattern; never re-compressed                  |
| `CLASSIFIER_SKIP` | Clean, inert output: builds with 0 errors/warnings, exit=0 terminals, zero-match searches | Left as-is - the preview is the complete story, no marker needed |

## Skip Gates

Compression only happens when the result is eligible. A result is left
untouched when it is empty; when the tool is in the essential set - the
agent needs raw output; when the tool is one of Aphrodite's own or a
headroom helper; or when the result is below the threshold.

| Gate            | Applies to                                                                                            | Setting (default)                                           |
| --------------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Essential tools | `skill_view`, `skills_list`, `skill_manage`, `memory`, `session_search`, `read_file`, `read_terminal` | always raw - never compressed                               |
| Self tools      | `aphrodite_*` and headroom helpers - already compact metadata                                         | always raw                                                  |
| Tool threshold  | All other tool results                                                                                | `tool_threshold_token` (4,096 chars; `0` = always compress) |
| Terminal gate   | Terminal output without chain-split markers                                                           | `terminal_threshold` (1,024 chars; `0` = always compress)   |

## Preview Forms

The preview each type carries is compact and type-aware. Representative
examples from the enriched catalog:

```text
[build:1E 1W 142L | error[E0432]: unresolved import ...]
[diff:2F +7/-3 12L | src/main.rs Cargo.toml +N more]
[git:2M 2A 1D 3?? | src/x.rs src/y.rs +5 more]
[gitlog:2 commits | abc1234 fix... → def5678 feat...]
[ls:3 files 2 dirs | .rs x2 .md x1]
[test:220 pass 0 fail 1 ignored | 0.31s]
[grep:4 hits in 3 files | src/preview.rs:12 ...]
[code:3fns 2structs fn main() 414L]
[search:Nhits ML]
[json:5 items 3L]
[terminal:14L exit code: 0]
[text:3L 50B | some unrecognizable prose here]
```

When the classifier only reaches a generic bucket (`text`/`terminal`/`log`/
empty), a semantic detector upgrades the arm to a high-signal shape (git
status, git log, grep, ls, test) before the preview is built. The rendered
preview length is capped by `preview_max_chars` (default 120;
`APHRODITE_PREVIEW_MAX_CHARS` overrides it, and environment beats TOML).
The [Enriched Preview
Catalog](../proxy/compression.md#enriched-preview-catalog) is authoritative
for the exact emitted strings.

## See also

- [Content Type Taxonomy](../classification/content-types.md) - the 30-type registry, detection order, and threshold groups
- [Enriched Preview Catalog](../proxy/compression.md#enriched-preview-catalog) - exact preview strings for every type
- [Plugin Hooks](../plugin/hooks.md) - the skip gates and thresholds in context
- [Marker Format](../ccr/marker-format.md) - the `<<<CCR:hash|type|size>>>` marker layout
