# ISSUE-11 PREVIEW BATTERY - Empirical preview-quality audit of aphrodite-hermes 1.4.5

**Date:** 2026-09-17 · **Binary:** `target/release/libaphrodite_hermes.dylib` (built 17:01:49, fresh vs. latest source 16:44:06) · **Version:** `1.4.5` (confirmed via `aphrodite_hermes_version`)
**Repo:** `…/PlayForm/Aphrodite` (branch `Development`)
**Scope:** catalog every MISLEADING / SHALLOW / TYPE-WRONG preview like issue #11 (`{'success':true,...}` → literal `"ok"`) across all compression types, and score the engine.

---

## 1. Method

The dylib was loaded via `ctypes` and driven two ways:

1. **Direct tool path** - `aphrodite_hermes_dispatch_tool("aphrodite_compress", {"content": <payload>, "type": <hint>})` as specified in the task. 75 payload×hint rows.
2. **Production hook path** - `aphrodite_hermes_call_hook("transform_tool_result" | "transform_terminal_output", ...)`, which is what the LLM _actually_ sees in Hermes. 20 rows. Payloads were padded past the production thresholds (tool `tool_threshold=512B`, `terminal_threshold=256B`, from `state.rs`) so they are compressible, matching prod behavior; below-threshold content returns `null` and passes through raw (no bug there - the LLM sees the full text).

Every marker's preview line (`<<<CCR:hash|type|size>>>\n[preview]`) was captured verbatim. Round-trip sanity: `aphrodite_retrieve(hash)` returned the **exact original content** for all sampled entries (`MATCH`) - retrieval is lossless; the defects found are exclusively in **type + preview** quality, not storage.

Probe scripts: `/tmp/aphrodite_probe.py` (battery) and `/tmp/aphrodite_probe2.py` (hook path + real tool shapes). Full contents in Appendix A.

**Verdict definitions:**

- **OK** - preview informative and truthful.
- **MISLEADING** - preview content wrong / hides the payload's real meaning (the #11 bug class).
- **SHALLOW** - preview exists but hides the payload's real shape (first line only, generic counts, lost totals).
- **TYPE-WRONG** - marker `type` ≠ what the content really is.

---

## 2. Score

### Table A - direct `aphrodite_compress` battery (75 rows)

| verdict        | count  | share   |
| -------------- | ------ | ------- |
| OK             | 26     | 35%     |
| **MISLEADING** | **11** | **15%** |
| **SHALLOW**    | **8**  | **11%** |
| **TYPE-WRONG** | **30** | **40%** |

### Table B - production hook path, what the LLM actually sees (20 rows)

| verdict        | count | share   |
| -------------- | ----- | ------- |
| OK             | 9     | 45%     |
| **MISLEADING** | **6** | **30%** |
| **SHALLOW**    | **5** | **25%** |
| TYPE-WRONG     | 0     | 0%      |

**Bottom line: on the production hook path, 11 of 20 realistic compressible outputs (55%) produce a preview that misleads the LLM or hides the payload's real shape. On the direct compress path (the documented `type=tool_result` pattern) the defect rate is 65%, dominated by the hint poisoning the classifier.**

---

## 3. Table A - full battery (payload | type-hint | returned type | preview verbatim | verdict)

| payload                                   | type-hint   | returned type | preview verbatim                                                                          | verdict    |
| ----------------------------------------- | ----------- | ------------- | ----------------------------------------------------------------------------------------- | ---------- |
| tiny_success                              | tool_result | text          | `[text:1L 2B \| ok]`                                                                      | MISLEADING |
| success_plus_id                           | tool_result | text          | `[text:1L 2B \| ok]`                                                                      | MISLEADING |
| success_plus_error                        | tool_result | text          | `[text:1L 9B \| disk full]`                                                               | OK         |
| success_str                               | tool_result | text          | `[text:1L 4B \| done]`                                                                    | OK         |
| error_only                                | tool_result | text          | `[text:1L 22B \| file not found: foo.rs]`                                                 | OK         |
| error_found                               | tool_result | text          | `[text:1L 26B \| Found 3 matches in 2 files]`                                             | OK         |
| error_nested_json                         | tool_result | tool_result   | `[tool_result:1L 31B \| {"error": "{\"nested\": true}"}]`                                 | TYPE-WRONG |
| term_short                                | tool_result | text          | `[text:1L 11B \| hello world]`                                                            | OK         |
| term_prose                                | tool_result | text          | `[text:2L 35B \| All 12 tests passed.]`                                                   | OK         |
| term_empty_err                            | tool_result | text          | `[text:1L 28B \| bash: foo: command not found]`                                           | OK         |
| term_empty_noerr                          | tool_result | tool_result   | `[tool_result:1L 45B \| {"output": "", "exit_code": 1, "error": null}]`                   | MISLEADING |
| term_build_error                          | tool_result | build_error   | `[build:1E 0W 3L \| error[E0432]: unresolved import `foo`]`                               | OK         |
| term_build_output                         | tool_result | build_output  | `[build:0E 0W 2L]`                                                                        | OK         |
| term_test_result                          | tool_result | build_output  | `[build:0E 0W 3L]`                                                                        | MISLEADING |
| term_warning                              | tool_result | build_output  | `[build:0E 2W 4L]`                                                                        | OK         |
| search_shape                              | tool_result | search        | `[search:2 hits in 2 files \| src/a.rs:10 …]`                                             | MISLEADING |
| search_zero                               | tool_result | search        | `[search:1L]`                                                                             | SHALLOW    |
| search_count_only                         | tool_result | search        | `[search:1L]`                                                                             | MISLEADING |
| read_file                                 | tool_result | text          | `[text:3L 33B \| fn main() {]`                                                            | SHALLOW    |
| read_file_empty                           | tool_result | text          | `[text:0L 0B]`                                                                            | OK         |
| read_file_json_content                    | tool_result | text          | `[text:1L 16B \| {"a": 1, "b": 2}]`                                                       | TYPE-WRONG |
| skill_view_shape                          | tool_result | text          | `[text:1L 37B \| Use when doing X. Do Y first, then Z.]`                                  | OK         |
| retrieve_shape                            | tool_result | text          | `[text:1L 18B \| the real body text]`                                                     | OK         |
| found_only                                | tool_result | tool_result   | `[tool_result:1L 15B \| {"found": true}]`                                                 | SHALLOW    |
| result_key                                | tool_result | text          | `[text:1L 27B \| successfully wrote 12 bytes]`                                            | OK         |
| message_key                               | tool_result | text          | `[text:1L 17B \| All checks passed]`                                                      | OK         |
| diff_shape                                | tool_result | diff          | `[diff:0F +1/-1 5L]`                                                                      | OK         |
| diff_empty                                | tool_result | text          | `[text:1L 2B \| ok]`                                                                      | MISLEADING |
| array_objects                             | tool_result | tool_result   | `[tool_result:1L 54B \| [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}]]`          | TYPE-WRONG |
| array_huge (5000 items)                   | tool_result | tool_result   | `[tool_result:1L 28890B \| [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 1]` | TYPE-WRONG |
| nested_obj                                | tool_result | tool_result   | `[tool_result:1L 99B \| {"data": {"user": {"name": "Alice", "email": "a@b.com"}, "me]`    | TYPE-WRONG |
| flat_json                                 | tool_result | tool_result   | `[tool_result:1L 54B \| {"user": "alice", "role": "admin", "status": "active"}]`          | TYPE-WRONG |
| num_output                                | tool_result | tool_result   | `[tool_result:1L 30B \| {"output": 42, "exit_code": 0}]`                                  | TYPE-WRONG |
| exit_code_only                            | tool_result | tool_result   | `[tool_result:1L 16B \| {"exit_code": 0}]`                                                | TYPE-WRONG |
| preview_key                               | tool_result | text          | `[text:1L 17B \| something to show]`                                                      | OK         |
| prose                                     | tool_result | tool_result   | `[tool_result:2L 106B \| The build failed because the linker could not find libz.]`       | TYPE-WRONG |
| term_plain                                | tool_result | tool_result   | `[tool_result:3L 42B \| running 3 tests]`                                                 | TYPE-WRONG |
| diff_raw                                  | tool_result | tool_result   | `[tool_result:6L 83B \| --- a/foo.rs]`                                                    | TYPE-WRONG |
| build_log_raw                             | tool_result | tool_result   | `[tool_result:3L 74B \| Compiling foo v0.1.0]`                                            | TYPE-WRONG |
| rust_code                                 | tool_result | tool_result   | `[tool_result:7L 124B \| use std::collections::HashMap;]`                                 | TYPE-WRONG |
| python_code                               | tool_result | tool_result   | `[tool_result:8L 108B \| import os]`                                                      | TYPE-WRONG |
| markdown_table                            | tool_result | tool_result   | `[tool_result:5L 102B \| \| Name \| Age \| City \|]`                                      | TYPE-WRONG |
| markdown_doc                              | tool_result | tool_result   | `[tool_result:7L 107B \| # Release Notes]`                                                | TYPE-WRONG |
| url                                       | tool_result | tool_result   | `[tool_result:1L 63B \| https://example.com/very/long/path?query=1&page=2&filter=act]`    | TYPE-WRONG |
| yaml                                      | tool_result | tool_result   | `[tool_result:6L 64B \| name: webapp]`                                                    | TYPE-WRONG |
| xml                                       | tool_result | tool_result   | `[tool_result:4L 69B \| <root>]`                                                          | TYPE-WRONG |
| csv                                       | tool_result | tool_result   | `[tool_result:4L 48B \| name,age,city]`                                                   | TYPE-WRONG |
| long_single_line (10 KB)                  | tool_result | tool_result   | `[tool_result:1L 10000B \| word word word word word word word word word word word word ]` | SHALLOW    |
| short_ok                                  | tool_result | tool_result   | `[tool_result:1L 2B \| ok]`                                                               | TYPE-WRONG |
| whitespace                                | tool_result | tool_result   | `[tool_result:3L 9B]`                                                                     | OK         |
| empty                                     | tool_result | ERR           | `content is required`                                                                     | OK         |
| ansi                                      | tool_result | tool_result   | `[tool_result:2L 57B \| [31mFAIL[0m: test_one failed]`                                    | TYPE-WRONG |
| repeated                                  | tool_result | tool_result   | `[tool_result:1L 1000B \| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa]`  | SHALLOW    |
| git_status_raw                            | tool_result | tool_result   | `[tool_result:4L 47B \| M src/a.rs]`                                                      | TYPE-WRONG |
| ls_raw                                    | tool_result | tool_result   | `[tool_result:3L 158B \| drwxr-xr-x  3 nikola staff  96 Sep 17 10:00 src]`                | TYPE-WRONG |
| base64                                    | tool_result | tool_result   | `[tool_result:1L 84B \| QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVphYmNkZWZnaGlqa2xtbm9wcXJz]`    | TYPE-WRONG |
| binaryish                                 | tool_result | tool_result   | `[tool_result:1L 18B \| [2m dim [0m]`                                                     | TYPE-WRONG |
| ccr_marker_content                        | tool_result | tool_result   | `[tool_result:1L 26B \| <<<CCR:deadbeef\|text\|10>>>]`                                    | OK         |
| html                                      | tool_result | tool_result   | `[tool_result:1L 76B \| <html><head><title>Docs</title></head><body><a href='/x'>x</]`    | TYPE-WRONG |
| json_pretty_raw                           | tool_result | tool_result   | `[tool_result:5L 63B \| {]`                                                               | MISLEADING |
| prose_nohint                              | (none)      | text          | `[text:2L 106B \| The build failed because the linker could not find libz.]`              | OK         |
| prose_hint_text                           | text        | text          | `[text:2L 106B \| The build failed because the linker could not find libz.]`              | OK         |
| prose_hint_terminal                       | terminal    | terminal      | `[terminal:2L Try installing zlib via homebrew and re-running.]`                          | TYPE-WRONG |
| rust_code_nohint                          | (none)      | text          | `[text:7L 124B \| use std::collections::HashMap;]`                                        | SHALLOW    |
| rust_code_hint_terminal                   | terminal    | terminal      | `[terminal:7L }]`                                                                         | MISLEADING |
| term_short_hint_terminal                  | terminal    | text          | `[text:1L 11B \| hello world]`                                                            | OK         |
| array_objects_nohint                      | (none)      | json_array    | `[json:2items 1L \| keys: id, name]`                                                      | OK         |
| nested_obj_nohint                         | (none)      | text          | `[text:1L 99B \| {"data": {"user": {"name": "Alice", "email": "a@b.com"}, "me]`           | SHALLOW    |
| flat_json_nohint                          | (none)      | text          | `[text:1L 54B \| {"user": "alice", "role": "admin", "status": "active"}]`                 | SHALLOW    |
| term_test_hint_text                       | text        | build_output  | `[build:0E 0W 3L]`                                                                        | MISLEADING |
| search_realshape (matches_text, 19 total) | tool_result | search        | `[search:1L]`                                                                             | MISLEADING |
| flat_many_keys (7 keys)                   | tool_result | tool_result   | `[tool_result:1L 97B \| {"user":"alice","role":"admin","status":"active","plan":"pro]`    | TYPE-WRONG |
| content_obj                               | tool_result | tool_result   | `[tool_result:1L 21B \| {"content": {"a": 1}}]`                                           | TYPE-WRONG |
| term_exit_code_line                       | tool_result | terminal      | `[terminal:1L exit code: 1]`                                                              | OK         |
| term_error_colon                          | tool_result | terminal      | `[terminal:1L Error: something went wrong]`                                               | OK         |

---

## 4. Table B - production hook path, exact marker previews the LLM sees

### 4.1 `transform_tool_result` (Hermes tool results, wrapped JSON)

| payload                                                                                              | returned type | preview verbatim                                                             | verdict    |
| ---------------------------------------------------------------------------------------------------- | ------------- | ---------------------------------------------------------------------------- | ---------- |
| search_files real shape (`total_count:19` + `matches_text`, path-grouped - the actual Hermes output) | search        | `[search:1L]`                                                                | MISLEADING |
| tiny_success (`{"success": true}`)                                                                   | text          | `[text:1L 2B \| ok]`                                                         | MISLEADING |
| success_plus_id (`{"success": true, "id": 42}`)                                                      | text          | `[text:1L 2B \| ok]`                                                         | MISLEADING |
| search_zero (`{"total_count": 0, ...}`)                                                              | search        | `[search:1L]`                                                                | SHALLOW    |
| search_shape_array (150 total, truncated, 1 match)                                                   | search        | `[search:1 hits in 1 files \| src/a.rs:10 …]`                                | MISLEADING |
| term_test_result (`test result: ok. 10 passed; 0 failed`)                                            | build_output  | `[build:0E 0W 3L]`                                                           | MISLEADING |
| read_file wrapper (rust code)                                                                        | text          | `[text:3L 33B \| fn main() {]`                                               | SHALLOW    |
| patch_diff                                                                                           | diff          | `[diff:0F +1/-1 5L]`                                                         | OK         |
| prose_raw (non-JSON tool result)                                                                     | text          | `[text:4L 527B \| The build failed because the linker could not find libz.]` | OK         |
| diff_empty (`{"diff": "", "success": true}`)                                                         | text          | `[text:1L 2B \| ok]`                                                         | MISLEADING |
| skill_view                                                                                           | text          | `[text:1L 37B \| Use when doing X. Do Y first, then Z.]`                     | OK         |

### 4.2 `transform_terminal_output` (raw terminal output, no wrapper)

| payload                                             | returned type | preview verbatim                                            | verdict |
| --------------------------------------------------- | ------------- | ----------------------------------------------------------- | ------- |
| term_test_result_raw (`test result: ok. 3 passed`)  | test          | `[test:3 pass 0 fail 0 ignored]`                            | OK      |
| term_cargo_raw (`Compiling … Finished dev profile`) | text          | `[text:4L 379B \| Compiling foo v0.1.0]`                    | SHALLOW |
| term_error_raw (`error[E0432]: …`)                  | build         | `[build:1E 0W 4L \| error[E0432]: unresolved import `foo`]` | OK      |
| term_ls_raw (`ls -l` listing)                       | ls            | `[ls:2 files 1 dirs \| .toml×1]`                            | OK      |
| term_git_raw (porcelain status)                     | git           | `[git:1M 1A 1?? \| src/a.rs src/b.rs src/c.rs]`             | OK      |
| term_prose_raw                                      | text          | `[text:4L 351B \| All 12 tests passed.]`                    | OK      |
| term_markdown_raw (table)                           | text          | `[text:6L 374B \| \| Name \| Age \|]`                       | SHALLOW |
| term_plain_ok                                       | text          | `[text:3L 319B \| ok]`                                      | OK      |
| term_rust_code_raw                                  | text          | `[text:9L 441B \| use std::collections::HashMap;]`          | SHALLOW |

**Key contrast:** the raw terminal path is _strong_ (test/ls/git/build_error all get rich, truthful previews) - but the **tool-result wrapper path regresses the same content**:

| Content             | Raw terminal path (correct)                 | Tool-result wrapper path (regressed, before 1.4.5)                                                             |
| ------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| test output         | `[test:3 pass 0 fail 0 ignored]`            | the identical test output inside `{"output":…,"exit_code":0}` becomes `[build:0E 0W 3L]` instead of `[test:…]` |
| real search results | rich `[search:N hits in M files …]` preview | collapse to `[search:1L]`                                                                                      |

---

## 5. Hint-handling behavior (observed)

`compress_into` (tools.rs ~193-199): `unwrap_hermes_result(content)` wins → **hint ignored**; otherwise `ccr_type = if hint.is_empty() || hint == "text" { detect_type } else { hint }` → **hint honored verbatim**.

| Payload class           | Hint                                       | Observed behavior                                                                                                                                                                                                                     |
| ----------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wrapper-shaped payloads | any (e.g. `terminal`, `text`)              | hint discarded (good) - e.g. `term_short` with hint `terminal` still returns `text`, `term_test_result` with hint `text` still returns `build_output`. Consequence: **a caller hint can never rescue the test→build_output misroute** |
| Non-wrapper payloads    | `""` / `text`                              | detected type (best quality: `array_objects_nohint → json_array [json:2items …]`)                                                                                                                                                     |
| Non-wrapper payloads    | `tool_result` / `terminal` / anything else | **literal marker type** - the documented `type=tool_result` call pattern therefore retypes every raw payload as `tool_result` and drops to the generic first-line fallback preview (24 of 25 raw rows in Table A)                     |
| `rust_code`             | `terminal`                                 | `[terminal:7L }]` - preview is the _last_ line of the code (`}`), actively misleading                                                                                                                                                 |

---

## 6. Top-5 worst offenders (user-visible impact: what an LLM would wrongly conclude)

| #   | Case                                                                                                                             | Before (1.4.5 broken preview)                                                                                                                                                                                                                                                                                                                                                                                       | What an LLM would wrongly conclude                                                                                                                                                                   |
| --- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Real `search_files` result (both paths)                                                                                          | `[search:1L]` - `unwrap_hermes_result` only reads the `matches` array; the real Hermes shape ships `matches_text` (path-grouped). The `"19 total"` label is built, then `build_search_preview` regex-counts it and finds zero `file:line:` hits → `[search:1L]`. The _entire_ search result is invisible behind a 1-line marker.                                                                                    | the search found nothing usable - skips real references, re-runs the search, or asserts a symbol/pattern "doesn't exist" in the codebase                                                             |
| 2   | Test-summary wrapper (both paths)                                                                                                | `test result:` routes to `build_output` in the unwrap heuristics; the preview counts only the literal substrings `error`/`warning`. **`test result: FAILED. 3 failed` also previews as `[build:0E 0W …]`** - a failing test run looks like a clean build. The raw terminal path already handles this correctly (`[test:3 pass 0 fail 0 ignored]`), so the wrapper path is a pure regression.                        | "build clean, no failures" - misses red tests entirely                                                                                                                                               |
| 3   | `{"success": true, "id": 42}` (both paths)                                                                                       | `[text:1L 2B \| ok]` - the `obj.len() <= 2` heuristic in the success branch fires on _any_ success-plus-one-key envelope, dropping real data. Strictly worse than issue #11's original `{"success":true}` case: it is data loss, not just terseness. (`{"diff": "", "success": true}` → `ok` is the same class: an LLM assumes a patch applied changes when it didn't.)                                             | told "create resource X and report its id" wrongly concludes success and answers without the id - or hallucinates one                                                                                |
| 4   | `type=tool_result` hint poisoning (compress path, 24/25 raw payloads)                                                            | any non-wrapper content passed with the documented hint gets marker type literally `tool_result` and a 60-char first-line preview: unified diff → `[tool_result:6L 83B \| --- a/foo.rs]`, ls → first line, git status → first line, html → no title/stats, **pretty JSON → `[tool_result:5L 63B \| {]`** (the preview is a lone brace).                                                                             | the payload is an unknown/trivial blob - the diff/code/html/json/ls/git/test typing and all enriched previews are dead on this path. Most actively misleading single preview in the battery: the `{` |
| 5   | JSON object envelopes → `text`/`tool_result` with raw first-60-chars preview (nested_obj, flat_json, flat_many_keys; both paths) | `detect_type` only returns `json_array` for arrays - objects fall to `text` (no-hint) or `tool_result` (hint), and the fallback preview truncates mid-value at 60 chars. A 7-key config object previews as `{"user":"alice","role":"admin","status":"active","plan":"pro` - the LLM sees one-and-a-half key/value pairs and a wrong type; it cannot see `org`, `age`, `city`, or even that this is structured data. | the payload is a one-and-a-half-pair blob, not structured data; `org`/`age`/`city` and the object's shape are invisible                                                                              |

---

## 7. Root-cause classes

- **RC-A - `unwrap_hermes_result` envelope misfires (tools.rs 68-177):** `success`+field collapse (`len <= 2`), `test result:` → `build_output` routing, search unwrap ignores `matches_text` and drops `total_count`/`truncated` once match lines exist, JSON-typed `error` strings fall through, non-string `output` falls through, empty-output + null-error failed commands fall through to a raw-envelope preview.
- **RC-B - hint override in `compress_into` (tools.rs 197):** any non-`""`/`text` hint becomes the literal marker type, silencing `detect_type` and every rich preview arm. Drives 30/75 Table-A TYPE-WRONG rows.
- **RC-C - `detect_type` (headroom) limitations:** JSON objects → `text` (only arrays → `json_array`); rust/python code → `text`; raw cargo lines → `text`. These classes always land in the generic fallback preview.
- **RC-D - generic fallback preview = first line only (preview.rs `_` arm):** tables, CSV, YAML, XML, JSON, ls, git all show just the first line/60 chars when typing fails - no shape signal, no counts.

**No storage/retrieval defects found:** hashes resolve losslessly, sizes are exact, markers are well-formed, and below-threshold content passes through raw.

---

## Appendix A - probe scripts

- `/tmp/aphrodite_probe.py` - Table A battery (75 rows): loads the dylib, dispatches `aphrodite_compress` via `aphrodite_hermes_dispatch_tool` with `{"content": …, "type": …}`, prints `payload | hint | type | size | preview | marker`, verifies round-trip via `aphrodite_retrieve`.
- `/tmp/aphrodite_probe2.py` - Table B (production hooks): calls `aphrodite_hermes_call_hook("transform_tool_result"/"transform_terminal_output")` with realistic Hermes shapes (real `search_files` `matches_text` output, test wrappers, raw terminal outputs), padding payloads past the production thresholds (tool 512 B, terminal 256 B) so they are compressible.

Key call logic (both scripts):

```python
import ctypes, json
lib = ctypes.CDLL("…/PlayForm/Aphrodite/target/release/libaphrodite_hermes.dylib")
lib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_hermes_dispatch_tool.restype = ctypes.c_void_p
lib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]

def compress(content, hint=""):
    args = {"content": content}
    if hint: args["type"] = hint
    ptr = lib.aphrodite_hermes_dispatch_tool(b"aphrodite_compress", json.dumps(args).encode())
    out = json.loads(ctypes.string_at(ptr).decode())
    lib.aphrodite_hermes_free_string(ptr)
    return out  # {hash, type, size, preview, marker}
```

Full battery definitions (payload names → contents) are embedded in the scripts; Table A maps 1:1 to `/tmp/aphrodite_probe.py`'s `B` list, Table B to `/tmp/aphrodite_probe2.py`.
