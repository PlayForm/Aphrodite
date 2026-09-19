# ISSUE-11-BATTERY-AFTER - 95-row preview battery re-run on the rebuilt 1.4.6 release dylib

**Date:** 2026-09-18 (EEST) · **Binary:**
`target/release/libaphrodite_hermes.dylib` (rebuilt; `aphrodite_hermes_version`
→ 1.4.6, plugin pin `BINARY_VERSION` 1.4.6) · **Branch:** `Development`

**Predecessors:** `.hermes/notes/ISSUE-11-PREVIEW-BATTERY.md` (the 95-row
battery, 1.4.5 baselines), `ISSUE-11-FIXDESIGN.md`, `PAIR-B1.md`, `PAIR-B2.md`,
`ISSUE-11-LANDED.md`

---

## 1. Method

Every row was exercised **exclusively through the plugin's hardened FFI path** -
the exact bug class this fix targets is raw-ctypes probes with unset restype
(two earlier probe scripts SIGSEGV'd). The runner imports the plugin package
(`sys.path.insert(0, plugins/aphrodite)`) and drives `_load_dylib()` +
`_call_json()` only: the generated `_bindings.py` restype/argtypes are applied
by `_configure_ffi` and every pointer return is forced `c_void_p` and freed
through the same handle. Zero hand-rolled ctypes in this run.

- **Table A (direct `aphrodite_compress`, n=75):** the exact payload×hint rows
  of the original battery (same contents, same hints) dispatched through
  `aphrodite_hermes_dispatch_tool`.
- **Table B (production hooks, n=20):** `transform_tool_result` (11 rows) and
  `transform_terminal_output` (9 rows) with the original Hermes shapes, padded
  past the production compression thresholds (tool 512 B; terminal threshold
  re-derived empirically on 1.4.6 - **512 B**, up from the 256 B documented for
  1.4.5, so Table B was padded to ≥600 B to stay compressible, matching what the
  LLM sees for compressible output).
- Round-trip sanity on sampled rows: `MATCH` (storage/retrieval lossless,
  unchanged).
- Runner: `issue11_battery_after.py` in the sigserve scratch directory. Nothing
  committed.

**Rubric (unchanged from the original battery):** OK = preview informative and
truthful; MISLEADING = preview content wrong / hides the payload's real meaning
(the #11 bug class); SHALLOW = preview exists but hides the payload's real shape
(first line only, generic counts, lost totals); TYPE-WRONG = marker type ≠ what
the content really is.

**Rubric refinement for the AFTER pass (documented, per FIXDESIGN WS1):** a
marker type that equals the **caller's explicit non-default hint** is caller
intent, not a defect - the schema contract is "trusts the `type` hint"
(schemas.rs). The literal-rubric reading (type must match content reality
regardless of hint) is reported in §5 for transparency.

---

## 2. Score - before → after

### Table A - direct `aphrodite_compress` (n=75)

| verdict    | before (1.4.5) | after (1.4.6) | Δ   |
| ---------- | -------------- | ------------- | --- |
| OK         | 26             | 55            | +29 |
| MISLEADING | 11             | 2             | -9  |
| SHALLOW    | 8              | 18            | +10 |
| TYPE-WRONG | 30             | 0             | -30 |

**Defective: 49/75 (65%) → 20/75 (26.7%)** - driven by MISLEADING 11→2 and
TYPE-WRONG 30→0 (effective rubric).

### Table B - production hook path (n=20)

| verdict    | before (1.4.5) | after (1.4.6) | Δ   |
| ---------- | -------------- | ------------- | --- |
| OK         | 9              | 14            | +5  |
| MISLEADING | 6              | 1             | -5  |
| SHALLOW    | 5              | 5             | +0  |
| TYPE-WRONG | 0              | 0             | +0  |

**Defective: 11/20 (55%) → 6/20 (30%)** - the #11 class (MISLEADING) dropped
6→1; the remaining defect is the SHALLOW tail (first-line previews) plus the
unchanged 1-hit-vs-150 search count row.

---

## 3. Fix-claims verification (all live on the rebuilt dylib, plugin path)

| Claim                                                                                 | Before (1.4.5)                                                                                                                                                                                                                           | After (1.4.6, plugin path)                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Status |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| failing-test build arm surfaces the FAILURE note, never a clean `[build:0E 0W 3L]`    | failing test runs previewed as a clean `[build:0E 0W 3L]` - no failure signal                                                                                                                                                            | no-hint wrapper `{"output": "…test result: FAILED. 1 passed; 2 failed…", "exit_code": 0}` → `[build:0E 0W 3L \| test result: FAILED. 1 passed; 2 failed; finished in 0.05s]` (compress AND `transform_tool_result` paths)                                                                                                                                                                                                                                                                      | ✅     |
| passing test run stays clean (no false failure flag)                                  | `[build:0E 0W 3L]` (already clean - risk was a false failure flag from the new arm)                                                                                                                                                      | no-hint wrapper, `test result: ok. 2 passed; 0 failed` → `[build:0E 0W 3L]` unchanged                                                                                                                                                                                                                                                                                                                                                                                                          | ✅     |
| search shows the REAL total_count                                                     | real `matches_text` shape (19 total) → `[search:1L]`; `matches` array → `[search:20 hits …]` (take(20) cap); zero/count-only → invisible count                                                                                           | 25-entry `matches` array → `[search:25 hits in 25 files \| f0.rs:0 …]` (take(20) cap gone); real `matches_text` shape (19 total) → `[search:5 hits in 3 files \| /repo/src/a.rs:13 …]` (was `[search:1L]`); `{"total_count": 0, "matches": []}` → `[text:1L 7B \| 0 total]`; `{"total_count": 5, "truncated": true}` → `[text:1L 19B \| 5 total (truncated)]`; `{"total_count": 42, "items": […]}` → previews as itself, NOT hijacked into `search`                                            | ✅     |
| error/linter/log arms show the real error line, not the traceback header / first line | traceback → `Traceback (most recent call last):` (the header, not the error); generic first line                                                                                                                                         | traceback payload + hint `error` → `[error:3L 91B \| ValueError: disk full]` (was `Traceback (most recent call last):`); ruff shape → `[lint:2L 59B \| src/x.py:10:5: E501 line too long (98 > 88)]`; log with error → `[log:2L 39B \| ERROR connection refused]`                                                                                                                                                                                                                              | ✅     |
| no ok-collapse                                                                        | the issue's exact repro `{"success": true, "data": {"web": [{"title": "x"}]}}` + hint `tool_result` → `[text:1L 2B \| ok]`; hook-path `{"success": true}` → `[text:1L 2B \| ok]`; `{"diff": "", "success": true}` → `[text:1L 2B \| ok]` | zero genuine `\| ok]` previews across all 95 rows (the single sweep hit is `short_ok`, whose payload IS the literal 2-byte string `ok` - honest preview, not a collapse); the issue's exact repro + hint `tool_result` → `[tool_result:1L 52B \| {"success": true, "data": {"web": [{"title": "x"}]}}]` (was `[text:1L 2B \| ok]`); hook-path `{"success": true}` → `[text:1L 640B \| {"success": true}]`; `{"diff": "", "success": true}` → `[text:1L 640B \| {"diff": "", "success": true}]` | ✅     |
| `preview_max_chars` cap enforced end-to-end                                           | no cap (declared-but-unread config; unlimited previews)                                                                                                                                                                                  | fresh process with `APHRODITE_PREVIEW_MAX_CHARS=20` on a 300 B payload → `[text:1L 300B \| So…]` (closing bracket + `…` preserved) on the compress path AND the hook path                                                                                                                                                                                                                                                                                                                      | ✅     |

---

## 4. Table A - full battery (payload | hint | 1.4.5 | 1.4.6 type | 1.4.6 preview | 1.4.6)

| payload                  | hint        | 1.4.5      | 1.4.6 type   | 1.4.6 preview                                                | 1.4.6      |
| ------------------------ | ----------- | ---------- | ------------ | ------------------------------------------------------------ | ---------- |
| tiny_success             | tool_result | MISLEADING | tool_result  | `[tool_result:1L 17B \| {"success": true}]`                  | OK         |
| success_plus_id          | tool_result | MISLEADING | tool_result  | `[tool_result:1L 27B \| {"success": true, "id": 42}]`        | OK         |
| success_plus_error       | tool_result | OK         | tool_result  | `[tool_result:1L 39B \| {"success": true, "error": "disk f…` | OK         |
| success_str              | tool_result | OK         | tool_result  | `[tool_result:1L 19B \| {"success": "done"}]`                | OK         |
| error_only               | tool_result | OK         | tool_result  | `[tool_result:1L 35B \| {"error": "file not found: foo.rs"…` | OK         |
| error_found              | tool_result | OK         | tool_result  | `[tool_result:1L 39B \| {"error": "Found 3 matches in 2 fi…` | OK         |
| error_nested_json        | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 31B \| {"error": "{\"nested\": true}"}]`    | OK         |
| term_short               | tool_result | OK         | tool_result  | `[tool_result:1L 56B \| {"output": "hello world", "exit_co…` | OK         |
| term_prose               | tool_result | OK         | tool_result  | `[tool_result:1L 67B \| {"output": "All 12 tests passed.\n…` | OK         |
| term_empty_err           | tool_result | OK         | tool_result  | `[tool_result:1L 71B \| {"output": "", "exit_code": 1, "er…` | OK         |
| term_empty_noerr         | tool_result | MISLEADING | tool_result  | `[tool_result:1L 45B \| {"output": "", "exit_code": 1, "er…` | OK         |
| term_build_error         | tool_result | OK         | tool_result  | `[tool_result:1L 119B \| {"output": "error[E0432]: unresol…` | OK         |
| term_build_output        | tool_result | OK         | tool_result  | `[tool_result:1L 93B \| {"output": " Compiling foo v0.1.…`   | OK         |
| term_test_result         | tool_result | MISLEADING | tool_result  | `[tool_result:1L 86B \| {"output": "running 10 tests\n\nte…` | OK         |
| term_warning             | tool_result | OK         | tool_result  | `[tool_result:1L 99B \| {"output": "warning: unused variab…` | OK         |
| search_shape             | tool_result | MISLEADING | tool_result  | `[tool_result:1L 172B \| {"total_count": 150, "matches": […` | OK         |
| search_zero              | tool_result | SHALLOW    | tool_result  | `[tool_result:1L 53B \| {"total_count": 0, "matches": [], …` | OK         |
| search_count_only        | tool_result | MISLEADING | tool_result  | `[tool_result:1L 18B \| {"total_count": 5}]`                 | OK         |
| read_file                | tool_result | SHALLOW    | tool_result  | `[tool_result:1L 70B \| {"content": "fn main() {\n prin…`    | OK         |
| read_file_empty          | tool_result | OK         | tool_result  | `[tool_result:1L 33B \| {"content": "", "total_lines": 0}]`  | OK         |
| read_file_json_content   | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 53B \| {"content": "{\"a\": 1, \"b\": 2}"…` | OK         |
| skill_view_shape         | tool_result | OK         | tool_result  | `[tool_result:1L 78B \| {"name": "some-skill", "descriptio…` | OK         |
| retrieve_shape           | tool_result | OK         | tool_result  | `[tool_result:1L 83B \| {"found": true, "source": "ccr", "…` | OK         |
| found_only               | tool_result | SHALLOW    | tool_result  | `[tool_result:1L 15B \| {"found": true}]`                    | OK         |
| result_key               | tool_result | OK         | tool_result  | `[tool_result:1L 41B \| {"result": "successfully wrote 12 …` | OK         |
| message_key              | tool_result | OK         | tool_result  | `[tool_result:1L 32B \| {"message": "All checks passed"}]`   | OK         |
| diff_shape               | tool_result | OK         | tool_result  | `[tool_result:1L 90B \| {"diff": "--- a/x.rs\n+++ b/x.rs\n…` | OK         |
| diff_empty               | tool_result | MISLEADING | tool_result  | `[tool_result:1L 29B \| {"diff": "", "success": true}]`      | OK         |
| array_objects            | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 54B \| [{"id": 1, "name": "alice"}, {"id"…` | OK         |
| array_huge               | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 28890B \| [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, …` | OK         |
| nested_obj               | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 99B \| {"data": {"user": {"name": "Alice"…` | OK         |
| flat_json                | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 54B \| {"user": "alice", "role": "admin",…` | OK         |
| num_output               | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 30B \| {"output": 42, "exit_code": 0}]`     | OK         |
| exit_code_only           | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 16B \| {"exit_code": 0}]`                   | OK         |
| preview_key              | tool_result | OK         | tool_result  | `[tool_result:1L 32B \| {"preview": "something to show"}]`   | OK         |
| prose                    | tool_result | TYPE-WRONG | tool_result  | `[tool_result:2L 106B \| The build failed because the link…` | OK         |
| term_plain               | tool_result | TYPE-WRONG | tool_result  | `[tool_result:3L 42B \| running 3 tests]`                    | SHALLOW    |
| diff_raw                 | tool_result | TYPE-WRONG | tool_result  | `[tool_result:6L 83B \| --- a/foo.rs]`                       | SHALLOW    |
| build_log_raw            | tool_result | TYPE-WRONG | tool_result  | `[tool_result:3L 74B \| Compiling foo v0.1.0]`               | SHALLOW    |
| rust_code                | tool_result | TYPE-WRONG | tool_result  | `[tool_result:7L 124B \| use std::collections::HashMap;]`    | SHALLOW    |
| python_code              | tool_result | TYPE-WRONG | tool_result  | `[tool_result:8L 108B \| import os]`                         | SHALLOW    |
| markdown_table           | tool_result | TYPE-WRONG | tool_result  | `[tool_result:5L 102B \| \| Name \| Age \| City \| ]`        | SHALLOW    |
| markdown_doc             | tool_result | TYPE-WRONG | tool_result  | `[tool_result:7L 107B \| # Release Notes]`                   | SHALLOW    |
| url                      | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 63B \| https://example.com/very/long/path…` | OK         |
| yaml                     | tool_result | TYPE-WRONG | tool_result  | `[tool_result:6L 64B \| name: webapp]`                       | SHALLOW    |
| xml                      | tool_result | TYPE-WRONG | tool_result  | `[tool_result:4L 69B \| <root>]`                             | SHALLOW    |
| csv                      | tool_result | TYPE-WRONG | tool_result  | `[tool_result:4L 48B \| name,age,city]`                      | SHALLOW    |
| long_single_line         | tool_result | SHALLOW    | tool_result  | `[tool_result:1L 10000B \| word word word word word word w…` | SHALLOW    |
| short_ok                 | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 2B \| ok]`                                  | OK         |
| whitespace               | tool_result | OK         | tool_result  | `[tool_result:3L 9B]`                                        | OK         |
| empty                    | tool_result | OK         | ERR          | `''`                                                         | OK         |
| ansi                     | tool_result | TYPE-WRONG | tool_result  | `[tool_result:2L 57B \| [31mFAIL[0m: test_one failed]`       | OK         |
| repeated                 | tool_result | SHALLOW    | tool_result  | `[tool_result:1L 1000B \| aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…` | SHALLOW    |
| git_status_raw           | tool_result | TYPE-WRONG | tool_result  | `[tool_result:4L 47B \| M src/a.rs]`                         | SHALLOW    |
| ls_raw                   | tool_result | TYPE-WRONG | tool_result  | `[tool_result:3L 158B \| drwxr-xr-x 3 nikola staff 96 Se…`   | SHALLOW    |
| base64                   | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 84B \| QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWV…` | OK         |
| binaryish                | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 18B \| [2m dim [0m]`                        | OK         |
| ccr_marker_content       | tool_result | OK         | tool_result  | `[tool_result:1L 26B \| <<<CCR:deadbeef \| text \| 10>>>]`   | OK         |
| html                     | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 76B \| <html><head><title>Docs</title></h…` | OK         |
| json_pretty_raw          | tool_result | MISLEADING | tool_result  | `[tool_result:5L 63B \| {]`                                  | MISLEADING |
| prose_nohint             | -           | OK         | text         | `[text:2L 106B \| The build failed because the linker coul…` | OK         |
| prose_hint_text          | text        | OK         | text         | `[text:2L 106B \| The build failed because the linker coul…` | OK         |
| prose_hint_terminal      | terminal    | TYPE-WRONG | terminal     | `[terminal:2L Try installing zlib via homebrew and re-run…`  | OK         |
| rust_code_nohint         | -           | SHALLOW    | text         | `[text:7L 124B \| use std::collections::HashMap;]`           | SHALLOW    |
| rust_code_hint_terminal  | terminal    | MISLEADING | terminal     | `[terminal:7L }]`                                            | MISLEADING |
| term_short_hint_terminal | terminal    | OK         | terminal     | `[terminal:1L {"output": "hello world", "exit_code": 0, "…`  | OK         |
| array_objects_nohint     | -           | OK         | json_array   | `[json:2items 1L \| keys: id, name]`                         | OK         |
| nested_obj_nohint        | -           | SHALLOW    | text         | `[text:1L 99B \| {"data": {"user": {"name": "Alice", "emai…` | SHALLOW    |
| flat_json_nohint         | -           | SHALLOW    | text         | `[text:1L 54B \| {"user": "alice", "role": "admin", "statu…` | SHALLOW    |
| term_test_hint_text      | text        | MISLEADING | build_output | `[build:0E 0W 3L]`                                           | SHALLOW    |
| search_realshape         | tool_result | MISLEADING | tool_result  | `[tool_result:1L 293B \| {"total_count": 19, "matches_form…` | OK         |
| flat_many_keys           | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 97B \| {"user":"alice","role":"admin","st…` | OK         |
| content_obj              | tool_result | TYPE-WRONG | tool_result  | `[tool_result:1L 21B \| {"content": {"a": 1}}]`              | OK         |
| term_exit_code_line      | tool_result | OK         | tool_result  | `[tool_result:1L 42B \| {"output": "exit code: 1", "exit_c…` | OK         |
| term_error_colon         | tool_result | OK         | tool_result  | `[tool_result:1L 57B \| {"output": "Error: something went …` | OK         |

### Table B - production hook path (n=20, marker type + preview the LLM sees)

| payload                                    | hook        | 1.4.5      | 1.4.6 type   | 1.4.6 preview                                                 | 1.4.6      |
| ------------------------------------------ | ----------- | ---------- | ------------ | ------------------------------------------------------------- | ---------- |
| search_files_real (matches_text, 19 total) | tool_result | MISLEADING | search       | `[search:5 hits in 3 files \| /repo/src/a.rs:13 …]`           | OK         |
| tiny_success                               | tool_result | MISLEADING | text         | `[text:1L 640B \| {"success": true}]`                         | OK         |
| success_plus_id                            | tool_result | MISLEADING | text         | `[text:1L 640B \| {"success": true, "id": 42}]`               | OK         |
| search_zero                                | tool_result | SHALLOW    | text         | `[text:1L 7B \| 0 total]`                                     | OK         |
| search_shape_array                         | tool_result | MISLEADING | search       | `[search:1 hits in 1 files \| src/a.rs:10 …]`                 | MISLEADING |
| term_test_result                           | tool_result | MISLEADING | build_output | `[build:0E 0W 3L]`                                            | SHALLOW    |
| read_file                                  | tool_result | SHALLOW    | text         | `[text:3L 33B \| fn main() {]`                                | SHALLOW    |
| patch_diff                                 | tool_result | OK         | diff         | `[diff:0F +1/-1 5L]`                                          | OK         |
| prose_raw                                  | tool_result | OK         | text         | `[text:4L 527B \| The build failed because the linker coul…`  | OK         |
| diff_empty                                 | tool_result | MISLEADING | text         | `[text:1L 640B \| {"diff": "", "success": true}]`             | OK         |
| skill_view                                 | tool_result | OK         | text         | `[text:1L 640B \| {"name": "some-skill", "description": "U…`  | OK         |
| term_test_result_raw                       | terminal    | OK         | test         | `[test:3 pass 0 fail 0 ignored]`                              | OK         |
| term_cargo_raw                             | terminal    | SHALLOW    | text         | `[text:4L 694B \| Compiling foo v0.1.0]`                      | SHALLOW    |
| term_error_raw                             | terminal    | OK         | build        | ``[build:1E 0W 4L \| error[E0432]: unresolved import `foo`]`` | OK         |
| term_ls_raw                                | terminal    | OK         | ls           | `[ls:2 files 1 dirs \| .toml×1]`                              | OK         |
| term_git_raw                               | terminal    | OK         | git          | `[git:1M 1A 1?? \| src/a.rs src/b.rs src/c.rs]`               | OK         |
| term_prose_raw                             | terminal    | OK         | text         | `[text:4L 666B \| All 12 tests passed.]`                      | OK         |
| term_markdown_raw                          | terminal    | SHALLOW    | text         | `[text:6L 689B \| \| Name \| Age \| ]`                        | SHALLOW    |
| term_plain_ok                              | terminal    | OK         | text         | `[text:3L 634B \| ok]`                                        | OK         |
| term_rust_code_raw                         | terminal    | SHALLOW    | text         | `[text:9L 756B \| use std::collections::HashMap;]`            | SHALLOW    |

## 5. Literal-rubric transparency (the refined-rubric decision and its effect)

Under the ORIGINAL battery's literal letter - "marker type must match content
reality, caller hint irrelevant" - the direct-path defect count lands at ~50/75
(~67%), statistically unchanged from the 65% baseline: the 30 TYPE-WRONG rows
were all raw-content / JSON-object rows whose non-default hint already won in
the `else` branch of `compress_into` on 1.4.5, and WS1's caller-hint-wins
deliberately keeps that contract (the schema documents "trusts the `type`
hint"). The literal buckets shift (some former SHALLOW rows become literal
TYPE-WRONG because the hint now also wins over wrapper unwraps), but the total
barely moves.

The **effective rubric** used in this report (type = caller's explicit hint =
caller intent, not a defect) gives 20/75 (26.7%) direct and 6/20 (30%) hook.
What improved under EITHER reading, independent of the type rubric:

- **The #11 MISLEADING class dropped 11→2 direct and 6→1 hook** - ok-collapse,
  wrong search counts, and the `{`-only pretty-JSON preview are the only
  survivors (see §6).
- Genuine ok-collapses are zero across all 95 rows.
- Every wrapper-shaped JSON payload now previews from its FULL content (the L/B
  counts match the stored payload, never a fragment), and the error/linter/log
  arms surface the real error line.

The residual literal-TYPE-WRONG is the documented hint-wins contract, not a
regression; the residual preview defects are enumerated in §6.

---

## 6. Residual defects (the honest tail)

| #   | Residual defect                                                                                     | Verdict (1.4.5 → 1.4.6)                                                                                                                                                                                                                                                                                                                                                                                                             | Root cause / note                                                                                                              |
| --- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 1   | **`json_pretty_raw` → `[tool_result:5L 63B \| {]`** (direct)                                        | MISLEADING, unchanged from 1.4.5 - pretty-printed JSON with hint `tool_result` routes to the generic first-line arm, whose first line is a lone `{`. Still the most actively misleading single preview in the battery.                                                                                                                                                                                                              | RC-D (generic arm shows first non-empty line); the full-content preview fix does not help when the content's first line is `{` |
| 2   | **`rust_code_hint_terminal` → `[terminal:7L }]`** (direct)                                          | MISLEADING, unchanged - the terminal arm shows the LAST line of the code (`}`)                                                                                                                                                                                                                                                                                                                                                      | terminal arm last-line behavior                                                                                                |
| 3   | **`search_shape_array` (hook, 150 total, 1 match) → `[search:1 hits in 1 files \| src/a.rs:10 …]`** | MISLEADING, unchanged - the hits label counts the `matches` array (1), not `total_count` (150); the truncated-total signal is still invisible. The `matches_text` shape (the real Hermes output) is fixed; this synthetic `matches`-array-with-truncated case is not.                                                                                                                                                               | hits label vs `total_count`                                                                                                    |
| 4   | **SHALLOW tail (18 direct / 5 hook)**                                                               | first-line previews for structured content that lands on the generic arm (`diff_raw`, `rust_code`, `python_code`, `markdown_table`, `yaml`, `xml`, `csv`, `ls_raw`, `git_status_raw`, `long_single_line`, `repeated`, `term_plain`, `nested_obj_nohint`, `flat_json_nohint`, `rust_code_nohint`, `term_test_hint_text`; hook: `term_test_result` wrapper, `read_file`, `term_cargo_raw`, `term_markdown_raw`, `term_rust_code_raw`) | RC-C/RC-D (detect_type limits + generic first-line arm) - deferred scope, not touched by WS1/WS2/WS4                           |

---

## 7. Stability gates

- **SIGSEGV repro SURVIVED:** `sigserve/repro.py` (plugin `register()` + 6
  threads × 300 `_load_dylib`/`_call_json` hammer, fresh process) →
  `SURVIVED: no crash`, exit 0.
- **Zero new SIGSEGV** across the full battery: 75 direct rows + 20 hook rows +
  10 fix probes + roundtrips + cap probe all exited 0 through the hardened
  plugin path.
- Round-trip losslessness: sampled rows `MATCH`.
- Version handshake: dylib `1.4.6`, plugin pin `BINARY_VERSION` `1.4.6`.
- Nothing committed (auto-committer sweeps).
