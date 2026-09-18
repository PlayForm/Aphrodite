# Issue #11 - JSON Preview-Collapse Bug Family

The complete record of PlayForm/Aphrodite-Hermes#11: a JSON tool result
preview collapsing to `[text:1L 2B | ok]`. Status: **WS1 + WS2 + WS4 + the
residual rewrite (defects 1/2/4) LANDED in the 1.4.6 line; WS3 (proxy
pipeline parity) + residual defect 3 PENDING (1.5.0).**

## 1. The bug

A JSON payload like `{"success": true, "data": {"web": [...]}}` compressed via
`aphrodite_compress` with `type: "tool_result"` produced:

```
<<<CCR:<hash>|text|58>>>
[text:1L 2B | ok]
```

The marker type flipped to `text`, the preview showed a 2-byte literal `"ok"`
and claimed 1L/2B while the stored payload was 58-737 bytes, and the caller's
explicit `type` hint was silently discarded. Storage/retrieval was always
lossless - only type + preview were corrupted.

## 2. Root cause

In `crates/aphrodite-hermes/src/tools.rs`, `unwrap_hermes_result`
(tools.rs:68-177) runs on **every** `aphrodite_compress` call before the
caller hint is consulted. Its success-bool branch
(`{"success": true}` with `obj.len() <= 2`) returned the hardcoded
`("ok", "text")` fragment; `compress_into` used that type verbatim and
previewed the **fragment** instead of the full payload (preview.rs generic
arm). The heuristic was written for the hook path (Hermes envelopes
`{output,exit_code}` / `{diff}` / `{error}` / `{total_count,matches}` /
`{content}`) and cannot distinguish an envelope from a user JSON payload.
`[previews] preview_max_chars` + `model_family` were declared-but-never-read
at the time - the reporter's config attempts could not help.

## 3. Timeline

| Date          | Event                                                                                       | Record                                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 2026-09-17    | Root cause + empirical verify on the 1.4.5 dylib: **NOT FIXED** above 1.3.7                 | `issue11/ISSUE-11-ROOT-CAUSE.md`, `ISSUE-11-VERIFY-1.4.5.md`                                               |
| 2026-09-17    | Full bug-class audit of `unwrap_hermes_result` (9 shapes) + preview-system deep audit       | `ISSUE-11-AUDIT.md`, `ISSUE-11-PREVIEW-SYSTEM-AUDIT.md`                                                    |
| 2026-09-17    | 95-row preview battery on 1.4.5: 65% defective direct / 55% hook                            | `ISSUE-11-PREVIEW-BATTERY.md`                                                                              |
| 2026-09-17    | Complete fix design space (candidates a-f); recommendation (a)+(b)                          | `ISSUE-11-FIXDESIGN.md`                                                                                    |
| 2026-09-17/18 | Pair B1/B2/B-FIX land WS1 + WS2 + WS4                                                       | `PAIR-B1.md`, `PAIR-B2.md`, `PAIR-B-FIX.md`                                                                |
| 2026-09-18    | C2 sweep green; battery-after re-run on the rebuilt dylib                                   | `ISSUE-11-BATTERY-AFTER.md`, `ISSUE-11-LANDED.md`                                                          |
| 2026-09-18    | Residual rewrite lands (commits `3b8b5d3` fix + `1d202b2` regex-drop + `7674456` detection) | see `issue11/PREVIEW-RACE-FIX-ROOTCAUSE.md` context; residual tail in `ISSUE-11-BATTERY-AFTER.md` section 6 |

## 4. What landed (WS1 + WS2 + WS4)

- **WS1** (`crates/aphrodite-hermes/src/tools.rs`): `ok` collapse deleted;
  success-string + priority-key arms guarded (single-key only); caller-hint-wins
  in `compress_into` - with a non-default hint, JSON payloads preview from the
  FULL content (honest L/B counts).
- **WS2**: honest `total_count` (`matches_text` normalized to `path:line:content`,
  `take(20)` cap removed, zero/count-only totals surface, no `total_count`-only
  hijack into fake `search`); honest build/error/linter/log arms in
  `crates/aphrodite/src/preview.rs` (line-based tallies - a FAILING test run now
  surfaces `test result: FAILED...`, never a clean `[build:0E 0W 3L]`).
- **WS4**: `preview_max_chars` wired end-to-end (env > TOML > default 120,
  single choke point at the end of `build_preview`, char-boundary-safe with
  closing `]` + `…` preserved, applied at startup / hot-reload / dylib init).

Test counts before -> after: `cargo test -p aphrodite` 348 -> 406 (current
verified: 377 lib + 29 bins, 1 ignored); `cargo test -p aphrodite-hermes`
46 -> 52; codegen self-tests 23 (unchanged); checker self-tests 12 -> 13.

## 5. Battery before -> after (effective rubric)

| Path                               | Before (1.4.5)                                                     | After (1.4.6)                                     |
| ---------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------- |
| Direct `aphrodite_compress` (n=75) | 49/75 defective (65%): 40% TYPE-WRONG, 15% MISLEADING, 11% SHALLOW | 20/75 (26.7%): MISLEADING 11->2, TYPE-WRONG 30->0 |
| Production hook path (n=20)        | 11/20 (55%): 30% MISLEADING, 25% SHALLOW                           | 6/20 (30%): MISLEADING 6->1                       |

The issue's exact repro -> `[tool_result:1L 52B | {...}]` (was
`[text:1L 2B | ok]`); real 25-hit search -> `[search:25 hits in 25 files | ...]`
(was `[search:20 hits ...]` / `[search:1L]`). Stability gates: SIGSEGV repro
SURVIVED (6 threads x 300), zero new SIGSEGV across all 95 rows + fix probes,
round-trips MATCH.

## 6. What remains (honest tail + deferrals)

Residual defects enumerated in `ISSUE-11-BATTERY-AFTER.md` section 6:

1. `json_pretty_raw` -> `[tool_result:5L 63B | {]` (first line is a lone `{`) -
   **fixed by the residual rewrite** (structured-shape arms, `3b8b5d3`).
2. `rust_code_hint_terminal` -> `[terminal:7L }]` (last line) - **fixed by the
   residual rewrite**.
3. `search_shape_array` (hook, 150 total / 1 match) -> hits label counts the
   `matches` array, not `total_count` - **PENDING** (1.5.0).
4. SHALLOW tail (first-line previews for structured content on the generic
   arm) - **addressed by the residual rewrite's structured-shape arms**; the
   generic-arm remainder stays a 1.5.0 item.

Deferrals (1.5.0): **WS3 proxy-pipeline parity** (`proxy.rs` parallel preview
arms -> route through `build_preview`); `model_family` / `code_structure_map`
/ `rust_preview_lines` remain inert knobs; rust-bindgen corpus stress tests
before new exports; real-codegen-path CI (ctypesgen byte-identity on CI).

## 7. Sweep (2026-09-18, all gates run)

checker PASS 0 violations; repro SURVIVED; `cargo test -p aphrodite` 406
passed (377 lib + 29 bins) / 0 failed / 1 ignored; `cargo test -p
aphrodite-hermes` 52 passed; codegen 23/23; checker self-tests 13/13 (50
asserts); drift-guard identical; ruff clean except the 2 known pre-existing
perf-probe errors; prettier clean.
