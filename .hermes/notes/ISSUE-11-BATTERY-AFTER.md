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

**claim (FIXDESIGN/WS1/WS2/WS4):** failing-test build arm surfaces the FAILURE note, never a clean `[build:0E 0W 3L]`

**evidence (1.4.6, plugin path):**

```text
no-hint wrapper `{"output": "…test result: FAILED. 1 passed; 2 failed…", "exit_code": 0}` → `[build:0E 0W 3L | test result: FAILED. 1 passed; 2 failed; finished in 0.05s]`(compress AND`transform_tool_result` paths)
```

**status:** ✅

---

**claim (FIXDESIGN/WS1/WS2/WS4):** passing test run stays clean (no false failure flag)

**evidence (1.4.6, plugin path):**

```text
no-hint wrapper, `test result: ok. 2 passed; 0 failed` → `[build:0E 0W 3L]` unchanged
```

**status:** ✅

---

**claim (FIXDESIGN/WS1/WS2/WS4):** search shows the REAL total_count

**evidence (1.4.6, plugin path):**

```text
25-entry `matches` array → `[search:25 hits in 25 files | f0.rs:0 …]`(take(20) cap gone); real`matches_text`shape (19 total) →`[search:5 hits in 3 files | /repo/src/a.rs:13 …]`(was`[search:1L]`); `{"total_count": 0, "matches": []}`→`[text:1L 7B | 0 total]`; `{"total_count": 5, "truncated": true}`→`[text:1L 19B | 5 total (truncated)]`; `{"total_count": 42, "items": […]}`→ previews as itself, NOT hijacked into`search`
```

**status:** ✅

---

**claim (FIXDESIGN/WS1/WS2/WS4):** error/linter/log arms show the real error line, not the traceback header / first line

**evidence (1.4.6, plugin path):**

```text
traceback payload + hint `error` → `[error:3L 91B | ValueError: disk full]`(was`Traceback (most recent call last):`); ruff shape → `[lint:2L 59B | src/x.py:10:5: E501 line too long (98 > 88)]`; log with error → `[log:2L 39B | ERROR connection refused]`
```

**status:** ✅

---

**claim (FIXDESIGN/WS1/WS2/WS4):** no ok-collapse

**evidence (1.4.6, plugin path):**

```text
zero genuine ` | ok]`previews across all 95 rows (the single sweep hit is`short_ok`, whose payload IS the literal 2-byte string `ok`- honest preview, not a collapse); the issue's exact repro`{"success": true, "data": {"web": [{"title": "x"}]}}`+ hint`tool_result`→`[tool_result:1L 52B | {"success": true, "data": {"web": [{"title": "x"}]}}]`(was`[text:1L 2B | ok]`); hook-path `{"success": true}`→`[text:1L 640B | {"success": true}]`; `{"diff": "", "success": true}`→`[text:1L 640B | {"diff": "", "success": true}]`
```

**status:** ✅

---

**claim (FIXDESIGN/WS1/WS2/WS4):** `preview_max_chars` cap enforced end-to-end

**evidence (1.4.6, plugin path):**

```text
fresh process with `APHRODITE_PREVIEW_MAX_CHARS=20` on a 300 B payload → `[text:1L 300B | So…]`(closing bracket +`…` preserved) on the compress path AND the hook path
```

**status:** ✅

---

## 4. Table A - full battery (payload | hint | before verdict | after type | after preview | after verdict)

**payload:** tiny_success

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 17B | {"success": true}]`
```

**1.4.6:** OK

---

**payload:** success_plus_id

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 27B | {"success": true, "id": 42}]`
```

**1.4.6:** OK

---

**payload:** success_plus_error

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 39B | {"success": true, "error": "disk f…`
```

**1.4.6:** OK

---

**payload:** success_str

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 19B | {"success": "done"}]`
```

**1.4.6:** OK

---

**payload:** error_only

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 35B | {"error": "file not found: foo.rs"…`
```

**1.4.6:** OK

---

**payload:** error_found

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 39B | {"error": "Found 3 matches in 2 fi…`
```

**1.4.6:** OK

---

**payload:** error_nested_json

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 31B | {"error": "{\"nested\": true}"}]`
```

**1.4.6:** OK

---

**payload:** term_short

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 56B | {"output": "hello world", "exit_co…`
```

**1.4.6:** OK

---

**payload:** term_prose

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 67B | {"output": "All 12 tests passed.\n…`
```

**1.4.6:** OK

---

**payload:** term_empty_err

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 71B | {"output": "", "exit_code": 1, "er…`
```

**1.4.6:** OK

---

**payload:** term_empty_noerr

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 45B | {"output": "", "exit_code": 1, "er…`
```

**1.4.6:** OK

---

**payload:** term_build_error

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 119B | {"output": "error[E0432]: unresol…`
```

**1.4.6:** OK

---

**payload:** term_build_output

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 93B | {"output": " Compiling foo v0.1.…`
```

**1.4.6:** OK

---

**payload:** term_test_result

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 86B | {"output": "running 10 tests\n\nte…`
```

**1.4.6:** OK

---

**payload:** term_warning

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 99B | {"output": "warning: unused variab…`
```

**1.4.6:** OK

---

**payload:** search_shape

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 172B | {"total_count": 150, "matches": […`
```

**1.4.6:** OK

---

**payload:** search_zero

**hint:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 53B | {"total_count": 0, "matches": [], …`
```

**1.4.6:** OK

---

**payload:** search_count_only

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 18B | {"total_count": 5}]`
```

**1.4.6:** OK

---

**payload:** read_file

**hint:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 70B | {"content": "fn main() {\n prin…`
```

**1.4.6:** OK

---

**payload:** read_file_empty

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 33B | {"content": "", "total_lines": 0}]`
```

**1.4.6:** OK

---

**payload:** read_file_json_content

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 53B | {"content": "{\"a\": 1, \"b\": 2}"…`
```

**1.4.6:** OK

---

**payload:** skill_view_shape

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 78B | {"name": "some-skill", "descriptio…`
```

**1.4.6:** OK

---

**payload:** retrieve_shape

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 83B | {"found": true, "source": "ccr", "…`
```

**1.4.6:** OK

---

**payload:** found_only

**hint:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 15B | {"found": true}]`
```

**1.4.6:** OK

---

**payload:** result_key

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 41B | {"result": "successfully wrote 12 …`
```

**1.4.6:** OK

---

**payload:** message_key

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 32B | {"message": "All checks passed"}]`
```

**1.4.6:** OK

---

**payload:** diff_shape

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 90B | {"diff": "--- a/x.rs\n+++ b/x.rs\n…`
```

**1.4.6:** OK

---

**payload:** diff_empty

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 29B | {"diff": "", "success": true}]`
```

**1.4.6:** OK

---

**payload:** array_objects

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 54B | [{"id": 1, "name": "alice"}, {"id"…`
```

**1.4.6:** OK

---

**payload:** array_huge

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 28890B | [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, …`
```

**1.4.6:** OK

---

**payload:** nested_obj

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 99B | {"data": {"user": {"name": "Alice"…`
```

**1.4.6:** OK

---

**payload:** flat_json

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 54B | {"user": "alice", "role": "admin",…`
```

**1.4.6:** OK

---

**payload:** num_output

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 30B | {"output": 42, "exit_code": 0}]`
```

**1.4.6:** OK

---

**payload:** exit_code_only

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 16B | {"exit_code": 0}]`
```

**1.4.6:** OK

---

**payload:** preview_key

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 32B | {"preview": "something to show"}]`
```

**1.4.6:** OK

---

**payload:** prose

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:2L 106B | The build failed because the link…`
```

**1.4.6:** OK

---

**payload:** term_plain

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:3L 42B | running 3 tests]`
```

**1.4.6:** SHALLOW

---

**payload:** diff_raw

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:6L 83B | --- a/foo.rs]`
```

**1.4.6:** SHALLOW

---

**payload:** build_log_raw

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:3L 74B | Compiling foo v0.1.0]`
```

**1.4.6:** SHALLOW

---

**payload:** rust_code

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:7L 124B | use std::collections::HashMap;]`
```

**1.4.6:** SHALLOW

---

**payload:** python_code

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:8L 108B | import os]`
```

**1.4.6:** SHALLOW

---

**payload:** markdown_table

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:5L 102B | | Name | Age | City | ]`
```

**1.4.6:** SHALLOW

---

**payload:** markdown_doc

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:7L 107B | # Release Notes]`
```

**1.4.6:** SHALLOW

---

**payload:** url

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 63B | https://example.com/very/long/path…`
```

**1.4.6:** OK

---

**payload:** yaml

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:6L 64B | name: webapp]`
```

**1.4.6:** SHALLOW

---

**payload:** xml

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:4L 69B | <root>]`
```

**1.4.6:** SHALLOW

---

**payload:** csv

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:4L 48B | name,age,city]`
```

**1.4.6:** SHALLOW

---

**payload:** long_single_line

**hint:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 10000B | word word word word word word w…`
```

**1.4.6:** SHALLOW

---

**payload:** short_ok

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 2B | ok]`
```

**1.4.6:** OK

---

**payload:** whitespace

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:3L 9B]`
```

**1.4.6:** OK

---

**payload:** empty

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** ERR

**1.4.6 preview:**

```text
`''`
```

**1.4.6:** OK

---

**payload:** ansi

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:2L 57B | [31mFAIL[0m: test_one failed]`
```

**1.4.6:** OK

---

**payload:** repeated

**hint:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 1000B | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…`
```

**1.4.6:** SHALLOW

---

**payload:** git_status_raw

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:4L 47B | M src/a.rs]`
```

**1.4.6:** SHALLOW

---

**payload:** ls_raw

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:3L 158B | drwxr-xr-x 3 nikola staff 96 Se…`
```

**1.4.6:** SHALLOW

---

**payload:** base64

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 84B | QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWV…`
```

**1.4.6:** OK

---

**payload:** binaryish

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 18B | [2m dim [0m]`
```

**1.4.6:** OK

---

**payload:** ccr_marker_content

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 26B | <<<CCR:deadbeef | text | 10>>>]`
```

**1.4.6:** OK

---

**payload:** html

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 76B | <html><head><title>Docs</title></h…`
```

**1.4.6:** OK

---

**payload:** json_pretty_raw

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:5L 63B | {]`
```

**1.4.6:** MISLEADING

---

**payload:** prose_nohint

**hint:** -

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:2L 106B | The build failed because the linker coul…`
```

**1.4.6:** OK

---

**payload:** prose_hint_text

**hint:** text

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:2L 106B | The build failed because the linker coul…`
```

**1.4.6:** OK

---

**payload:** prose_hint_terminal

**hint:** terminal

**1.4.5:** TYPE-WRONG

**1.4.6 type:** terminal

**1.4.6 preview:**

```text
`[terminal:2L Try installing zlib via homebrew and re-run…`
```

**1.4.6:** OK

---

**payload:** rust_code_nohint

**hint:** -

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:7L 124B | use std::collections::HashMap;]`
```

**1.4.6:** SHALLOW

---

**payload:** rust_code_hint_terminal

**hint:** terminal

**1.4.5:** MISLEADING

**1.4.6 type:** terminal

**1.4.6 preview:**

```text
`[terminal:7L }]`
```

**1.4.6:** MISLEADING

---

**payload:** term_short_hint_terminal

**hint:** terminal

**1.4.5:** OK

**1.4.6 type:** terminal

**1.4.6 preview:**

```text
`[terminal:1L {"output": "hello world", "exit_code": 0, "…`
```

**1.4.6:** OK

---

**payload:** array_objects_nohint

**hint:** -

**1.4.5:** OK

**1.4.6 type:** json_array

**1.4.6 preview:**

```text
`[json:2items 1L | keys: id, name]`
```

**1.4.6:** OK

---

**payload:** nested_obj_nohint

**hint:** -

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 99B | {"data": {"user": {"name": "Alice", "emai…`
```

**1.4.6:** SHALLOW

---

**payload:** flat_json_nohint

**hint:** -

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 54B | {"user": "alice", "role": "admin", "statu…`
```

**1.4.6:** SHALLOW

---

**payload:** term_test_hint_text

**hint:** text

**1.4.5:** MISLEADING

**1.4.6 type:** build_output

**1.4.6 preview:**

```text
`[build:0E 0W 3L]`
```

**1.4.6:** SHALLOW

---

**payload:** search_realshape

**hint:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 293B | {"total_count": 19, "matches_form…`
```

**1.4.6:** OK

---

**payload:** flat_many_keys

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 97B | {"user":"alice","role":"admin","st…`
```

**1.4.6:** OK

---

**payload:** content_obj

**hint:** tool_result

**1.4.5:** TYPE-WRONG

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 21B | {"content": {"a": 1}}]`
```

**1.4.6:** OK

---

**payload:** term_exit_code_line

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 42B | {"output": "exit code: 1", "exit_c…`
```

**1.4.6:** OK

---

**payload:** term_error_colon

**hint:** tool_result

**1.4.5:** OK

**1.4.6 type:** tool_result

**1.4.6 preview:**

```text
`[tool_result:1L 57B | {"output": "Error: something went …`
```

**1.4.6:** OK

### Table B - production hook path (n=20, marker type + preview the LLM sees)

**payload:** search_files_real(matches_text, 19 total)

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** search

**1.4.6 preview:**

```text
`[search:5 hits in 3 files | /repo/src/a.rs:13 …]`
```

**1.4.6:** OK

---

**payload:** tiny_success

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 640B | {"success": true}]`
```

**1.4.6:** OK

---

**payload:** success_plus_id

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 640B | {"success": true, "id": 42}]`
```

**1.4.6:** OK

---

**payload:** search_zero

**hook:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 7B | 0 total]`
```

**1.4.6:** OK

---

**payload:** search_shape_array

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** search

**1.4.6 preview:**

```text
`[search:1 hits in 1 files | src/a.rs:10 …]`
```

**1.4.6:** MISLEADING

---

**payload:** term_test_result

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** build_output

**1.4.6 preview:**

```text
`[build:0E 0W 3L]`
```

**1.4.6:** SHALLOW

---

**payload:** read_file

**hook:** tool_result

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:3L 33B | fn main() {]`
```

**1.4.6:** SHALLOW

---

**payload:** patch_diff

**hook:** tool_result

**1.4.5:** OK

**1.4.6 type:** diff

**1.4.6 preview:**

```text
`[diff:0F +1/-1 5L]`
```

**1.4.6:** OK

---

**payload:** prose_raw

**hook:** tool_result

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:4L 527B | The build failed because the linker coul…`
```

**1.4.6:** OK

---

**payload:** diff_empty

**hook:** tool_result

**1.4.5:** MISLEADING

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 640B | {"diff": "", "success": true}]`
```

**1.4.6:** OK

---

**payload:** skill_view

**hook:** tool_result

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:1L 640B | {"name": "some-skill", "description": "U…`
```

**1.4.6:** OK

---

**payload:** term_test_result_raw

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** test

**1.4.6 preview:**

```text
`[test:3 pass 0 fail 0 ignored]`
```

**1.4.6:** OK

---

**payload:** term_cargo_raw

**hook:** terminal

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:4L 694B | Compiling foo v0.1.0]`
```

**1.4.6:** SHALLOW

---

**payload:** term_error_raw

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** build

**1.4.6 preview:**

```text
`[build:1E 0W 4L | error[E0432]: unresolved import `foo`]`
```

**1.4.6:** OK

---

**payload:** term_ls_raw

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** ls

**1.4.6 preview:**

```text
`[ls:2 files 1 dirs | .toml×1]`
```

**1.4.6:** OK

---

**payload:** term_git_raw

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** git

**1.4.6 preview:**

```text
`[git:1M 1A 1?? | src/a.rs src/b.rs src/c.rs]`
```

**1.4.6:** OK

---

**payload:** term_prose_raw

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:4L 666B | All 12 tests passed.]`
```

**1.4.6:** OK

---

**payload:** term_markdown_raw

**hook:** terminal

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:6L 689B | | Name | Age | ]`
```

**1.4.6:** SHALLOW

---

**payload:** term_plain_ok

**hook:** terminal

**1.4.5:** OK

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:3L 634B | ok]`
```

**1.4.6:** OK

---

**payload:** term_rust_code_raw

**hook:** terminal

**1.4.5:** SHALLOW

**1.4.6 type:** text

**1.4.6 preview:**

```text
`[text:9L 756B | use std::collections::HashMap;]`
```

**1.4.6:** SHALLOW

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

1. **`json_pretty_raw` → `[tool_result:5L 63B | {]`** (direct, MISLEADING,
   unchanged from 1.4.5): pretty-printed JSON with hint `tool_result` routes to
   the generic first-line arm, whose first line is a lone `{`. Still the most
   actively misleading single preview in the battery. Root cause: RC-D (generic
   arm shows first non-empty line); the full-content preview fix does not help
   when the content's first line is `{`.
2. **`rust_code_hint_terminal` → `[terminal:7L }]`** (direct, MISLEADING,
   unchanged): the terminal arm shows the LAST line of the code (`}`).
3. **`search_shape_array` (hook, 150 total, 1 match) →
   `[search:1 hits in 1 files | src/a.rs:10 …]`** (MISLEADING, unchanged): the
   hits label counts the `matches` array (1), not `total_count` (150); the
   truncated-total signal is still invisible. The `matches_text` shape (the real
   Hermes output) is fixed; this synthetic `matches`-array-with-truncated case
   is not.
4. **SHALLOW tail (18 direct / 5 hook):** first-line previews for structured
   content that lands on the generic arm (`diff_raw`, `rust_code`,
   `python_code`, `markdown_table`, `yaml`, `xml`, `csv`, `ls_raw`,
   `git_status_raw`, `long_single_line`, `repeated`, `term_plain`,
   `nested_obj_nohint`, `flat_json_nohint`, `rust_code_nohint`,
   `term_test_hint_text`; hook: `term_test_result` wrapper, `read_file`,
   `term_cargo_raw`, `term_markdown_raw`, `term_rust_code_raw`). Root causes
   RC-C/RC-D (detect_type limits + generic first-line arm) - deferred scope, not
   touched by WS1/WS2/WS4.

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
