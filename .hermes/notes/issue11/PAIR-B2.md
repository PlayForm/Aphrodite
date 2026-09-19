# PAIR-B2 - Issue #11 WS2-preview + WS4 landed (preview.rs + config wiring)

**Repo:** PlayForm/Aphrodite, branch `Development`, binary 1.4.6 / plugin 2.1.4
**Date:** 2026-09-17
**Ownership:** `crates/aphrodite/src/preview.rs` + `crates/aphrodite/src/config.rs` family
(engine binary + config loader + template) + `crates/aphrodite-hermes/src/lib.rs` (dylib
init). `crates/aphrodite-hermes/src/tools.rs` is partner B1's file - NOT touched.
**Predecessors:** `.hermes/notes/ISSUE-11-ROOT-CAUSE.md`, `ISSUE-11-FIXDESIGN.md`,
`ISSUE-11-PREVIEW-BATTERY.md`, `SESSION-DISPATCH.md` (Pair B2), `HANDOFF-2026-09-17.md`.

---

## 1. What changed

### 1.1 WS2-preview - honest build arm (`crates/aphrodite/src/preview.rs`)

| Behavior                       | Before (1.4.5)                                                                                                                                                                                                                                                                                                                   | After (fixed)                                                                                                                                                                                                                                                                                                         |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Line-based tallies             | tallied with `content.matches("error").count()` / `content.matches("warning").count()` - substring counting that inflated lines with repeated occurrences, matched inside unrelated words (`noerror`, `error-prone`), and MISSED capitalized `Error:` (Python/Swift/clang output). A genuinely failed build could render as `0E` | `error`/`warning` counts come from `is_error_line` / `is_warning_line` (rustc/clang/gcc `error[E0432]:`, `error:`, `Error:`, `ERROR`, `panicked at`, `file:line: error[`, Python `ValueError:`/`Exception:`; warning equivalents). Counts are honest per line, case-insensitive on the conventional forms             |
| Failure honesty                | never surfaced a FAILING test run: `test result: FAILED. 3 failed` previewed as `[build:0E 0W 3L]` - a failing run looked exactly like a clean build (battery top-5 offender #2)                                                                                                                                                 | when no error/warning line is tallied but the output carries a FAILURE signal (`FAILED`, `FAIL`, `--- FAIL:`, non-zero `N failed` via `is_failure_line`), the failure summary line is surfaced instead of a clean-looking `[build:0E 0W N L]`. `0 failed` (passing runs) never matches, so clean summaries stay clean |
| First-error-message enrichment | followed the old substring matching - a capitalized `Error:` line was counted, not surfaced                                                                                                                                                                                                                                      | now matches the same markers as the tally (a capitalized `Error:` line is surfaced, not just counted)                                                                                                                                                                                                                 |

Verified on the production dylib path (`aphrodite_compress` with the Hermes
`{"output":...,"exit_code":N}` wrapper, which routes to `build_output`),
exercised through the PLUGIN's hardened FFI path (import the plugin package,
`_load_dylib` + `_call_json`, which applies the generated `_bindings.py`
restype/argtypes and forces `c_void_p`):

| case                                                                                        | before                                         | after                                                                            |
| ------------------------------------------------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------- |
| failing test run                                                                            | `[build:0E 0W 3L]` (looks clean)               | `[build:0E 0W 3L \| test result: FAILED. 1 passed; 2 failed; finished in 0.05s]` |
| passing test run                                                                            | `[build:0E 0W 3L]`                             | `[build:0E 0W 3L]` (unchanged - no false failure flag)                           |
| `error[E0432]: error in crate foo` + `Error: failed to build` + `warning:` + `noerror here` | 3E (occurrences) / missed the capitalized line | `[build:2E 1W 4L ...]` (2 error lines, 1 warning line)                           |

### 1.2 WS2-preview - honest error/linter/log arms (`preview.rs`)

| Behavior              | Before (1.4.5)                                                                                                                                                                                   | After (fixed)                                                                                                                   |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `error` arm           | fell into the generic `_` arm, which shows the FIRST non-empty line - for a Python traceback that is `Traceback (most recent call last):` (the header, not the error), hiding the actual failure | first real error/failure line (`is_error_line`/`is_failure_line`), else the last non-empty line (tail = most recent state)      |
| `linter` / `lint` arm | generic `_` arm - first non-empty line                                                                                                                                                           | first issue line (`path:line:col:` prefix, `E###`/`W###`/`F###` codes via `is_lint_line`, `error:`/`warning:` lines), else tail |
| `log` arm             | generic `_` arm - for a compiler log the first `Compiling` line (innocuous), hiding the actual failure                                                                                           | error/failure line if any (signal wins), else the last non-empty line (tail)                                                    |

Verified via the release dylib through the plugin's hardened FFI path
(`aphrodite_compress` with explicit type hint):

| case                                          | before (generic first-line)                            | after                                                          |
| --------------------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------- |
| traceback (`ValueError: disk full` at line 3) | `[error:3L 91B \| Traceback (most recent call last):]` | `[error:3L 91B \| ValueError: disk full]`                      |
| ruff output                                   | first line (could be noise)                            | `[lint:2L 81B \| src/x.py:10:5: E501 line too long (98 > 88)]` |
| log with error                                | `[log:4L 62B \| INFO starting]`                        | `[log:4L 62B \| ERROR connection refused]`                     |
| log, no error                                 | first line                                             | `[log:2L 62B \| 2026-09-17T10:00:05Z INFO done]` (tail)        |

### 1.3 WS4 - wire `preview_max_chars` end-to-end

| Behavior                                                    | Before (1.4.5)                                                                                                                                                                | After (1.4.6)                                                                                                                                                                                                                                        |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `preview_max_chars` enforcement                             | `PreviewsConfig.preview_max_chars` (`crates/aphrodite/src/config.rs`) was declared but never read anywhere (ROOT-CAUSE §3 confirmed exactly 2 occurrences, both declarations) | the cap is now enforced at the single choke point - the end of `build_preview` - so every path (proxy, hooks, prefetch, poll_worker, engine C-ABI, and the Hermes dylib) renders bounded previews                                                    |
| End-to-end verification (env override, no config file edit) | no cap applied (unlimited previews)                                                                                                                                           | `APHRODITE_PREVIEW_MAX_CHARS=20` on a 311-byte prose payload produced `[text:1L 311B \| so…]` - exactly 20 chars, closing bracket + `…` preserved. The shipped config default (120) bounds the normal case; absent key = unlimited (legacy behavior) |

Wiring path:

1. `crates/aphrodite/src/preview.rs`: `PREVIEW_MAX_CHARS` atomic (0 = unlimited),
   `set_preview_max_chars(Option<u32>) -> Option<u32>` (returns previous value for
   restore/hot-reload), `apply_preview_cap()` applied to every `build_preview` result.
   Truncation is char-boundary safe and preserves the closing `]` (with a `…` marker)
   so `render_marker`/`parse_preview`/`chain_split` keep working - a bare cut that
   dropped `]` would re-trigger the `[text:[text:...]]` double-wrap bug class.
2. `crates/aphrodite/src/main.rs`: after `MultiConfig::load`, the engine binary pushes
   `config.previews.preview_max_chars` into the builder (absent = unlimited).
3. `crates/aphrodite/src/proxy.rs` (`handle_ccr_reload`): same call on hot-reload.
4. `crates/aphrodite/src/config_loader.rs`: new `Config::apply_previews()` resolves
   `[previews] preview_max_chars` with the standard env > TOML > default chain
   (`APHRODITE_PREVIEW_MAX_CHARS`), clamping to `u32`.
5. `crates/aphrodite-hermes/src/lib.rs` (`shared()` init): dylib applies
   `Config::load().apply_previews()` alongside `apply_compression`, so the production
   Hermes path caps previews exactly like the proxy path.
6. `crates/aphrodite/templates/aphrodite.toml`: comment updated (the key was already
   documented at `preview_max_chars = 120`).

Verified end-to-end on the dylib (env override, no config file edit): see the
`preview_max_chars` enforcement table above.

## 2. Tests

New tests (12) - all in `crates/aphrodite`:

- `preview.rs` (11): build line-counts vs substring-counts; failing test run never looks
  clean; passing test run stays clean; capitalized `Error:` surfaced; error arm surfaces
  the error line not the traceback header; linter arm surfaces the issue line; log arm
  surfaces error signal or tail; cap truncates to ≤ cap with closing bracket preserved;
  cap unset = unlimited; multibyte truncation on char boundary; cap applies to the build
  arm too.
- `config_loader.rs` (1): `apply_previews` wires TOML value, env override wins, absent/0
  = unlimited (with a process-wide cap guard shared with `preview.rs` tests).

Results:

- `cargo test -p aphrodite`: **389 passing, 0 failed** (lib 360 passed + 1 ignored; bins
  16+5+2+4+2). Baseline was 348; +12 new, +9 net from prior session drift.
- `cargo test -p aphrodite-hermes`: **52 passing, 0 failed** (baseline 46; +6 from B1's
  tools.rs work - untouched here).
- `cargo build --release -p aphrodite-hermes`: clean; build.rs reported "header unchanged
    - skipping ctypesgen/finalize", i.e. the committed `_bindings.py` stays byte-identical
      (no FFI surface change, no shim/template `__init__.py` touch).
- Repro: `python3 repro.py` (6 threads x 300 hammer) → **SURVIVED: no crash** (exit 0).
- Verification method: all preview checks were re-run through the PLUGIN's hardened
  FFI path (`_load_dylib`/`_call_json`, generated `_bindings.py` restype/argtypes,
  forced `c_void_p`, same-handle free) - zero new SIGSEGV; the two crash reports in
  the window (23:57 / 00:04, `_platform_strlen <- string_at`) are the raw-ctypes
  probe class (initial probe run + a partner's probe), not the plugin path.

## 3. Files touched

- `crates/aphrodite/src/preview.rs` - build arm honesty, error/linter/log arms, cap
  machinery + tests
- `crates/aphrodite/src/config_loader.rs` - `apply_previews()` + test
- `crates/aphrodite/src/main.rs` - cap set at startup
- `crates/aphrodite/src/proxy.rs` - cap set on `/reload`
- `crates/aphrodite/templates/aphrodite.toml` - comment only (key was already documented)
- `crates/aphrodite-hermes/src/lib.rs` - dylib init applies previews config

NOT touched: `crates/aphrodite-hermes/src/tools.rs` (B1), `plugins/aphrodite/__init__.py`
and `crates/aphrodite/templates/__init__.py` (shim byte-identity preserved; no commit
made - auto-committer sweeps).

## 4. Remaining deferrals

- WS3 proxy-pipeline parity (`proxy.rs` parallel preview arms) stays deferred to 1.5.0.
- `model_family`, `code_structure_map`, `rust_preview_lines` remain declared-unread knobs
  (out of scope; only `preview_max_chars` was wired per dispatch).
