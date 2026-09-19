# PAIR-B1 - Issue #11 WS1 + WS2-tools: ok-collapse removal, success-string guards, caller-hint-wins, honest total_count

Date: 2026-09-17 (EEST) · Branch: Development · Binary 1.4.6 / plugin 2.1.4 ·
macOS Scope: B1 of the Issue #11 preview-fix pair. File owned:
`crates/aphrodite-hermes/src/tools.rs` ONLY - `preview.rs` / `config.rs` /
templates are B2's (untouched here). No commit (auto-committer sweeps).

## What changed (WS1)

All four changes are in `crates/aphrodite-hermes/src/tools.rs`; line numbers
below are pre-fix (shifted after edits).

| Change                                                 | Before (1.4.5)                                                                                                                                                                                                                                                                                                                                    | After (fixed)                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Deleted the `ok` collapse (tools.rs:123-126)           | the success-bool branch `if ok.as_bool() == Some(true) && obj.len() <= 2 { return Some(("ok", "text")) }` fired on ANY success-plus-one-key envelope and replaced the whole payload with a 2-byte fragment, producing the reported `[text:1L 2B \| ok]`                                                                                           | branch is removed; every genuine Hermes envelope is caught by the branches above it (`output`/`exit_code`, `diff`, `error`), so a success-only object is a user payload: it now falls through to `detect_type` / the caller hint                                                                                                                                                                                                                                |
| Guarded the success-string arm (tools.rs:127-131)      | `{"success": "ok", "data": [1]}` collapsed to the word                                                                                                                                                                                                                                                                                            | the string-success branch now requires a SINGLE-key object (`obj.len() <= 1`); `{"success": "wrote 3 files"}` still extracts; `{"success": "ok", "data": [1]}` no longer collapses to the word                                                                                                                                                                                                                                                                  |
| Guarded the priority-key arm (tools.rs:167-172)        | `description\|summary\|result\|message\|preview\|found` collapsed for ANY object size; `{"result": "ok", "data": {...}}` collapsed to `ok` (same bug class as the deleted success-bool collapse, previously ungated by any size); multi-key skill_view-shaped objects (`{"name": ..., "description": ...}`) previewed as the description fragment | the priority keys now only collapse for SINGLE-key objects; `{"result": "ok", "data": {...}}` (any size) no longer collapses to `ok`; a multi-key skill_view-shaped object now previews as its own JSON (honest; the description remains visible in the payload). The pinned table row encoding the old collapse was updated to `None` (test catching up with intent)                                                                                           |
| Caller-hint-wins in `compress_into` (tools.rs:193-198) | when `unwrap_hermes_result` returns `Some`, the unwrap verdict won and the caller's `type` hint was dropped; JSON previews counted the unwrap fragment (wrong `[text:1L 2B \| ok]`)                                                                                                                                                               | when `unwrap_hermes_result` returns `Some` AND the caller passed a non-default `type` hint (`!empty && != "text"`), the hint wins over the unwrap verdict; for JSON payloads (`{`-starting) the preview is built from the FULL content - an unwrap fragment would count the fragment, not the stored payload, and drops the shape signal. No-hint behavior is bit-identical (Hermes envelopes still unwrap; the pinned terminal-envelope test passes unchanged) |

## What changed (WS2-tools) - honest `total_count` (tools.rs:135-156)

| Change                                               | Before (1.4.5)                                                                                                                                                              | After (fixed)                                                                                                                                                                          |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`matches_text` support**                           | the real Hermes `search_files` shape ships `matches_text` (path-grouped `path\n  <line>: <content>`) instead of a `matches` array; a real search collapses to `[search:1L]` | normalized to the same grep-style `path:line:content` lines the search preview counts - a real search no longer collapses to `[search:1L]`                                             |
| **`take(20)` cap removed**                           | only 20 grep-style lines emitted; a 5,000-hit search previews as `[search:20 hits ...]`                                                                                     | one grep-style line per REAL match is emitted (the lines feed only the preview, never stored/hashed, so no size cost); a 5,000-hit search no longer previews as `[search:20 hits ...]` |
| **Zero / count-only results surface the REAL total** | `{"total_count": 0, "matches": []}` and `{"total_count": 5, "truncated": true}` preview as an unreadable `[search:1L]` whose count is invisible                             | return the label as type `text` so the generic arm renders `0 total` / `5 total (truncated)`                                                                                           |
| **`total_count`-only data objects are not hijacked** | `{"total_count": 42, "items": [...]}` becomes a fake `search` marker                                                                                                        | falls through (no `matches`/`matches_text`/`matches_format`/`truncated`) and previews as itself                                                                                        |

## Before -> after (preview strings, rebuilt `target/release/libaphrodite_hermes.dylib`)

Captured with `sigserve/pair_b1_probe.py` (ctypes, same call pattern as the
PREVIEW-BATTERY probes; every `void*` export gets explicit argtypes/restype -
the c_int-default restype truncation is the exact SIGSEGV class this repo
fixed). "Before" = 1.4.5 behavior, from
`.hermes/notes/ISSUE-11-PREVIEW-BATTERY.md` / `ISSUE-11-VERIFY-1.4.5.md`.

| payload (type hint)                                                    | before (1.4.5)                      | after (fixed)                                                                                       |
| ---------------------------------------------------------------------- | ----------------------------------- | --------------------------------------------------------------------------------------------------- |
| `{"success": true, "data": {"web": [{"title": "x"}]}}` (`tool_result`) | type `text` · `[text:1L 2B \| ok]`  | type `tool_result` · `[tool_result:1L 52B \| {"success": true, "data": {"web": [{"title": "x"}]}}]` |
| `{"success": true}` (none)                                             | `[text:1L 2B \| ok]`                | `[text:1L 17B \| {"success": true}]`                                                                |
| `{"success": "ok", "data": [1]}` (`tool_result`)                       | `[text:1L 2B \| ok]`                | `[tool_result:1L 30B \| {"success": "ok", "data": [1]}]`                                            |
| `{"result": "ok", "data": {"web": [{"title": "x"}]}}` (`tool_result`)  | `[text:1L 2B \| ok]`                | `[tool_result:1L 51B \| {"result": "ok", "data": {"web": [{"title": "x"}]}}]`                       |
| real search shape `matches_text`, 19 total (none)                      | `[search:1L]`                       | `[search:2 hits in 2 files \| src/a.rs:10 …]`                                                       |
| search `matches` array, 25 entries (none)                              | `[search:20 hits …]` (take(20) cap) | `[search:25 hits in 25 files \| f0.rs:0 …]`                                                         |
| `{"total_count": 0, "matches": []}` (none)                             | `[search:1L]`                       | `[text:1L 7B \| 0 total]`                                                                           |
| `{"total_count": 5, "truncated": true}` (none)                         | `[search:1L]`                       | `[text:1L 19B \| 5 total (truncated)]`                                                              |
| `{"total_count": 42, "items": [1, 2, 3]}` (none)                       | `[search:1L]`                       | `[text:1L 39B \| {"total_count": 42, "items": [1, 2, 3]}]`                                          |
| terminal envelope no hint                                              | `[terminal:2L exit code: 1]`        | unchanged `[terminal:2L exit code: 1]`                                                              |
| terminal envelope + hint `tool_result`                                 | hint dropped (type terminal/text)   | `[tool_result:1L 58B \| {"output": "error: broke\nexit code: 1\n", "exit_code": 1}]` (hint wins)    |

| Hook path (`transform_tool_result`, what the LLM sees)  | Before (1.4.5)                   | After (fixed)                                            |
| ------------------------------------------------------- | -------------------------------- | -------------------------------------------------------- |
| 1,212-byte real search envelope compressed via the hook | `[search:1L]`                    | `[search:25 hits in 25 files \| f0.rs:0 …]` - real count |
| Sub-threshold payloads                                  | pass through raw (`null` marker) | unchanged - pass through raw (`null` marker) as before   |

## Test results

- `cargo test -p aphrodite-hermes`: **52 passed, 0 failed** (46 existing + 6
  new; finished 0.10s).
- `python3 sigserve/repro.py`: **SURVIVED, exit=0** (6 threads x 300 hammer,
  fresh dylib).

### New regression tests (in `tools.rs`)

- `test_compress_json_payload_with_hint_keeps_type_and_full_preview` - the
  issue's exact repro shape with `type: "tool_result"`: hinted type kept, full
  payload previewed, size counts the payload, round-trip lossless (FIXDESIGN
  3.1).
- `test_compress_bare_success_object_preview_is_honest` - no-hint
  `{"success": true}` never renders `| ok]`; payload visible (FIXDESIGN 3.2,
  adapted - see deviation below).
- `test_compress_search_result_shows_real_match_count` - real Hermes
  `matches_text` shape: type `search`, preview counts the real hits
  (`[search:2 hits in 2 files …]`), no `[search:1L]`.
- `test_compress_search_matches_array_counts_all_matches` - 25-entry `matches`
  array previews `25 hits`, not the old 20-cap.
- `test_compress_search_zero_or_count_only_surfaces_total` -
  `{"total_count": 0, "matches": []}` shows `0 total`, never `[search:1L]`.
- `test_compress_data_object_with_total_count_key_is_not_search` -
  `total_count`-only data objects keep their own type/preview.

### Pinned table (`test_unwrap_hermes_result_table`) updates - pins that encoded the buggy collapse now encode the fix

| pinned table case                 | before                           | after                                                                                                                                                          |
| --------------------------------- | -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `{"success": true}`               | `Some(("ok", "text"))`           | `None`; added `{"success": true, "data": ...}` -> `None`, `{"success": "ok", "data": [1]}` -> `None`                                                           |
| `search without matches`          | type `search`                    | type `text` (`5 total (truncated)` label now renders visibly); added `matches_text` row -> `search` lines, and `{"total_count": 42, "items": [...]}` -> `None` |
| `priority key fallback` 2-key row | `Some(("does a thing", "text"))` | `None`; added single-key `{"description": ...}` -> extract (surviving guarded behavior) and `{"result": "ok", "data": ...}` -> `None`                          |

## Notes / deviations from FIXDESIGN

- **FIXDESIGN 3.2 asserts `type != "text"`** for bare `{"success": true}` - not
  achievable: headroom classifies JSON _objects_ as `text` (only arrays get
  `json_array`, `content_detector.rs` `try_detect_json` requires a `[` prefix).
  The adapted test asserts preview honesty instead (no `| ok]`, payload
  visible) - the meaningful property.
- **Priority keys guarded with `len <= 1`** per task scope (B1: "guard
  success-string arms (127-131 / 167-172)"); FIXDESIGN matrix row D3 flags this
  exact pin break (`priority key fallback` -> `None`). The multi-key
  skill_view-shaped object trade-off is documented above; genuine Hermes
  envelopes (`output`/`exit_code`, `diff`, `error`, `total_count`/`matches`,
  `content`) are all caught by branches that fire before the guards and are
  unaffected.
- Storage/retrieval untouched: hash/store still use the ORIGINAL content on
  every path; only type/preview choice changed. `aphrodite_reclassify` was
  already the honest-render reference and remains consistent.
- The 60-char preview hint cap (preview.rs generic arm) is unchanged - B2's
  territory.

## Files

- Modified: `crates/aphrodite-hermes/src/tools.rs` (only file; ~+180 / ~-30
  lines: 4 code changes + 6 new tests + 9 updated/added table rows).
- Probe: `sigserve/pair_b1_probe.py`.
- This report: `sigserve/PAIR-B1.md` + `.hermes/notes/PAIR-B1.md`.
- Nothing committed.
