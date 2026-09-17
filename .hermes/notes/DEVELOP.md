# DEVELOP.md - FFI pipeline implementation (research findings -> code)

Date: 2026-09-17 · Branch: Development · Binary 1.4.6 / plugin 2.1.4 · macOS
Role: implementation half of the research/develop pair. Consumes
`RESEARCH-UPSTREAM.md` + `RESEARCH-FORK-INTEGRATION.md` (durable copies live in
`.hermes/notes/`; coordination scratch in the sigserve dir). This file is the
durable record: what was implemented from each report, what was deliberately
not, and the full verification matrix.

Files changed (this pass):

- `crates/aphrodite-hermes/codegen/finalize_bindings.py` (rewritten core:
  regex union, argtypes rewrite, dead-code strip, `__all__`, AST validator,
  fork shape-gate, comments)
- `crates/aphrodite-hermes/build.rs` (ctypesgen variant probe + cargo:warning;
  relays finalize's `CTYPESGEN_FORK_WARNING:` marker)
- `crates/aphrodite-hermes/codegen/test_finalize_bindings.py` (NEW - 23
  stdlib-unittest cases, one per regex form and contract check)
- `plugins/aphrodite/_bindings.py` (regenerated artifact: 981 lines -> 133
  lines; byte-stable across regenerations)

Not touched: `plugins/aphrodite/__init__.py` (hot-reload/lock/sentinel
machinery), the byte-identical template copy
(`crates/aphrodite/templates/__init__.py` - drift-guard re-verified identical),
`Maintain/check_ffi_contract.py`, CI workflows, package.json. No git hooks
exist; nothing committed manually (auto-committer sweeps).

---

## 1. Implemented from RESEARCH-UPSTREAM.md

**Report item:** **G1/R1** - `restype = c_char_p` single-line gap

**Implementation:** `_RESTYPE_RE` single-line alternation extended to `ReturnString|String|c_char_p|WideString`. A `const char *`/`char * const` return (upstream `CtypesFunction` rewrites POINTER(c_char)+const to `c_char_p`) and a `wchar_t *` return now normalize to `c_void_p` like every other pointer restype. Tests: `test_single_line_c_char_p`, `test_single_line_widestring`.

---

**Report item:** **G2/R3** - single-library `if _libs["X"].has(...)` form

**Implementation:** NOT rewritten (see §4 - deliberately handled by the shape gate instead; safer than the pre-change panic).

---

**Report item:** **G3/R2** - variadic `_restype = String`

**Implementation:** NOT implemented: a variadic export can never pass the AST validator anyway (no `NAME.restype/argtypes` assignment lines => missing from the declared set => contract violation, loud). Adding a dead rewrite buys nothing.

---

**Report item:** **G6/R7** - stale `char const *` comment

**Implementation:** Fixed: cbindgen emits `const char *tool_name` (pointee const BEFORE the type). Comment-only.

---

**Report item:** **R4** - strip the loader head

**Implementation:** `postprocess()` no longer keeps the raw head at all: the entire preamble (string machinery, `c_ptrdiff_t` loop, loader classes, `load_library`, `add_library_search_dirs`, `# Begin libraries` markers) is replaced by a 16-line minimal head - canonical docstring + `__docformat__` + `from ctypes import *` + `_libs = {}`. `_LOAD_LINE_RE` survives as a pure SHAPE check (the replacement line is discarded).

---

**Report item:** **R5** - argtypes `String` -> `c_char_p` + strip the String classes

**Implementation:** `_ARGYPES_STRING_RE` rewrites `\bString\b` -> `c_char_p` AFTER the restype/errcheck rewrites (so only argtypes entries still contain `String`; `ReturnString`/`WideString` have no word boundary at the capital). This is what makes the UserString/MutableString/String/ReturnString classes dead code. PLUS a runtime fix the report did not predict (see §2).

---

**Report item:** **R6** - `__all__`

**Implementation:** `__all__ = ["bind_to"]` in the minimal head. Verified by executing a real `from _bindings import *`: only `bind_to` is re-exported (the ctypes namespace bleed is contained).

## 2. Runtime fix beyond the reports (free_string argtypes)

The reports assumed `c_char_p` argtypes "accept bytes/str/None exactly like
`String.from_param` for the plugin's usage" - true for the JSON-payload calls,
FALSE for `free_string`: the plugin's `_call_json` passes it the raw pointer
**int** produced by the c_void_p restype of the call
(`plugins/aphrodite/__init__.py`::`_call_json` ->
`dylib.aphrodite_hermes_free_string(ptr)`). ctypesgen's `String` helper
accepted ints (the leniency the plugin relied on); plain `c_char_p` raises
`TypeError: 'int' object cannot be interpreted as ctypes.c_char_p`.

The repro caught this immediately (register() crashed). Fix:
`postprocess()` rewrites `aphrodite_hermes_free_string.argtypes = [c_char_p]`
-> `[c_void_p]`, mirroring the plugin's own `_manual_ffi_setup` (which has
declared `[ctypes.c_void_p]` all along - line 477). All other arg-carrying
entry points receive bytes (`_call_json` JSON payloads) or nothing, so
`c_char_p` is correct for them. Test: `test_free_string_argtypes_c_void_p`.

## 3. Implemented from RESEARCH-FORK-INTEGRATION.md

The fork (`pypdfium2-team/ctypesgen`, `pypdfium2` branch) is a redesign, not
a backport candidate (§1.5 verdict): top-level `NAME = _libs[L][N]`
declarations, no `for _lib in _libs.values():` loop, no if/else restype
block, no errcheck lines, no preamble classes, `_get_library` loader. One
finalize_bindings.py cannot serve both shapes, so the integration is
DETECTION + GRACEFUL SKIP:

- `classify_shape(raw)` returns `upstream-loop` (the `for _lib in
_libs.values():` shape - the only rewritable form), `fork-flat`
  (`NAME = _libs[...][...]` subscript), or `unknown`.
- Non-upstream shape: finalize prints `CTYPESGEN_FORK_WARNING: ...` to stderr
  and exits 2 - the committed artifact stays in effect. build.rs scans
  finalize's output for the marker and re-emits it as a `cargo:warning`, so
  the fork is identifiable in build logs.
- build.rs `probe_ctypesgen()` now returns the `--version` string;
  `report_ctypesgen_variant()` emits a `cargo:warning` when the version does
  not look like upstream ctypesgen (semver-ish prefix; the fork identifies
  itself by name). Upstream -> silent. The installed toolchain
  (`ctypesgen --version` -> `2.7.4-27202-gb3625f73d3`, upstream layout,
  emits the loop form) produces no warning.
- The restype rewrite patterns are the UNION of every known upstream form
  (if/else block + all single-line pointer restypes); the shape gate decides
  whether the union applies at all.

Tests: `test_classify_upstream/fork_flat/unknown`,
`test_fork_flat_raises_gracefully`.

Report items reviewed and NOT actioned (documented, future work):

- Fork switch (`--generator {upstream|fork}`): the installed tool and the
  committed artifact are upstream; switching would need a second rewrite set
  and re-verification against real fork output. Kept as detection+skip.
- `proxy_health` dead-export quarantine, hook-kwargs expressiveness,
  Rust teardown export, `codegen/fixtures/expected.h` baseline, rust-bindgen
  stress fixtures (§5/§6 of the fork report): out of scope for this pass
  (plugin/Rust-side decisions; the task pins the touched surface).

## 4. Item-by-item task mapping

1. Regex coverage: `c_char_p` single-line branch CONFIRMED at source level
   (upstream ctypdescs.py `CtypesFunction`, const branch) and covered;
   `WideString` added; `_IF_ELSE_RESTYPE_RE` + `_RESTYPE_SINGLE_RE` were
   already consolidated before this pass (LEAN-FINALIZE) and the single-line
   alt is extended. Tests per form.
2. Fork detection: version probe in build.rs (cargo:warning on non-upstream)
    - output-shape gate in finalize (`CTYPESGEN_FORK_WARNING` marker relayed
      as cargo:warning). Unknown fork -> warn + skip; the union of patterns is
      what the gate guards.
3. Dead-code strip: UserString/MutableString/String/ReturnString/UNCHECKED/
   `_variadic_function`/`ord_if_char`/`_int_types`/`c_ptrdiff_t` loop +
   entire loader section gone from the artifact (981 -> 133 lines). Verified:
   artifact imports, binds against the real dylib, and the repro survives.
4. `__all__ = ["bind_to"]`: present in the artifact; verified via real
   star-import (only `bind_to` re-exported).
5. AST-based validate: `validate()` no longer executes anything - `ast.parse`
    - `compile()` (no exec) + walk of `ast.Assign` nodes targeting
      `NAME.restype/argtypes/errcheck` only (same approach as
      `Maintain/check_ffi_contract.py`). Same semantics: declared == header
      exports, pointer-returning exports have pointer-width restype (c_void_p
      after rewrite, never c_int), argtypes counts match the header, errcheck
      absent, `--required` present. The `_StubDylib` replay and its
      `defaultdict`/`SimpleNamespace`/`ctypes` imports are gone.
6. CtypesNoErrorCheck: verified it renders NO line for fixed functions
   (`__bool__` is False -> printer's `if function.errcheck:` gate skips it;
   `restype = None` void fns carry no errcheck line to strip) - documented in
   the `_ERRCHECK_RE` comment.
7. errcheck nuance: `CtypesPointerCast(c_void_p)` (void* returns ->
   `POINTER(c_ubyte)` + cast lambda) is safe-to-keep but stripping is correct
   here because the restype rewrite runs FIRST (c_void_p full-width read;
   `_call_json` clamps to c_void_p anyway) - documented in the `_ERRCHECK_RE`
   comment and the binder header. Behavior unchanged.
8. Other low-risk fixes: argtypes `String` -> `c_char_p` (satisfies the
   "dispatch/call_hook argtypes c_char_p x2" contract textually), the
   free_string `c_void_p` runtime fix (§2), stale `char const *` comment
   (R7), `POINTER(c_ubyte)` single-line restype explicitly left as-is (it is
   pointer-width; the checker would flag it if a void* return ever lands -
   noted in the `_ERRCHECK_RE` comment as the CtypesPointerCast case).

## 5. Verification matrix (all after final state)

**Check:** `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`

**Result:** 23/23 OK

---

**Check:** `python3 -m py_compile` on all touched .py (+ plugin)

**Result:** OK

---

**Check:** Direct finalize run (exact build.rs args, `--required` 9 exports)

**Result:** exit 0, "validated 10 exports (9 required pointer-returning) - all pointer restypes are c_void_p, argtypes are c_char_p and match the header"

---

**Check:** Full pipeline regen (`cargo build -p aphrodite-hermes` with OUT_DIR header deleted)

**Result:** exit 0, no warnings (upstream variant -> silent), raw+header regenerated

---

**Check:** Second regen / direct regen

**Result:** artifact byte-stable (copy-on-change: unchanged)

---

**Check:** `python3 Maintain/check_ffi_contract.py`

**Result:** PASS, 0 violations / 0 warnings, restype source = generated bindings

---

**Check:** `python3 repro.py` (the sigserve repro, 6 threads x 300 hammer)

**Result:** SURVIVED: no crash, exit 0

---

**Check:** `cargo test -p aphrodite-hermes`

**Result:** 46 passed, 0 failed

---

**Check:** bind_to replay vs real release dylib

**Result:** dispatch_tool/call_hook `[c_char_p, c_char_p]` + c_void_p; free_string `[c_void_p]`, restype None; version c_void_p; live version() round-trip + free_string(int) OK

---

**Check:** Artifact shape

**Result:** no UserString/load_library/String/ReturnString/`c_ptrdiff_t` (grep 0); `__all__ = ["bind_to"]`; no hardcoded dylib path; no machine paths (docstring canonical, header comments normalized)

---

**Check:** Drift-guard

**Result:** `crates/aphrodite/templates/__init__.py` still byte-identical to `plugins/aphrodite/__init__.py` (diff -q identical; plugin untouched)

Artifact size: 31,716 B / 981 lines (committed before this pass) ->
5,586 B / 133 lines. Generator: 17,075 B / 409 lines -> ~24.5 KB / ~490
lines (the machinery lives in the generator, not the shipped artifact).

## 6. Known limitations / future work

- Single-library `if _libs["X"].has(...)` shapes (upstream fork's older
  printer) are classified "unknown"/"fork-flat" and SKIP gracefully (warn +
  exit 2). The installed 2.7.4 always emits the loop form, so this is
  unreachable today; if a ctypesgen emitting the single-lib form ever runs,
  the build warns and keeps the committed artifact instead of mangling.
- Variadic exports cannot pass the AST validator (no restype/argtypes
  assignment lines) - loud contract violation, by design.
- A future `void *` return would emit `restype = POINTER(c_ubyte)` (not
  rewritten; pointer-width, validator-accepted) and the checker would flag it
    - the CtypesPointerCast comment in finalize_bindings.py explains the
      decision; extend the single-line alternation when that ABI lands.
- Fork switch / `--generator` flag, `proxy_health` quarantine, fixtures,
  Rust teardown export: see RESEARCH-FORK-INTEGRATION.md rollup.

Anonymization note: this file and every deliverable from this pass contain no
local absolute paths - all references are relative ("the repo",
`crates/aphrodite-hermes/codegen/finalize_bindings.py`,
`plugins/aphrodite/_bindings.py`, "the sigserve repro (python3 repro.py)").
