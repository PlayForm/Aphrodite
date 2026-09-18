# issue11/ - Issue #11 Preview-Collapse Bug Family

The full record of PlayForm/Aphrodite-Hermes#11: `[text:1L 2B | ok]` preview
collapse for JSON tool results - root cause, fix design, before/after
batteries, what landed in 1.4.6, and the honest residual tail. Root entry
point: `../ISSUE-11.md`.

| File                                                                 | One-line description                                                                                                         |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| [ISSUE-11-ROOT-CAUSE.md](ISSUE-11-ROOT-CAUSE.md)                     | Exact code path emitting `ok` + `2B` + type `text`, the mechanism, and the minimal fix.                                      |
| [ISSUE-11-VERIFY-1.4.5.md](ISSUE-11-VERIFY-1.4.5.md)                 | Empirical repro on the 1.4.5 dylib: NOT FIXED, all input variants tabled.                                                    |
| [ISSUE-11-FIXDESIGN.md](ISSUE-11-FIXDESIGN.md)                       | Complete fix design space (candidates a-f), the recommended (a)+(b), regression tests, preview-format coupling audit.        |
| [ISSUE-11-AUDIT.md](ISSUE-11-AUDIT.md)                               | Deep audit of `unwrap_hermes_result`: 9-shape branch table, git history, hook-path reality check, dead config findings.      |
| [ISSUE-11-PREVIEW-SYSTEM-AUDIT.md](ISSUE-11-PREVIEW-SYSTEM-AUDIT.md) | Whole preview-generation pipeline audit: builders, call sites, 10 defect classes, ideal-preview spec, priority order.        |
| [ISSUE-11-PREVIEW-BATTERY.md](ISSUE-11-PREVIEW-BATTERY.md)           | 95-row empirical preview-quality battery on 1.4.5 - the BEFORE baselines (65% direct / 55% hook defective).                  |
| [ISSUE-11-BATTERY-AFTER.md](ISSUE-11-BATTERY-AFTER.md)               | 95-row battery re-run on the rebuilt 1.4.6 dylib - AFTER scores, fix-claims verification, residual defects, stability gates. |
| [ISSUE-11-LANDED.md](ISSUE-11-LANDED.md)                             | What shipped in 1.4.6 (WS1 + WS2 + WS4), test counts before/after, sweep table, deferrals.                                   |
| [PAIR-B1.md](PAIR-B1.md)                                             | Pair B1: WS1 + WS2-tools implementation in `tools.rs` (ok-collapse removal, guards, caller-hint-wins, honest total_count).   |
| [PAIR-B2.md](PAIR-B2.md)                                             | Pair B2: WS2-preview (honest build/error/linter/log arms) + WS4 (`preview_max_chars` end-to-end) in `preview.rs` + config.   |
| [PAIR-B-FIX.md](PAIR-B-FIX.md)                                       | Post-Pair-B catch: preview test-race fix (`cap_guard()` on 20 tests, process-global cap).                                    |
| [PREVIEW-RACE-FIX.md](PREVIEW-RACE-FIX.md)                           | The PREVIEW_MAX_CHARS test-suite flake: symptom, analysis, guard additions per test.                                         |
| [PREVIEW-RACE-FIX-ROOTCAUSE.md](PREVIEW-RACE-FIX-ROOTCAUSE.md)       | The actual root cause: leaked `APHRODITE_PREVIEW_MAX_CHARS` env var in the session shell.                                    |

Residual defects (post-fix honest tail) are enumerated in
[ISSUE-11-BATTERY-AFTER.md](ISSUE-11-BATTERY-AFTER.md) section 6 and rolled up
in `../ISSUE-11.md`.
