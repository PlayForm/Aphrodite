# ISSUE-11-LANDED - Issue #11 preview-collapse bug family: what shipped in 1.4.6

Date: 2026-09-18 (EEST) · Branch: Development · Binary 1.4.6 / plugin 2.1.4 · macOS
Status: WS1 + WS2 + WS4 LANDED and verified end-to-end (Pairs B1/B2/B-FIX + C2 sweep).
Predecessors: `.hermes/notes/ISSUE-11-ROOT-CAUSE.md`, `ISSUE-11-FIXDESIGN.md`,
`ISSUE-11-PREVIEW-BATTERY.md`, `PAIR-B1.md`, `PAIR-B2.md`, `PAIR-B-FIX.md`,
`SESSION-DISPATCH.md` (Pair C2).

---

## 1. What landed for Issue #11

### WS1 - ok-collapse removal + success guards + caller-hint-wins (`crates/aphrodite-hermes/src/tools.rs`, B1)

| Behavior                                     | Before (1.4.5)                                                             | After (1.4.6)                                                                                                                                                                                                 |
| -------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Deleted the `ok` collapse                    | success-bool objects collapse to `[text:1L 2B \| ok]`                      | success-bool objects no longer collapse; genuine Hermes envelopes are caught by the branches above them, everything else previews as itself                                                                   |
| Success-string arm guarded (single-key only) | `{"success": "ok", "data": [...]}` collapses to the word                   | no longer collapses to the word                                                                                                                                                                               |
| Priority-key arm guarded (single-key only)   | `{"result": "ok", "data": {...}}` collapses to `ok`                        | no longer collapses to `ok`; multi-key objects preview as their own JSON                                                                                                                                      |
| Caller-hint-wins                             | when `unwrap_hermes_result` returns `Some`, the caller's hint is discarded | when `unwrap_hermes_result` returns `Some` AND the caller passed a non-default type hint, the hint wins; JSON payloads preview from the FULL content (size counts the stored payload, not an unwrap fragment) |

### WS2 - honest build/error/linter/log arms + honest total_count

`tools.rs` (B1):

| Behavior                                        | Before (1.4.5)                                                                                                                                                                           | After (1.4.6)                                                                                                                                                                                                                                                                               |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Search `matches_text` + totals (`tools.rs`, B1) | real search shape collapses to `[search:1L]`; `take(20)` cap hides the true count; zero/count-only totals invisible; `total_count`-only data objects hijacked into fake `search` markers | `matches_text` (the real Hermes search shape) normalized to grep-style `path:line:content` lines; `take(20)` cap removed; zero / count-only results surface the real total (`0 total` / `5 total (truncated)`); `total_count`-only data objects are not hijacked into fake `search` markers |
| Build arm (`preview.rs`, B2)                    | substring counting (`error`/`warning` occurrences, misses capitalized `Error:`); failing test runs look clean (`[build:0E 0W 3L]`)                                                       | line-based error/warning tallies (rustc/clang/gcc/Python markers, capitalized `Error:` included) instead of substring counting; failing test runs surface the failure summary instead of a clean-looking `[build:0E 0W 3L]`; first-error enrichment matches the tally markers               |
| Error/linter/log arms (`preview.rs`, B2)        | generic first non-empty line (traceback header, `Compiling` noise)                                                                                                                       | first real error/failure line, first lint issue line, or tail                                                                                                                                                                                                                               |

### WS4 - `preview_max_chars` wired end-to-end (config plumbing, B2)

| Behavior                        | Before (1.4.5)                                         | After (1.4.6)                                                                                                                                                                          |
| ------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `preview_max_chars` enforcement | declared-but-unread config field (never read anywhere) | enforced at the single choke point (end of `build_preview`): every path (proxy, hooks, prefetch, poll_worker, engine C-ABI, Hermes dylib) renders bounded previews                     |
| Resolution chain                | no resolution chain (key never read)                   | env `APHRODITE_PREVIEW_MAX_CHARS` > TOML > default (120); absent key = unlimited (legacy behavior)                                                                                     |
| Truncation safety               | n/a (no cap applied)                                   | char-boundary-safe truncation preserves the closing `]` + `…` marker so `render_marker` / `parse_preview` / `chain_split` keep working (no `[text:[text:...]]` double-wrap regression) |
| Application points              | n/a (never applied anywhere)                           | applied at startup (`main.rs`), on hot-reload (`proxy.rs` `/reload`), and in dylib init (`crates/aphrodite-hermes/src/lib.rs`)                                                         |

## 2. Test counts before -> after

| suite                                                       | before | after | source                                   |
| ----------------------------------------------------------- | ------ | ----- | ---------------------------------------- |
| `cargo test -p aphrodite`                                   | 348    | 389   | B2 (+12 new; +9 net prior-session drift) |
| `cargo test -p aphrodite-hermes`                            | 46     | 52    | B1 (+6 tools.rs regression tests)        |
| `crates/aphrodite-hermes/codegen/test_finalize_bindings.py` | 23     | 23    | unchanged (A1 kept byte-stable)          |
| `Maintain/tests/test_check_ffi_contract.py`                 | 12     | 13    | A2 (+1 argtypes-count test)              |

New regression coverage: 6 tools.rs tests (hint-keeps-type-and-full-preview, bare-success-object honesty, real-match-count search, matches-array counting, zero/count-only totals, total_count-only-not-search) + 12 aphrodite tests (11 preview.rs incl. cap machinery + 1 config_loader `apply_previews`). PAIR-B-FIX hardened 20 preview tests with `cap_guard()` (test race from the new process-global cap).

## 3. Battery result summary

| path                        | before (1.4.5)                                              | after (1.4.6)                                                                                                                                                                                                                      | source                                                        |
| --------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| hook path (n=20)            | 55% defective (30% MISLEADING / 25% SHALLOW)                | **PENDING** - `ISSUE-11-BATTERY-AFTER.md` was not present at sweep time (no PAIR-C1 report found in `.hermes/notes/` or the sigserve scratch). Re-run the 95-row battery against the rebuilt dylib to record the defect-rate drop. | `.hermes/notes/ISSUE-11-PREVIEW-BATTERY.md` (1.4.5 baselines) |
| direct compress path (n=75) | 65% defective (40% TYPE-WRONG, 15% MISLEADING, 11% SHALLOW) | **PENDING** (same note as above)                                                                                                                                                                                                   | `.hermes/notes/ISSUE-11-PREVIEW-BATTERY.md` (1.4.5 baselines) |

Spot evidence the fix works (PAIR-B1/B2 probes on the rebuilt dylib):

| case                                                                | before (1.4.5)                       | after (rebuilt 1.4.6 dylib)                                        |
| ------------------------------------------------------------------- | ------------------------------------ | ------------------------------------------------------------------ |
| the issue's exact repro `{"success": true, "data": {"web": [...]}}` | `[text:1L 2B \| ok]`                 | `[tool_result:1L 52B \| {...}]`                                    |
| failing test run                                                    | `[build:0E 0W 3L]`                   | `[build:0E 0W 3L \| test result: FAILED. 1 passed; 2 failed; ...]` |
| real 25-hit search                                                  | `[search:20 hits …]` / `[search:1L]` | `[search:25 hits in 25 files \| f0.rs:0 …]`                        |

## 4. Remaining deferrals

- WS3 proxy-pipeline parity (`proxy.rs` parallel preview arms) -> 1.5.0.
- `model_family`, `code_structure_map`, `rust_preview_lines` remain declared-unread knobs (out of scope; only `preview_max_chars` was wired per dispatch).
- rust-bindgen corpus stress tests before adding new exports (1.5.0).

## 5. CI status after Pair A (A1/A2)

| CI item                                                           | before                                                                                                                       | after / status                                                                                                                                                                                                                                   |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| ruff on generated `_bindings.py`                                  | ~30+ errors                                                                                                                  | **FIXED** (FFI-attributable): `ruff.toml` per-file-ignores for the generated artifact + `# noqa: N812` on the alias import + top-level `extend-exclude` for `crates/aphrodite/templates`; verified clean on ruff 0.15.17 AND 0.16.8 (CI version) |
| stale checker self-test (`test_cli_exit_codes_and_missing_files`) | 11/12                                                                                                                        | **FIXED**: rewritten 13/13 with both ground-truth halves pinned + new `[wrong-argcount]` / `[unknown-export-argtypes]` violation classes (argtypes-count gap closed)                                                                             |
| ffi-check.yml trigger gap                                         | `build.rs`, `cbindgen.toml`, `codegen/**`, `crates/aphrodite-hermes/**`, `plugins/aphrodite/**` missing from push + PR paths | **FIXED**: added to push + PR paths                                                                                                                                                                                                              |
| `kompress_parity`                                                 | E0432 (`ml`-feature gate in vendor/headroom) blocks the dylib build + pytest FFI steps in CI                                 | **STILL FAILING** (pre-existing, NOT FFI); clippy doc-lazy-continuation at `crates/aphrodite-hermes/src/lib.rs:539`                                                                                                                              |
| CI codegen path                                                   | only the committed-artifact fallback (no ctypesgen on ubuntu)                                                                | real codegen-path byte-identity check is a 1.5.0 CI follow-up                                                                                                                                                                                    |

## 6. Verification sweep (C2, 2026-09-18 - all gates run, actual numbers)

| gate                                                                                       | result                                                                                                                  |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `python3 Maintain/check_ffi_contract.py`                                                   | PASS, 0 violation(s), 0 warning(s), exit 0                                                                              |
| `python3 sigserve/repro.py`                                                                | SURVIVED, no crash, exit 0 (6 threads x 300)                                                                            |
| `cargo test -p aphrodite`                                                                  | 389 passed, 0 failed, 1 ignored (360 lib + 16+5+2+4+2+0 bins)                                                           |
| `cargo test -p aphrodite-hermes`                                                           | 52 passed, 0 failed                                                                                                     |
| `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`                        | 23/23 OK                                                                                                                |
| `python3 Maintain/tests/test_check_ffi_contract.py`                                        | 13/13 cases passed, 50 asserts, exit 0                                                                                  |
| drift-guard `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` | identical (exit 0)                                                                                                      |
| `ruff check plugins/ crates/aphrodite-hermes/codegen/`                                     | only the 2 known pre-existing perf-probe errors (UP031 :119, W292 :191 in `plugins/aphrodite/tests/test_perf_probe.py`) |
| `npx prettier --check .hermes/notes/*.md`                                                  | clean                                                                                                                   |
