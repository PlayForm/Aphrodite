# RESEARCH-UPSTREAM.md - Upstream ctypesgen + cbindgen output shapes (FFI pipeline research)

**Agent:** research (upstream side) · **Consumer:** development agent (implements)
**Date:** 2026-09-17 · **Scope:** RESEARCH ONLY - no repo modified, nothing committed.
**Durable copy:** `.hermes/notes/RESEARCH-UPSTREAM.md` (this file is the prettier-formatted durable record; the sigserve copy is the coordination scratch).

---

## 0. Provenance / what is authoritative

| Source                                  | Path                                                                             | What it is                                                                                                                                                                                                   |
| --------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Upstream fork (research)                | `ctypesgen/` (Module tree)                                                       | git repo, last commit `a90952d "Add 'bool' to ctypes type map, needed for C23 (#225)"` - the **older** printer (`printer_python/`), retains the single-library shortcut                                      |
| Installed (what build.rs ACTUALLY runs) | the installed `ctypesgen` package (`site-packages`, Homebrew python 3.14)        | console script self-reports **`2.7.4-27202-gb3625f73d3`** (VERSION file says `1.1.1`); printer emits the **`for _lib in _libs.values():`** loop form for every function (`-l` does not set `source_library`) |
| pypdfium2 fork                          | `ctypesgen-pypdfium2/` (Module tree)                                             | `src/ctypesgen/` - heavily restructured printer (`printer_python.py`), no preamble classes                                                                                                                   |
| cbindgen                                | `cbindgen/` (Module tree)                                                        | last commit `2b757a2`; the Aphrodite build uses **cbindgen 0.29** (`crates/aphrodite-hermes/Cargo.toml:28`)                                                                                                  |
| Real generated artifacts                | the crate's `OUT_DIR` build artifacts (`aphrodite_hermes.h`, `_bindings.raw.py`) | regenerated 2026-09-17 22:33 by the actual build.rs chain - the ground truth for every regex verdict below                                                                                                   |

**Key finding:** the installed ctypesgen's `ctypedescs.py` restype logic is **byte-identical in behavior** to the Module fork (verified by diff of the `CtypesFunction` region - same `CtypesNoErrorCheck`, `CtypesPointerCast`, `void*→POINTER(c_ubyte)+cast`, `c_char_p`/`String` branch). The only divergence that matters for the pipeline is the **loop form**: installed emits `for _lib in _libs.values():` (→ `_BLOCK_START_RE` matches), the Module fork emits `if _libs["X"].has(...)` for a single `-l` library (→ `_BLOCK_START_RE` does NOT match the declaration loops). See §4, gap G3.

---

## 1. Output-shape catalog - upstream ctypesgen (Module fork + installed 2.7.4)

All shapes verified empirically: installed 2.7.4 on the real `aphrodite_hermes.h` (`_bindings.raw.py`), and the Module fork run from the Module tree (`PYTHONPATH=... python3 -m ctypesgen -l testlib -o /tmp/ctg_test/out_moduletree.py /tmp/ctg_test/test.h`) on a 12-function probe header covering every pointer-returning form.

### 1.1 Restype decision logic (source)

`ctypesgen/ctypesgen/ctypedescs.py` (Module fork; identical in installed 2.7.4):

| Line(s) | Code                                                                                                                           | Consequence                                                                           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| 241-248 | `class CtypesNoErrorCheck` - `py_string()` → `"None"`, `__bool__()` → `False`                                                  | default errcheck is **falsy** ⇒ never printed for fixed functions                     |
| 251-256 | `class CtypesPointerCast` - `py_string()` → `"lambda v,*a : cast(v, {})"`                                                      | printed only when `__bool__` truthy (no override ⇒ True)                              |
| 263     | `self.errcheck = CtypesNoErrorCheck()`                                                                                         | every function starts with the falsy default                                          |
| 269-276 | `void *` restype → `self.restype = CtypesPointer(CtypesSpecial("c_ubyte"), ())` **+** `errcheck = CtypesPointerCast(c_void_p)` | `void*`/`const void*` returns ⇒ `POINTER(c_ubyte)` + cast-lambda errcheck             |
| 278-283 | `POINTER(c_char)` restype → **`c_char_p`** if `"const" in self.restype.qualifiers`, else **`String`**                          | `const char *`/`char * const` returns ⇒ `c_char_p`; plain `char *` returns ⇒ `String` |
| 296-299 | `py_string()` → `"CFUNCTYPE(UNCHECKED(%s), %s)"`                                                                               | only used for function-pointer-typed items, not exported fns                          |

Empirically the parser propagates the pointee-`const` onto the `CtypesPointer.qualifiers`, so BOTH `const char *` and `char * const` returns hit the `c_char_p` branch (probe header lines `f_return_const_char` → `restype = c_char_p`, `f_return_const_ptr_char` → `restype = c_char_p`).

### 1.2 Fixed-function emission (`printer_python/printer.py:300-353`)

```
# srcinfo comment:  # <abs-path>/<header>: <lineno>      (line 301)
for _lib in _libs.values():                               # lines 317-324 (multi-lib; installed 2.7.4 ALWAYS this form)
    if not _lib.has("NAME", "cdecl"):
        continue
    NAME = _lib.get("NAME", "cdecl")
    NAME.argtypes = [A0, A1, ...]                         # lines 327-330
    [String branch, lines 333-342:]
    if sizeof(c_int) == sizeof(c_void_p):
        NAME.restype = ReturnString
    else:
        NAME.restype = String
        NAME.errcheck = ReturnString
    [else branch, lines 344-350:]
    NAME.restype = <RT>                                   # e.g. None / c_int / c_char_p / POINTER(c_ubyte) / POINTER(c_int)
    NAME.errcheck = <EC>                                  # ONLY if errcheck truthy (void* cast lambda)
    break                                                 # line 352-353
```

Per-return-type emission (verified raw forms):

| C return                         | Emitted shape                                                                                                                        | Where                                            |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------ |
| `char *`                         | `if sizeof(c_int) == sizeof(c_void_p): NAME.restype = ReturnString` / `else: NAME.restype = String` + `NAME.errcheck = ReturnString` | real raw `_bindings.raw.py:876-880` (and 8 more) |
| `const char *` / `char * const`  | single line `NAME.restype = c_char_p` (no errcheck)                                                                                  | Module run `out_moduletree.py:885, 891`          |
| `void *` / `const void *`        | `NAME.restype = POINTER(c_ubyte)` + `NAME.errcheck = lambda v,*a : cast(v, c_void_p)`                                                | Module run `out_moduletree.py:897-898, 904-905`  |
| `int *` / `unsigned char *` etc. | `NAME.restype = POINTER(c_int)` / `POINTER(c_ubyte)` (no errcheck)                                                                   | Module run `out_moduletree.py:911, 917`          |
| `void`                           | `NAME.restype = None`                                                                                                                | real raw `_bindings.raw.py:915`                  |
| `int`                            | `NAME.restype = c_int`                                                                                                               | Module run `out_moduletree.py:929`               |

Argtypes observed: `[]`, `[c_int, c_double]`, `[String]`, `[String, String]`, `[c_int, String, POINTER(None), String]`. Note `char*` **args** are always `String` (const or not - the c_char_p branch only runs for restypes), and `void*` args emit `POINTER(None)`.

### 1.3 Variadic emission (`printer_python/printer.py:355-392`)

```
for _lib in _libs.values():          # lines 377-392 (no source_library)
    if _lib.has("NAME", "cdecl"):
        _func = _lib.get("NAME", "cdecl")
        _restype = String            # unconditional py_string - "None" for no-error-check
        _errcheck = None
        _argtypes = [String]
        NAME = _variadic_function(_func,_restype,_argtypes,_errcheck)
```

Verified Module run `out_moduletree.py:932-938`. Note: variadic functions are NOT wrapped in the declaration loop shape; the `_restype = String` / `_errcheck = None` lines have **no dot**, so `_RESTYPE_RE`/`_ERRCHECK_RE` leave them alone (see §4, gaps G4/G5).

### 1.4 Preamble content (`printer_python/preamble.py`, embedded whole via `print_preamble` printer.py:132-143)

Verified present verbatim in real raw `_bindings.raw.py:11-440` AND retained in the final artifact (postprocess never touches `head`):

| Content                                                                                                                             | Source lines        | Retained in artifact?                            |
| ----------------------------------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------ |
| `import ctypes` / `import sys` / `from ctypes import *  # noqa: F401, F403`                                                         | preamble.py:1-3     | yes - namespace bleed, **no `__all__` anywhere** |
| `c_ptrdiff_t` size-selection loop (`_int_types`, `c_int16/c_int32/c_int64`)                                                         | preamble.py:5-15    | yes                                              |
| `class UserString` (~230 lines, full stdlib UserString port)                                                                        | preamble.py:19-252  | yes                                              |
| `class MutableString(UserString)`                                                                                                   | preamble.py:255-320 | yes                                              |
| `class String(MutableString, ctypes.Union)` (`_fields_ = [("raw", POINTER(c_char)), ("data", c_char_p)]`, `from_param` classmethod) | preamble.py:323-372 | yes - **required**: argtypes reference `String`  |
| `def ReturnString(obj, func=None, arguments=None)`                                                                                  | preamble.py:375-376 | yes                                              |
| `def UNCHECKED(type)`                                                                                                               | preamble.py:386-390 | yes                                              |
| `class _variadic_function`                                                                                                          | preamble.py:395-414 | yes                                              |
| `def ord_if_char(value)`                                                                                                            | preamble.py:417-425 | yes                                              |

Loader block (real raw `_bindings.raw.py:442-860`): `_libs = {}`, `_libdirs = []`, `# Begin loader`, full `LibraryLoader`/`Lookup`/`DarwinLibraryLoader`/`PosixLibraryLoader` (+ `ld.so.conf` cache)/`WindowsLibraryLoader`/`loaderclass`, `load_library = loaderclass.get(...)()`, `add_library_search_dirs([])`, `# End loader`. Also retained in the artifact.

Header/docstring: `r"""Wrapper for <header>` + generated command line + `Do not modify this file.` (raw 1-7), `__docformat__ = "restructuredtext"` (raw 9), `# Begin preamble for Python` / `# End preamble` markers (raw 11/440), `# Begin libraries` … `# End libraries` with the load line (raw 862-866), `# No modules`, `# No prefix-stripping`.

**Size accounting (final artifact):** 31,692 B / 982 lines total; head (preamble + loader + libraries) = **27,519 B (~87%)**; the rewritten declaration loops + `bind_to()` binder = ~4,200 B. The lean-ification target is the head.

---

## 2. cbindgen declaration format (whitespace question - ANSWERED)

**cbindgen emits `char *name` - one space between `char` and `*`, ZERO spaces between `*` and the name.** Args: `const char *tool_name` (qualifier BEFORE the type, then `*name`).

Proof - the real generated header (cbindgen 0.29 via build.rs, `out/aphrodite_hermes.h:13`):

```
char *aphrodite_hermes_dispatch_tool(const char *tool_name, const char *args_json);
char *aphrodite_hermes_list_tools(void);
void aphrodite_hermes_free_string(char *s);
```

So for `#[no_mangle] pub unsafe extern "C" fn ... -> *mut c_char` the emitted declaration is exactly `char *name(...)` - the current regexes `_HEADER_FN_RE`/`_HEADER_PTR_RE` (`char\s*\*` then `\s*` then name) match this fine; they would also tolerate `char * name`. The task's stated concern (whitespace between `*` and name) does **not** bite.

Source mechanics (`cbindgen/src/bindgen/cdecl.rs`): `write()` emits `type_name` ("char", line 208), then a space **before** the declarators when an identifier follows (line 225-227), then the `Ptr` declarator writes `*` (line 241), then the identifier immediately (line 271-272). Pointee const goes into `type_qualifiers` written BEFORE the type name (lines 198-199, 132-138) ⇒ `const char *name`, NOT `char const *name` - **the comment at `finalize_bindings.py:67` (`char const *tool_name`) is stale/wrong**; actual output is `const char *tool_name`. Cosmetic only (the regexes match only the `char *`/`void` return prefix).

Related config (`cbindgen/src/bindgen/config.rs`): `ExportConfig { include: Vec<String>, exclude: Vec<String> }` at lines 324-329 (Aphrodite `cbindgen.toml` uses `[export] include = ["aphrodite_hermes_*"]`); `no_includes: bool` at line 892, default `false` (line 1015) - the real header DOES carry `#include <stdarg.h>/<stdbool.h>/<stdint.h>/<stdlib.h>` plus the `#ifndef APHRODITE_HERMES_H` include guard.

---

## 3. Divergence table - upstream ctypesgen vs pypdfium2 fork

| Aspect                | Upstream (Module fork + installed 2.7.4)                                                                                                    | pypdfium2 fork (`src/ctypesgen/`)                                                                                                                                                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `char *` restype      | `String` (plain) / `c_char_p` (const-qualified) - ctypdescs.py:278-283                                                                      | **stays `POINTER(c_char)`** unless `--string-template` given; then `String` (POINTER(c_char)) or `WideString` (POINTER(c_wchar)) - ctypdescs.py:260-267 (the `options`-gated block, replacing the removed upstream 278-283)                                |
| `c_char_p` branch     | present (const check, line 280-282)                                                                                                         | **removed** - no `c_char_p` restype ever emitted                                                                                                                                                                                                           |
| `void *` restype      | `POINTER(c_ubyte)` + `errcheck = lambda v,*a : cast(v, c_void_p)` (ctypedescs.py:269-276, 251-256)                                          | **removed** (`CtypesPointerCast` class deleted - upstream 251-258 absent; the 269-276 block absent); `void*` passes through as parser produced it (FIXME comment at ctypdescs.py:260 references `CtypesParser.get_ctypes_type()` as the intended location) |
| errcheck emission     | fixed fns: only when truthy (`CtypesNoErrorCheck.__bool__` False ⇒ nothing; cast-lambda ⇒ printed); variadic: `_errcheck = None` literal    | same truthiness gate - printer_python.py:208-210 (`if function.errcheck:`)                                                                                                                                                                                 |
| restype emission      | fixed fns: `NAME.restype = <RT>` + optional `NAME.errcheck`; String ⇒ if/else `ReturnString`/`String`+`errcheck` block (printer.py:333-350) | `NAME.restype = <RT>` + optional errcheck only (printer_python.py:196-210) - **no ReturnString/String if/else block at all**; a `String` restype prints as bare `NAME.restype = String`                                                                    |
| `ptrdiff_t`/`ssize_t` | `c_ptrdiff_t` + preamble size-selection loop (ctypedescs.py:72-73, preamble.py:5-15)                                                        | `c_ssize_t` builtin (ctypedescs.py:70-71) - no preamble loop                                                                                                                                                                                               |
| Preamble              | UserString/MutableString/String/ReturnString/UNCHECKED/_variadic_function/ord_if_char + `from ctypes import *` (preamble.py:1-425)          | none of those; `from ctypes import *` + `UNCHECKED` from `T_UNCHECKED` template (`templates.py:4-11`); optional user `--string-template` file injected (printer_python.py:132-136)                                                                         |
| Variadic fns          | `_variadic_function` wrapper emission (printer.py:355-392)                                                                                  | **no variadic printer path** - `print_function` (printer_python.py:191-215) has no variadic branch; `variadic` flag carried only in descriptions.py:170/183 and printer_json.py:97                                                                         |
| Output framing        | `r"""Wrapper for ..."""` docstring + `# Begin preamble for Python` markers + full loader embed                                              | `R"""..."""` raw docstring with cmd line (printer_python.py:81) + `# -- Begin/End <section> --` paragraph markers (lines 26-34) + optional external `_ctg_loader.py`                                                                                       |
| Loop form             | single `-l` ⇒ `if _libs["X"].has(...)` (fork) / always `for _lib in _libs.values():` (installed 2.7.4)                                      | `{PN} = _libs[{L!r}][{CN!r}]` direct subscript (printer_python.py:197) - no loops, no `has/get`                                                                                                                                                            |
| `__all__`             | absent                                                                                                                                      | absent                                                                                                                                                                                                                                                     |

**Bottom line for the dev agent:** pypdfium2 is a redesign, not a backport candidate - its shapes (`restype = String` bare single-line, no if/else block, direct `_libs[L][CN]` binding, no loader embed) would need a different finalize_bindings.py. The pipeline is correctly matched to upstream (installed 2.7.4) today.

---

## 4. Regex-coverage verdict (per pattern, against REAL raw output)

Verified by executing `finalize_bindings.py`'s own regexes against (a) the real `_bindings.raw.py` (installed 2.7.4 output - what build.rs processes) and (b) the Module-fork probe output; plus a full `postprocess` + `validate` run on the real artifacts → **VALIDATE: PASS, 10 exports, 9 pointer-returning, 0 violations**, final artifact 31,692 B / 982 lines.

| Pattern (file:line)                                | Real-raw count          | Verdict                        | Notes                                                                                                                                                    |
| -------------------------------------------------- | ----------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `_BLOCK_START_RE` (88)                             | 10/10 loops             | ✅ COVERED                     | matches `for _lib in _libs.values():` - installed 2.7.4 always emits this form                                                                           |
| `_LOAD_LINE_RE` (93-95)                            | 1/1                     | ✅ COVERED                     | → `_libs["__APHRODITE_DYLIB__"] = None  # bound at runtime via bind_to()` (final artifact line 862)                                                      |
| `_LOOKUP_CALL_RE` (92)                             | 20/20 (10 has + 10 get) | ✅ COVERED                     | → `hasattr/getattr(_lib, "NAME")`; guard at line 242 passes                                                                                              |
| `_RESTYPE_RE` if/else branch (101-108)             | 9/9                     | ✅ COVERED                     | every `char *` return rewritten to `NAME.restype = c_void_p` (final has 9)                                                                               |
| `_RESTYPE_RE` single-line (109)                    | 0 (real raw)            | ✅ COVERED (unreachable today) | single-line `restype = String` is structurally unreachable from `print_fixed_function` (String always takes the if/else branch); kept as belt-and-braces |
| `_ERRCHECK_RE` (122)                               | 9/9                     | ✅ COVERED                     | strips all `errcheck = ReturnString`; also strips `errcheck = lambda v,*a : cast(v, c_void_p)` (verified on probe output - 4/4)                          |
| `_HEADER_FN_RE` / `_HEADER_PTR_RE` (71-74)         | 10 / 9                  | ✅ COVERED                     | matches real header incl. `void name(void)` and multi-arg lines; param counting correct                                                                  |
| `_DECLARED_NAME_RE` (124)                          | 10                      | ✅ COVERED                     | post-rewrite `hasattr(_lib, "NAME")` scan equals header set                                                                                              |
| `_DOCSTRING_RE` (129) / `_HEADER_COMMENT_RE` (130) | 1 / 10                  | ✅ COVERED                     | docstring canonicalized; `# <abs-path>/aphrodite_hermes.h: N` → `# aphrodite_hermes.h: N`                                                                |

**Gaps found (none currently active, all fail loudly not silently - except G6 which is cosmetic):**

- **G1 - `restype = c_char_p` single-line: GAP.** `_RESTYPE_RE` single-line alternation is `(?:ReturnString|String)` only (line 109). A `const char *`/`char * const` return emits `NAME.restype = c_char_p` (probe `out_moduletree.py:885,891`) → **not rewritten to c_void_p**, stays `c_char_p`. Validation still passes (`_pointer_width(c_char_p)` True, finalize line 195) - no correctness bug, but the stated invariant "every pointer restype is textually c_void_p" (docstring lines 27-33) would be false. Current header has no const char* returns, so dormant.
- **G2 - single-library loop form (`if _libs["X"].has(...)`): GAP, fails LOUDLY.** The Module-tree fork's printer keeps `use_single_lib = function.source_library or len(self.options.libraries) == 1` (Module printer.py:307-315) ⇒ single `-l` runs emit `if _libs["testlib"].has(...)`. `_BLOCK_START_RE` then finds only a later variadic `for` loop (probe output has 1 match - the variadic block), `postprocess` wraps a mixed body, and `_LOOKUP_CALL_RE` misses the `_libs["X"].has(...)` calls (pattern requires `_lib.`). Result: `validate()`'s `bind_to()` replay hits `KeyError: 'testlib'` → **ContractViolation → exit 1 → build PANICS** (loud, not silent). Installed 2.7.4 never emits this form (its printer tests only `function.source_library`, installed printer.py:307), so the current toolchain is safe. The `_LOOKUP_CALL_RE` guard at line 242 does NOT catch the mixed case (the variadic `_lib.has` matches satisfy `'hasattr(_lib, "' in loops`).
- **G3 - variadic `_restype = String`: GAP, fails LOUDLY if a variadic pointer-returning export is ever added.** `_restype = String` (probe `out_moduletree.py:935`) has no dot ⇒ `_RESTYPE_RE` leaves it. Today no variadic fns exist; if one returned a pointer, `validate()` would flag `rec.restype` None (the binding is a `_variadic_function` instance, record restype stays None → `_pointer_width(None)` False → violation → panic). Non-pointer variadic (e.g. printf-like) would work fine.
- **G4 - `POINTER(None)` in argtypes: not a regex issue**, but note `void*` args emit `POINTER(None)` which is legal ctypes (alias for `c_void_p`); no current header function has void* args.
- **G5 - variadic `_errcheck = None` / `_restype = POINTER(...)`: intentionally untouched, correct.**
- **G6 - stale comment `finalize_bindings.py:67`**: says `char const *tool_name`; cbindgen actually emits `const char *tool_name`. Comment-only.

**Task-specific confirmations:**

- c_char_p single-line branch: **emitted by upstream** (const-qualified char* returns), **NOT rewritten to c_void_p** by the current regex - stays c_char_p (pointer-width, validation-accepted). Recommend G1 fix for the textual invariant.
- `CtypesNoErrorCheck` rendering: fixed functions → **nothing** (falsy `__bool__`, printer.py:347 gate); variadic → literal **`_errcheck = None`**.
- UserString class + c_ptrdiff_t loop: **present in raw output** (raw 17-27, 31-264) and **left in the artifact** (head untouched by postprocess).
- `__all__`: **absent** from raw and final. Plugin imports via `from . import _bindings as _GENERATED_BINDINGS` (plugins/aphrodite/**init**.py:78) and consumes only `bind_to(dylib)` (line 442) - no star-import leak today. The `__all__ = ["bind_to"]` proposal is hygiene/introspection, not an active-leak fix.

---

## 5. Recommendations (concrete, for the development agent)

Each recommendation names the `finalize_bindings.py` target (file:line = `crates/aphrodite-hermes/codegen/finalize_bindings.py`) and the exact change. All are self-verifying: `validate()` (lines 258-325) executes the artifact and replays `bind_to()` against the stub - a wrong deletion/rewrite fails the build (exit 1), never ships silently.

| #      | Target (file:line)                                                            | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Why / risk                                                                                                                                                                                                                                                    |
| ------ | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **R1** | `_RESTYPE_RE` single-line alternation, line 109                               | `(?:ReturnString\|String\|c_char_p)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Makes G1 textually true: const char* returns normalize to `c_void_p`. Zero risk (`_pointer_width` already accepts both); no current header fn affected.                                                                                                       |
| **R2** | new pattern after line 112 + a branch in `_restype_repl` (115-121)            | `^[ \t]+_restype = String$` → `_restype = c_void_p` (only the String value; `POINTER(...)`/`None` values stay)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Closes G3: a future variadic pointer-returning export would otherwise panic the build at validate. No behavior change today (no variadic fns).                                                                                                                |
| **R3** | `_BLOCK_START_RE` (88) and `_LOOKUP_CALL_RE` (92)                             | extend `_BLOCK_START_RE` with alternation `\|^if _libs\[["'][^"']+["']\]\.has\(`; extend `_LOOKUP_CALL_RE` to also match `_libs\[["'][^"']+["']\]\.(has\|get)\(...` → rewrite to `hasattr/getattr(_lib, ...)`                                                                                                                                                                                                                                                                                                                                                                                                                                                | Closes G2 - protects against any ctypesgen that emits the single-library `if _libs["X"].has(...)` form (the Module-tree fork does). Optional if the toolchain stays pinned to installed 2.7.4, but cheap insurance given the fork is the research reference.  |
| **R4** | new head-strip step in `postprocess` (after line 225, before `_LOAD_LINE_RE`) | Remove from `head`: the `# Begin loader` … `# End loader` block (raw 442-860, ~420 lines incl. all `LibraryLoader` classes, `loaderclass`, `load_library`, `add_library_search_dirs([])`), and `# Begin libraries`/`# End libraries` markers; keep `_libs = {}` (bind_to assigns `_libs[PLACEHOLDER]`) and the rewritten load line. Also `# No modules`/`# No prefix-stripping` markers.                                                                                                                                                                                                                                                                     | Kills ~60% of the 27.5 KB head. **Keep `String`/`MutableString`/`UserString`/`ReturnString` unless R5 lands** - argtypes still reference `String` (raw 875/901/914/988). Do NOT remove `_variadic_function`/`UNCHECKED` while any shape could reference them. |
| **R5** | new `_ARG_TYPES_RE` step after line 241 + head strip of the String classes    | Rewrite argtypes `[String]` → `[c_char_p]` (`String, String` → `c_char_p, c_char_p`), then delete `class UserString` (preamble.py:19-252), `MutableString` (255-320), `String` (323-372), `ReturnString` (375-376) from the head - but ONLY if R4/R5's argtypes rewrite is in the same change (they must not land separately: a `String` reference with the classes deleted = NameError at validate = build panic, so the sequencing is enforced by validate itself). Behavior note: `c_char_p` argtypes accept bytes/str/None exactly like `String.from_param` for the plugin's usage (plugin passes JSON strings); `None`/0 → NULL is preserved by ctypes. | The 31.7 KB → ~12-14 KB lean-up the "lean-ify" effort wants. Safest ordering: R4 first, then R5.                                                                                                                                                              |
| **R6** | `BINDER_HEADER`, lines 141-160 (append after line 159)                        | add `\n__all__ = ["bind_to"]\n`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Proposal from the task: no `__all__` exists anywhere in raw/final; plugin currently imports module-qualified (no leak), this makes star-import/introspection safe.                                                                                            |
| **R7** | comment block lines 66-70                                                     | change `char const *tool_name` → `const char *tool_name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Stale comment (G6); regexes unaffected.                                                                                                                                                                                                                       |

**Explicitly NOT recommended:** rewriting `restype = None` (void fns) - correct as-is and validate already exempts non-pointer names (line 296 gate); rewriting `POINTER(c_ubyte)`/`POINTER(c_int)` single-lines - already pointer-width; switching the pipeline to pypdfium2 (different shapes entirely, see §3); touching `_variadic_function`/`UNCHECKED`/`ord_if_char` without a full shape audit of macros/callbacks (none in the current header - a future macro/callback would need them).

**Sequencing:** R1+R7 are trivial; R3 is optional insurance; R4 → R5 (with R6) is the lean-ification path, each step gated by the existing validate() so a mistake fails the build loudly.
