# Hermes Tool Output Schemas

This covers Hermes Agent's own tool surface rather than Aphrodite's code, so
treat it as reference for the agent side; see
[CCR: Content Types](ccr/content-types.md) for the classification taxonomy
it documents. Its machine-readable companion is
`Maintain/hermes_tool_output_formats.json`.

> **Every tool your agent runs produces output with a unique shape. This
> document maps every single one - 43 tools, 22 classification types, 100+
> extraction patterns. The absorptive classifier uses this as its playbook.**

Comprehensive reference of ALL Hermes tool output formats, their classification
types, and extraction patterns. The single source of truth for the absorptive
CCR preview pipeline. When new tools or output shapes appear, they get
documented here first - the classifier follows.

> **Preview shapes:** the `[type:...]` strings below illustrate the *fields*
> each tool's preview surfaces. The exact enriched shapes emitted by default
> (git status, git log, grep, ls, test, build-with-first-error, diff-with-file-
> names, code-with-signature, terminal, first-line text fallback) are the ones
> in [Enriched Preview Catalog](proxy/compression.md#enriched-preview-catalog),
> which is authoritative for the emitted format.

---

## Classification Taxonomy (22 types)

| Type               | Detects                  | Key fields                 |
| ------------------ | ------------------------ | -------------------------- |
| `diff`             | Unified diff patches     | files, fn, +, -, ln        |
| `terminal`         | Shell command output     | exit, cmd, last, ln        |
| `build_output`     | Cargo/build/test output  | errors, warnings, ln       |
| `build_error`      | Rust error[E] patterns   | code, loc, ln              |
| `error`            | Traceback/panic/Error    | msg, ln                    |
| `code`             | Source code (generic)    | fns, structs, ln           |
| `code_rust`        | Rust source              | fns, structs, impls        |
| `code_python`      | Python source            | fns, classes               |
| `code_go`          | Go source                | fns, types                 |
| `code_js`          | JS/TS source             | fns, classes               |
| `json`             | JSON object              | keys, ln                   |
| `json_list`        | JSON array               | len, ln                    |
| `search_files`     | Grep results             | files, q, total            |
| `search_results`   | Search API results       | total, q                   |
| `tabular`          | Pipe-delimited tables    | rows, ln                   |
| `commit`           | Git commit log           | hash, subject              |
| `process_output`   | Background process       | pid, uptime                |
| `log`              | Structured log entries   | entries, errs, warns       |
| `write_file`       | File write confirmations | path, bytes, syntax_errors |
| `browser_snapshot` | Page accessibility trees | elements, title            |
| `skill_view`       | Skill markdown docs      | name, desc, sections       |
| `text`             | Unrecognized (fallback)  | first, ln                  |

---

## Tool-by-Tool Reference

### read_file

- **Shape:** `LINE_NUM|CONTENT\n` repeated, optional hint footer
- **Classify:** `text` → reclassified to `code_*` if language detected, `json`
  if parseable
- **Extract:** path (fn), extension, line_count, code structure
- **Compress:** High - large files benefit from structure preview
- **Preview:** `[code_rust:3fns|2structs src/proxy.rs 414L]`

### write_file

- **Shape:** JSON `{status, path, bytes, syntax_errors}` or plain confirmation
- **Classify:** `json` / `text` / `error` (if syntax errors)
- **Extract:** path, status, bytes, syntax_errors count
- **Compress:** Low - typically short
- **Preview:** `[text:File written successfully: /path/to/file.py]`

### patch

- **Shape:** Unified diff format
  `--- a/path\n+++ b/path\n@@ ... @@\n context\n-removed\n+added`
- **Classify:** `diff`
- **Extract:** file_count, insertions, deletions, line_count, filename
- **Compress:** Medium - diffs compress well
- **Preview:** `[diff:3f +12/-3 42L src/main.rs]`

### search_files

- **Shape:** Three modes: content (`file:line:content`), files_only, count
- **Classify:** `search_files` (file:line pattern), `search_results` (JSON with
  total_count)
- **Extract:** query, match_count, file_count, line_count
- **Compress:** Medium - large result sets benefit
- **Preview:** `[grep:25 matches src/proxy.rs:841 30L]`

### terminal

- **Shape:** Raw stdout/stderr + optional `exit code: N`
- **Classify:** `terminal` (exit code), `build_output`, `build_error`, `error`
- **Extract:** command, exit_code, error_count, warning_count, last_line
- **Compress:** Very high - verbose output with dramatic savings
- **Preview:** `[terminal:cargo build exit=0]` or `[build:1E 2W 142L]`

### process

- **Shape:** Varies - list (table/JSON), poll (delta text), log (paginated),
  kill/write (confirmation)
- **Classify:** `process_output` (JSON with session_id), `terminal` (exit code),
  `json`, `text`
- **Extract:** session_id, pid, uptime, state, action_result
- **Compress:** Medium - log outputs verbose, poll deltas short
- **Preview:** `[process:pid=12345 up=2h 10L]`

### execute_code

- **Shape:** stdout text + optional stderr + execution result
- **Classify:** `error` (traceback), `terminal` (exit code), `code` (code-like
  output), `text`
- **Extract:** result, error_message, trace_location, execution_time
- **Compress:** Medium - errors need visibility, success is short
- **Preview:** `[error:NameError 'foo' is not defined 8L]` or `[text:42 1L]`

### cronjob

- **Shape:** JSON `{id, schedule, status, last_run, next_run}` or array
- **Classify:** `json_list` / `json`
- **Extract:** job_count, status_summary, schedule
- **Compress:** Low - short scheduling info
- **Preview:** `[json:5 items 3L]`

### delegate_task

- **Shape:** Narrative summary text with task results
- **Classify:** `text` (may reclassify if JSON/error patterns)
- **Extract:** summary, file_count, line_count
- **Compress:** High - delegate outputs can be very verbose
- **Preview:** `[text:Task completed: analyzed 42 files, found 3 issues...]`

### session_search

- **Shape:** `{total_count, query, results: [{turn, content, score}]}`
- **Classify:** `search_results`
- **Extract:** query, total_count, turn_numbers
- **Compress:** Medium - scales with session length
- **Preview:** `[grep:8 matches compression 3L]`

### memory

- **Shape:** Confirmation text or `{memories: [{id, content}]}`
- **Classify:** `json` / `text`
- **Extract:** memory_count, content_preview, operation_type
- **Compress:** Low - short snippets
- **Preview:** `[text:Memory stored: 'prefers Rust for backend...']`

### skill_view

- **Shape:** Markdown with YAML frontmatter
- **Classify:** `text`
- **Extract:** skill_name, description, line_count
- **Compress:** Medium - skills can be 100+ lines
- **Preview:** `[text:Skill 'dev-workflow' loaded - 120L]`

### skill_manage

- **Shape:** Confirmation text or JSON `{action, name, version, path}`
- **Classify:** `json` / `text`
- **Extract:** action, skill_name, version, path
- **Compress:** Low - short
- **Preview:** `[text:Skill 'my-skill' installed 2L]`

### skills_list

- **Shape:** Markdown table or JSON array
- **Classify:** `tabular` (pipe table >3 rows), `json_list`
- **Extract:** skill_count, names, line_count
- **Compress:** Medium - many skills produce long tables
- **Preview:** `[table:12 rows 15L]`

### clarify

- **Shape:** Question text - short string prompt
- **Classify:** `text`
- **Extract:** question_text, option_count
- **Compress:** Very low
- **Preview:** `[text:Which file did you mean: src/main.rs or src/lib.rs?]`

### vision_analyze

- **Shape:** Descriptive text or JSON `{description, objects, text}`
- **Classify:** `json` / `text` / `error`
- **Extract:** description_preview, object_count, detected_text
- **Compress:** Medium
- **Preview:**
  `[text:Image analysis: terminal window showing 'cargo build' output...]`

### computer_use

- **Shape:** Screenshot description + action result
- **Classify:** `json` / `text` / `error`
- **Extract:** action_type, result_summary, coordinates
- **Compress:** Low-medium
- **Preview:** `[text:Clicked at (450, 320) on 'Submit button']`

### browser_navigate

- **Shape:** `{url, title, status, state}` or confirmation text
- **Classify:** `json` / `text`
- **Extract:** url, title, status_code
- **Compress:** Low
- **Preview:** `[text:Navigated to https://example.com - Example Domain]`

### browser_snapshot

- **Shape:** Large accessibility tree / DOM array
- **Classify:** `json_list` / `json` / `tabular`
- **Extract:** element_count, page_title, interactive_count
- **Compress:** Very high - can be 10KB+
- **Preview:** `[json:342 items page=Example Domain 500L]`

### browser_click / browser_type / browser_scroll / browser_back / browser_press

- **Shape:** Short confirmation JSON/text
- **Classify:** `json` / `text`
- **Extract:** element, action_result
- **Compress:** Low - short
- **Preview:** `[text:Clicked 'Submit button']`

### browser_console

- **Shape:** Array of log entries `[{level, message, timestamp}]`
- **Classify:** `log` (new type) / `json_list`
- **Extract:** entry_count, error_count, warning_count, first_message
- **Compress:** High - console logs verbose
- **Preview:** `[log:42 entries 3E 5W 100L]`

### browser_vision

- **Shape:** Descriptive text or JSON with `{description, elements_detected}`
- **Classify:** `json` / `text` / `error`
- **Extract:** description, element_count, text_found
- **Compress:** Medium
- **Preview:** `[text:Screenshot: Form with 5 inputs, 2 buttons...]`

### browser_get_images

- **Shape:** JSON array of image URLs/descriptions
- **Classify:** `json_list`
- **Extract:** image_count, url_previews
- **Compress:** Low-medium
- **Preview:** `[json:12 items 3L]`

### image_generate

- **Shape:** `{image: "url-or-path"}` or `MEDIA:path`
- **Classify:** `json` / `text`
- **Extract:** image_url, prompt, aspect_ratio
- **Compress:** Low
- **Preview:** `[text:Image generated: /path/to/image.png]`

### text_to_speech

- **Shape:** `MEDIA:path` audio reference with confirmation
- **Classify:** `text`
- **Extract:** audio_path, text_length
- **Compress:** Low
- **Preview:** `[text:TTS generated: /voice-memos/20260617-...mp3]`

### web_search

- **Shape:** Array of search results `[{title, url, snippet}]`
- **Classify:** `json_list` / `search_results`
- **Extract:** result_count, query, source_urls
- **Compress:** Medium
- **Preview:** `[json:10 items search='rust tokio' 5L]`

### todo

- **Shape:** Task list JSON array `[{id, content, status}]`
- **Classify:** `json_list`
- **Extract:** task_count, pending_count, completed_count
- **Compress:** Low-medium
- **Preview:** `[json:5 items 3 pending 1 in_progress 1 completed]`

### aphrodite_catalog

- **Shape:** Markdown table `| Hash | Type | Size | Preview |`
- **Classify:** `tabular`
- **Extract:** item_count, bytes_saved, turn_count, type_summary
- **Compress:** Auto-formatted - always visible to LLM
- **Preview:** Auto-expanded (no CCR marker)

### aphrodite_stats

- **Shape:** Structured text `proxy:\n  token: on N created ...`
- **Classify:** `text` (structured)
- **Extract:** proxy_status, token_created, cache_created, engine_status
- **Compress:** Auto-formatted
- **Preview:** Auto-expanded

### aphrodite_diff / aphrodite_files

- **Shape:** Structured text with turn/file lists
- **Classify:** `text`
- **Extract:** turn_count, file_count
- **Compress:** Auto-formatted
- **Preview:** Auto-expanded

### aphrodite_retrieve

- **Shape:** Original uncompressed content with CCR markers resolved
- **Classify:** Whatever the original content type is
- **Extract:** Depends on content
- **Compress:** Medium - may be large if resolving nested markers
- **Preview:** As classified from retrieved content

---

## New Classifier Types to Add

### log (browser_console, structured logs)

```
Pattern: JSON array with entries containing "level"/"message"/"timestamp"
Fields: {entries} = count, {errs} = error count, {warns} = warning count
Preview: [log:{entries} entries {errs}E {warns}W {ln}L]
```

### write_file (enhanced detection)

```
Pattern: JSON with "status": "written" and "path"/"bytes"/"syntax_errors"
Fields: {path} = file path, {bytes} = size, {syntax_errors} = error count
Preview: [file:{path} {bytes}B] or [file:{path} {syntax_errors} syntax errors]
```

### browser_snapshot (enhanced)

```
Pattern: JSON with large "elements" array and/or "total_elements" count
Fields: {elements} = element count, {title} = page title
Preview: [dom:{elements} elements {title} {ln}L]
```
