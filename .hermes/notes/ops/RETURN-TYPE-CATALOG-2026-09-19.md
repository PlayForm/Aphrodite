# Return-Type Catalog - 2026-09-19

Complete reference for every envelope and content type the Aphrodite CCR
engine and its Hermes feeders can emit. Companion to
`ENVELOPE-MODEL-2026-09-19.md` (model verification + separation proposal).
All references are real `file:line`; observational evidence cited from
`.hermes/examples/` (real captured markers).

## 1. Envelope side - every Hermes tool result envelope

Hermes wraps every tool result in JSON; the Aphrodite unwrap
(`crates/aphrodite-hermes/src/tools.rs:67-233`) recognizes six families.
`✓` = recognized (family), `GAP` = not recognized (falls to generic
classification). Sources: `~/.hermes/hermes-agent/tools/`.

| Tool                     | Envelope keys                                                                                                           | Example (shape)                                                   | Unwrap                                     |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------ |
| terminal                 | `output`, `exit_code`, `error` (+`cwd`, `full_output_path`, `truncation_note`, …)                                       | `{"output":"…","exit_code":0,"error":null}`                       | F1 terminal ✓                              |
| patch                    | `success`, `diff`, `files_modified`, `files_created`, `lint`, `lsp_diagnostics`, `error`                                | `{"success":true,"diff":"…","files_modified":["/p"]}`             | F2 ✓ via `diff`/`error`                    |
| write_file               | `bytes_written`, `dirs_created`, `verified`, `lint`, `lsp_diagnostics`, `error`, `warning`                              | `{"bytes_written":123,"verified":true,"lint":{"status":"ok"}}`    | GAP (success); error ✓                     |
| read_file                | `content`, `total_lines`, `file_size`, `truncated`, `hint`, `is_binary`, `error`, `similar_files`                       | `{"content":"1\|…","total_lines":42,"truncated":true}`            | F5 content ✓                               |
| search_files             | `total_count`, `matches` / `matches_format`+`matches_text`, `files`, `truncated`, `error`                               | `{"total_count":22,"matches_text":"p\n 12: …","truncated":false}` | F4 search ✓                                |
| skill_view               | `success`, `name`, `description`, `tags`, `content`, `path`, `linked_files`, `usage_hint`                               | `{"success":true,"name":"x","description":"…","content":"…"}`     | GAP (multi-key)                            |
| skills_list              | `success`, `skills`, `categories`, `message`                                                                            | `{"success":true,"skills":[],"message":"No skills found"}`        | GAP                                        |
| todo_list                | `todos`, `revision`, `summary{total,pending,…}`                                                                         | `{"todos":[…],"revision":3,"summary":{"total":2}}`                | GAP                                        |
| memory (write)           | `success`, `staged`, `pending_id`, `message`, `proposal_staged`                                                         | `{"success":true,"staged":true,"pending_id":"…"}`                 | GAP (success); error ✓                     |
| cronjob                  | `success`, `forwarded_to_gateway`, `note` / `error`                                                                     | `{"success":true,"forwarded_to_gateway":true,"note":"…"}`         | GAP (success); error ✓                     |
| browser_*                | `success`, `url`, `title`, `snapshot`, `element_count`, `typed`, `element`, `result`, `result_type`, `analysis`, `data` | `{"success":true,"url":"…","title":"…","element_count":12}`       | GAP (success); error ✓                     |
| web_search               | `success`, `data{web[…]}`                                                                                               | `{"success":true,"data":{"web":[{"title":"…"}]}}`                 | GAP (success); error ✓                     |
| web_extract              | `results` (array)                                                                                                       | `{"results":[{"title":"…"}]}`                                     | GAP (`results` ∉ F6 keys)                  |
| image_generate           | `success`, `image`, `modality`, `upscaled` / `error`                                                                    | `{"success":true,"image":"…","modality":"text"}`                  | GAP (success); error ✓                     |
| execute_code (local)     | `status`, `output`, `exit_code`, `tool_calls_made`, `duration_seconds`, `kernel`, `error`                               | `{"status":"ok","output":"…","exit_code":0,"kernel":{…}}`         | F1 ✓ when output non-empty                 |
| execute_code (remote)    | `status`, `output`, `tool_calls_made`, `duration_seconds` (no `exit_code`)                                              | `{"status":"error","error":"…","tool_calls_made":2}`              | GAP (success); error ✓                     |
| process (exited)         | `status`, `command`, `exit_code`, `output`                                                                              | `{"status":"exited","exit_code":0,"output":"…"}`                  | F1 ✓                                       |
| process (timeout / list) | `status`,`command`,`output`,`process_running`,`timeout_note` / top-level array                                          | `{"status":"timeout","process_running":true}`                     | GAP                                        |
| delegate_task            | `results[{status,summary,…}]`, `total_duration_seconds`                                                                 | `{"results":[{"status":"completed","summary":"…"}]}`              | GAP                                        |
| session_search           | `success`, `mode`, `query`, `detail`, `results`, `count`, `message`                                                     | `{"success":true,"mode":"discover","results":[…],"count":3}`      | GAP (success); error ✓                     |
| desktop_project          | `success`, `id`, `slug`, `name`                                                                                         | `{"success":true,"id":"…","slug":"…","name":"…"}`                 | GAP (success); error ✓                     |
| annotate_preview         | passthrough JSON or `text`                                                                                              | `{"text":"…"}`                                                    | GAP (`text` ∉ F6 keys)                     |
| vision_analyze           | `success`, `analysis`, `scale_note`                                                                                     | `{"success":true,"analysis":"…"}`                                 | GAP (success); error ✓                     |
| tool_error (all)         | `error` (str) + extras                                                                                                  | `{"error":"…","status":"error"}`                                  | F2 error ✓                                 |
| aphrodite_compress       | `hash`, `type`, `size`, `preview`, `marker`                                                                             | `{"hash":"…","type":"json","size":1728,"marker":"<<<CCR:…>>>"}`   | self-tool (hook skips, `hooks.rs:161-163`) |
| aphrodite_retrieve       | `found`, `source` (`path` \| `ccr`), `hash` \| `path`, `content`                                                        | `{"found":true,"source":"ccr","hash":"…","content":"…"}`          | self-tool                                  |

Producer citations: `tools/terminal_tool_result.py:258`; `tools/file_operations_common.py:29-30,48-49,73-83,127-153`;
`tools/file_tools.py:650-704,811-868,952-985,1047-1069`; `tools/skills_tool.py:248-255,631-644`;
`tools/todo_tool.py:210-211`; `tools/memory_tool.py:77,159-161`; `tools/cronjob_tools.py:143-157`;
`tools/browser_tool.py:768,817,893,1013,1200,1256`; `tools/web_tools.py:271-272,305-311,396-399`;
`tools/image_generation_tool.py:472-476`; `tools/code_kernel.py:727-755`; `tools/code_execution_tool.py:521-526`;
`tools/process_registry.py:1849-1862,1868-1870,2374`; `tools/delegate_tool_dispatch.py:185-196`;
`tools/session_search_tool.py:90-91,310,446,493`; `tools/project_tools.py:62-63` (UNVERIFIED - file not
independently re-read); `tools/annotate_preview_tool.py:44-46`; `tools/vision_tools.py:752,766`;
`tools/registry.py:999-1002`. UNVERIFIED: `optional-mcps/` connector envelopes (MCP layer, not inspected).

## 2. Content side - every content type the classifiers can emit

### 2.1 Headroom `detect_content_type` (Rust, used by `detect_type`, `preview.rs:13-18`)

`vendor/headroom/crates/headroom-core/src/transforms/content_detector.rs:33-56`:

| Type          | Detection rule                                                                        | Example                  |
| ------------- | ------------------------------------------------------------------------------------- | ------------------------ |
| `json_array`  | strict parse + `as_array()` (272-283); object → NOT this                              | `[{“a”:1},{“a”:2}]`      |
| `source_code` | per-language pattern votes (python/js/ts/go/rust/java/csharp/…, 110+)                 | `fn main() {}`           |
| `search`      | `path:line:` regex majority (82)                                                      | `src/a.rs:12: let x = 1` |
| `build`       | compiler/test/lint verb patterns                                                      | `Compiling foo v0.1.0`   |
| `diff`        | `diff --git` / `--- a/` / `@@ -A,B +C,D @@` / `@@@` (92-97)                           | `diff --git a/x b/x`     |
| `html`        | `<!DOCTYPE html` / `<html` open                                                       | `<!DOCTYPE html><body>…` |
| `text`        | fallback - **JSON objects land here** (test `json_object_falls_through_to_text` :570) | `plain prose`            |

Python enum adds `tabular` + `structured_config` (`headroom/transforms/content_detector.py:25-36`);
the Rust port does not emit them.

### 2.2 `detect_semantic_type` (Aphrodite layer, `preview.rs:37-251`)

| Type       | Rule (file:line)                                                                                               | Example                                   |
| ---------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `json`     | strict parse of object/array, excluded if any key in GUARD (56-66, 255-275)                                    | `{"a":1,"b":2}`                           |
| `test`     | `test result:` / `=== RUN` / `N passed` / `running N tests` (73-85)                                            | `test result: ok. 3 passed`               |
| `diff`     | `diff --git` or `---`+`+++` or `@@`+delta (88-107)                                                             | `@@ -1,3 +1,4 @@\n+line`                  |
| `code`     | strong signature or ≥2 statement votes (112-116)                                                               | `fn add(a: i32) -> i32`                   |
| `table`    | ≥2 `                                                                       \|` lines + separator row (119-130) | `  \| a   \| b   \| \n  \| --- \| --- \|` |
| `markdown` | ≥2 headings + structure/body (136-141)                                                                         | `# Title\n- item`                         |
| `yaml`     | ≥3 lowercase key lines (146-149)                                                                               | `name: webapp\nport: 80`                  |
| `html`     | `<!DOCTYPE html` / `<html` (154-157)                                                                           | `<html><title>x</title>`                  |
| `xml`      | `<` open + `</` close (158-160)                                                                                | `<root><a/></root>`                       |
| `csv`      | ≥2 rows, identical field count, no `, ` (164-171)                                                              | `a,b,c\n1,2,3`                            |
| `build`    | ≥2 verb/error/warning markers (177-200)                                                                        | `error[E0308]: mismatched types`          |
| `git`      | porcelain majority (205-208)                                                                                   | ` M src/x.rs\n?? new.rs`                  |
| `ls`       | `ls -l` mode strings / bare path majority (231-248)                                                            | `-rw-r--r-- 1 u g 12 x`                   |
| `grep`     | `path:line:match` majority (224-227)                                                                           | `a.rs:12: let x = 1`                      |
| `gitlog`   | `commit <hash>` blocks (211-221)                                                                               | `commit a1b2c3d\nAuthor: x`               |

### 2.3 Proxy classifier (`proxy.rs:1325-1469`) - standalone proxy path

`tool_output` (JSON w/ `exit_code`/`status`, 1335-1337), `json` (valid JSON
otherwise, 1338), `code_rust`/`code_python`/`code_go`/`code_js`/`code`
(1342-1388), then `detect_semantic_type` passthrough (1397-1399), then
`error` (1402-1410), `build_output` (1413-1420), `linter` (1423-1434),
`diff` (1437-1443), `git` (1446-1448), `log` (1451-1467), `text` (1468).
Note: the proxy does NOT unwrap envelopes - it labels them `tool_output`
(open question 2 in the main doc).

### 2.4 `unwrap_hermes_result` emitted types (`tools.rs`)

`terminal`, `build_output`, `build_error` (85-103), `text` (116,119,136,201,226),
`search` (204), plus `detect_type` results for diff/content payloads.

### 2.5 Preview arms (`build_preview`, `preview.rs:714-980`) + reverse map

`build_preview` arms: build family (`build`/`build_output`/`build_error`, 744-789),
`diff` (790-829), `git`/`git_status` (831), `gitlog`/`git_log` (833),
`ls`/`dir`/`directory` (835), `test`/`test_output` (837), `grep`/`ripgrep` (839),
code family `source_code`/`code_*`/`code` (840-871), `search` (872),
`html` (873), `table`/`markdown_table`/`md_table` (877), `markdown`/`md` (878),
`yaml` (879), `xml` (880), `csv` (881), `json_array`/`json`/`json_list` (882),
`terminal` (895-909), `error` (917-928), `linter`/`lint` (931-942),
`log` (946-957), generic `_` (968-976). Generic buckets
(`text`/`terminal`/`log`/`""`/`plain`/`tool_result`) are semantically upgraded
first (733-742). Preview cap applied at 978-979.

`[templates.reverse]` keys (`templates/aphrodite.toml:269-297`): `build_output`,
`build_error`, `error`, `diff`, `terminal`, `commit`, `search_files`,
`search_results`, `tabular`, `json`, `json_list`, `process_output`,
`write_file`, `log`, `browser_snapshot`, `web_search`, `image_generate`,
`todo`, `memory`, `cronjob`, `code`, `code_rust`, `code_python`, `code_go`,
`code_js`, `code_ts`, `code_sh`, `text → _default`. Template families:
compact (130-154), code_first (158-187, active: `model_family = "code_first"`,
line 66), balance (191-213). Marker template: `[templates.marker]` (217-226);
Rust renderer `marker.rs:97-108` emits `<<<CCR:hash|type|size>>>` + preview
line (+ optional `[meta:…]`).

### 2.6 Master content-type table (rule → example → preview)

| Type                   | Produced by                                          | Preview arm / template                    | Example preview (captured where available)                                                                                |
| ---------------------- | ---------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `json`                 | semantic 56-66; proxy 1338                           | `json` (882) / `[json:{keys} {ln}L]`      | `[json:5keys 116L \| a, b, c, items, summary]` (`.hermes/examples/templates/json-preview-code_first.md:45`)               |
| `json_array`           | headroom 47                                          | `json` arm                                | `[json:{items} items {ln}L]`                                                                                              |
| `diff`                 | headroom 52; semantic 88-107; unwrap 111; proxy 1442 | `diff` (790-829)                          | `[diff:1F +31/-0 40L \| crates/aphrodite/src/preview.rs]` (`.hermes/examples/templates/diff-preview-code_first.md:56`)    |
| `build_error`          | unwrap 97                                            | build family (744-789)                    | `[build:2E 0W 11L \| error[E0308]: mismatched types]` (`.hermes/examples/templates/build_error-preview-code_first.md:45`) |
| `build_output`         | unwrap 99; proxy 1419                                | build family                              | `[build:{err}E {warn}W {ln}L]`                                                                                            |
| `build`                | headroom 50; semantic 198                            | build family                              | `[build:{err}E {warn}W {ln}L]`                                                                                            |
| `terminal`             | unwrap 86; hook force (`hooks.rs:379-381`)           | terminal (895-909)                        | `[terminal:{cmx} exit={exit}]`                                                                                            |
| `search`               | unwrap 204                                           | search (872, `build_search_preview` 1305) | `[grep:{files} matches {ln}L]`                                                                                            |
| `source_code`/`code_*` | headroom 48; proxy 1342-1388                         | code (840-871)                            | `[code:9fns fn new(cap:usize) -> Arc<Self> 112L]` (`.hermes/examples/previews/model_family-code_first.md:63`)             |
| `text`                 | fallback everywhere                                  | generic `_` (968-976)                     | `[text:{ln}L {size}B \| {first meaningful line}]`                                                                         |
| `error`                | proxy 1409                                           | error (917-928)                           | `[error:{ln}L {size}B \| {first error line}]`                                                                             |
| `linter`               | proxy 1433                                           | linter (931-942)                          | `[lint:{ln}L {size}B \| {first issue}]`                                                                                   |
| `log`                  | proxy 1466                                           | log (946-957)                             | `[log:{ln}L {size}B \| {error or tail}]`                                                                                  |
| `git`/`git_status`     | semantic 207; proxy 1447                             | git (831)                                 | `[git:5M 2A \| src/x.rs +6 more]`                                                                                         |
| `gitlog`               | semantic 220                                         | gitlog (833)                              | `[gitlog:{commits} commits {first} {last}]`                                                                               |
| `ls`                   | semantic 242-248                                     | ls (835)                                  | `[ls:68 files 32 dirs \| .json×13 .txt×13 .py×11]` (captured, previews/model_family-code_first.md:45)                     |
| `test`                 | semantic 84; preview upgrade 737                     | test (837)                                | `[test:0 pass 0 fail 0 ignored]` (captured, previews/model_family-code_first.md:54)                                       |
| `grep`/`ripgrep`       | semantic 226                                         | grep (839)                                | `[grep:{files} hits {ln}L \| {first loc}]`                                                                                |
| `table`/`tabular`      | semantic 129; python headroom 34                     | table (877)                               | `[table:{items} rows {ln}L]`                                                                                              |
| `markdown`             | semantic 139                                         | markdown (878)                            | `[markdown:{ln}L {size}B \| {first heading}]`                                                                             |
| `yaml`                 | semantic 148                                         | yaml (879)                                | `[yaml:{keys} keys {ln}L \| {first key}]`                                                                                 |
| `xml`                  | semantic 158                                         | xml (880)                                 | `[xml:{ln}L {size}B \| {root}]`                                                                                           |
| `csv`                  | semantic 167                                         | csv (881)                                 | `[csv:{rows} rows {ln}L]`                                                                                                 |
| `html`                 | headroom 52; semantic 155                            | html (873)                                | `[html:{title} {ln}L]`                                                                                                    |
| `tool_output`          | proxy 1336 (envelope guess)                          | - (proxy-only label)                      | -                                                                                                                         |

## 3. Cross-product summary - envelope → content → preview → what the model sees

| Hermes envelope                                                                              | Typical content type(s)                                                                  | Preview template                 | What the model sees (one line)                                                                                                                      |
| -------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| terminal                                                                                     | `terminal`, `build_output`, `build_error`, `ls`, `test`, `git`, `code_*`, `json`, `text` | terminal / build / semantic arms | `[terminal:N L <exit-or-first-line>]` or `[build:NE NW NL                     \| first error]` or the semantic preview - enough to decide retrieval |
| patch                                                                                        | `diff` (payload), `text` (error)                                                         | diff arm                         | `[diff:F f +A/-D L                                                            \| file … +more]` - files + churn                                     |
| write_file (success)                                                                         | `text`/`json` (GAP today)                                                                | generic                          | `[text:NL B                                                                   \| first line]` (no envelope facts)                                   |
| read_file                                                                                    | `text`, `json`, `code_*`, `yaml`, `markdown`, …                                          | matching arm                     | type label may say `text` while preview renders the real shape (mismatch, §7)                                                                       |
| search_files                                                                                 | `search`                                                                                 | search arm                       | `[grep:N hits in F files                                                      \| first loc]` - real hit count (WS2)                                 |
| skill_view                                                                                   | `text` (GAP - multi-key)                                                                 | generic                          | `[text:…]` key-less listing, no name/description extraction                                                                                         |
| todo / memory / cronjob / browser / web_search / image / delegate / session_search (success) | `json` or `text` (GAP)                                                                   | json / generic                   | `[json:keys L]` or `[text:…]` - shape preserved, envelope facts lost                                                                                |
| error envelopes (any tool)                                                                   | `text`                                                                                   | generic                          | `[text:…]` message text (F2 error branch)                                                                                                           |
| web_extract                                                                                  | `json`                                                                                   | json arm                         | `[json:keys L                                                                 \| first keys]`                                                       |
| aphrodite_* results                                                                          | self-tool - hook skips (hooks.rs:161-163)                                                | n/a                              | never compressed                                                                                                                                    |
| execute_code (local, non-empty)                                                              | `terminal`/`build_*`/`json`                                                              | terminal / build / json          | same as terminal row                                                                                                                                |
| execute_code (remote success)                                                                | `text`/`json` (GAP - no exit_code)                                                       | generic / json                   | shape preserved, `status` lost                                                                                                                      |
| process (exited)                                                                             | `terminal`/`build_*`                                                                     | terminal / build                 | same as terminal row                                                                                                                                |
| process (timeout/list)                                                                       | `json`/`text` (GAP)                                                                      | json / generic                   | shape preserved, `process_running` lost                                                                                                             |
| delegate_task                                                                                | `json` (GAP)                                                                             | json arm                         | `[json:keys L                                                                 \| results, total_duration_seconds]`                                  |

Observational evidence for the marker lines themselves: json
`<<<CCR:2607dcd770e3e7981a9b1c4bb0be22c4995700e6|json|1728>>>`, diff
`<<<CCR:b1b5c7c745a0dd927b5d77c7e26a4ce6ea4376ea|diff|987>>>`, build_error
`<<<CCR:fafc789b89d4659bae6cae77b53414e848fb6ca8|build_error|801>>>` (all in
`.hermes/examples/templates/`), ls `<<<CCR:940fbe9416d6fcf921db14c81580309ed8b98351|ls|5889>>>`
and source_code `<<<CCR:f8d6c87de81a74c79a9af2909022ffec0534f49c|source_code|2645>>>`
(`.hermes/examples/previews/model_family-code_first.md`).
