# RESEARCH-FORK-INTEGRATION.md

Fork + integration-pattern research for the Aphrodite FFI pipeline (development agent handoff).
Scope: ctypesgen-pypdfium2 fork output shapes, hermes-agent/prime-agent consumer surface,
roto hot-reload prior art, FFI safety books, rust-ffi-example fixture baseline,
rust-bindgen test-corpus stress cases. RESEARCH ONLY - no repos modified.

All paths are relative to the Module tree root (the `Module/` checkout with the reference repos)
unless noted. The Aphrodite pipeline lives at the repo root (the Aphrodite monorepo checkout).

---

## 1. ctypesgen-pypdfium2 fork: output shapes (a)

### 1.1 Where restype is actually emitted

The known grep "found no printer in the expected path" because this fork renamed the printer:
upstream ctypesgen has `ctypesgen/printer_python/printer.py`, the fork has a single flat file
**`ctypesgen-pypdfium2/src/ctypesgen/printer_python.py`**. That is the real printer.

restype emission is in `printer_python.py:191-215` (`print_function`), via the template at
**`printer_python.py:196-200`**:

```
197  {PN} = _libs[{L!r}][{CN!r}]
198  {PN}.argtypes = ({ATS})
199  {PN}.restype = {RT}
```

with `RT = function.restype.py_string()` at **`printer_python.py:206`**. The fork emits a
**top-level** declaration per function - no `for _lib in _libs.values():` loop (that loop is an
upstream-ctypesgen invention; see upstream `Module/ctypesgen/ctypesgen/printer_python/printer.py:316-353`).
The optional `--guard-symbols` flag wraps the declaration in `if hasattr(_libs[L], CN):`
(`printer_python.py:212-213`).

### 1.2 Does the fork keep the c_char_p restype branch? NO.

Upstream ctypesgen has a const-aware branch in `CtypesFunction.__init__`
(`Module/ctypesgen/ctypesgen/ctypedescs.py:279-287`):

```
281  if "const" in self.restype.qualifiers:
283      self.restype = CtypesSpecial("c_char_p")   # const char* -> c_char_p
285  else:
286      self.restype = CtypesSpecial("String")
```

That is the "upstream branch that emits single-line c_char_p". **The fork deleted it.**
The fork's `CtypesFunction.__init__` (`ctypesgen-pypdfium2/src/ctypesgen/ctypedescs.py:251-266`)
contains only the `options.string_template` rewrite:

```
261  if options.string_template:
262      restype_str = self.restype.py_string()
263      if restype_str == "POINTER(c_char)":
264          self.restype = CtypesSpecial("String")
265      elif restype_str == "POINTER(c_wchar)":
266          self.restype = CtypesSpecial("WideString")
```

Consequences, verified against the fork source:

- **No string_template**: every `char*`/`wchar_t*` return stays `POINTER(c_char)` /
  `POINTER(c_wchar)` (`CtypesPointer.py_string` → `"POINTER(%s)"`, `ctypedescs.py:214-215`;
  `char -> c_char` map at `ctypedescs.py:27`). The fork never emits `c_char_p`,
  `String`, `WideString`, or `ReturnString` on its own.
- **With `--string-template FILE`** (flag at `__main__.py:371`; embedded at `printer_python.py:134-136`):
  `restype = String` / `restype = WideString`, where the names are whatever the template file defines.
  The fork's own recommended template is **`tests/string_template.py:1-3`**:
  `String = ctypes.c_char_p; WideString = ctypes.c_wchar_p` - i.e. String is a plain `c_char_p` alias,
  NOT upstream's UserString/String-union preamble.
- The fork's CHANGELOG states the design intent explicitly (`docs/CHANGELOG.md:21`):
  "The bloated string wrappers have been removed. By default, no implicit string encoding/decoding
  is being done anymore... the `--string-template` option allows to plug in your own string helpers
  (e.g. `c_char_p`...). However, we recommend that new code assume the raw `POINTER(c_char)` and
  cast/decode on the caller side."

### 1.3 errcheck: the fork emits NONE

- `CtypesFunction.__init__` hard-sets `self.errcheck = CtypesNoErrorCheck()` (`ctypedescs.py:255`);
  `CtypesNoErrorCheck.__bool__` returns False (`ctypedescs.py:241-248`).
- Nothing in the fork ever assigns a real errcheck (grep of `src/` for errcheck assignments:
  only `__main__.py:262` `--preproc-errcheck`, which is about the **preprocessor exit code**
  (`__main__.py:261-266`), not function errchecks).
- Printer emits errcheck only `if function.errcheck:` (`printer_python.py:208-210`) - dead branch.
  Upstream, by contrast, emits `ReturnString` as an errcheck for the sizeof fallback path
  (`printer.py:333-350`).

### 1.4 Other fork output-shape differences that matter to finalize_bindings.py

| Shape            | Upstream (current pipeline target)                                                                           | Fork                                                                                                                                                                                                                           |
| ---------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Docstring        | `r"""Wrapper for ...`                                                                                        | `R"""\nAuto-generated by:\n{cmd_str}"""` (`printer_python.py:81`)                                                                                                                                                              |
| Preamble         | embeds UserString/String/ReturnString + loader classes (~250 lines, cf. committed `_bindings.py:30-334`)     | only `import ctypes` + `from ctypes import *` (`printer_python.py:82-85`); no String class unless template file supplies one                                                                                                   |
| Library binding  | `_libs["X"] = load_library("X")`                                                                             | `_libs["X"] = _get_library(name, dllclass = ctypes.CDLL, libpaths = (...), search_sys = ...)` (`printer_python.py:165-188`); `_get_library` at `libraryloader.py:14-34` returns a plain `dllclass(path)` - a raw `ctypes.CDLL` |
| Function binding | `for _lib in _libs.values(): if not _lib.has("N","cdecl"): continue; N = _lib.get("N","cdecl")` (loop shape) | top-level `N = _libs['LIB']['N']` (CDLL `__getitem__` subscript) (`printer_python.py:197`)                                                                                                                                     |
| restype lines    | if/else `ReturnString`/`String`+errcheck block, or indented single line                                      | single top-level line, value `POINTER(c_char)`/`String`/`WideString`/other                                                                                                                                                     |
| Templates/loader | embedded inline                                                                                              | embedded inline by default; `--no-embed-templates` + `--linkage-anchor` (both mandatory together, `__main__.py:462-465`) generates a shared `_ctg_loader.py` sibling (`printer_python.py:149-162`)                             |

### 1.5 Verdict: ONE finalize_bindings.py cannot serve both forks

Current `codegen/finalize_bindings.py` (`crates/aphrodite-hermes/codegen/finalize_bindings.py`) is
written 100% against the upstream loop shape; every structural assumption fails on fork output:

- `_BLOCK_START_RE = ^for _lib in _libs\.values\(\):` (`finalize_bindings.py:88`) → **no match** on
  fork output → `ValueError("no ctypesgen declaration loops found")` at line 221-223 → exit 2 → build
  warns+skips (build.rs:194).
- `_LOAD_LINE_RE` matches `load_library(...)` (`finalize_bindings.py:93-95`); the fork emits
  `_get_library(...)` → `ValueError("no import-time library load line found")` at 232-233.
- `_LOOKUP_CALL_RE` matches `_lib.has/get(...)` (`finalize_bindings.py:92`); fork emits
  `_libs[L][N]` subscripting → `ValueError("no has/get lookup calls found")` at 242-243.
- `_RESTYPE_RE` (`finalize_bindings.py:101-112`) matches the upstream if/else ReturnString block or an
  **indented** single-line `String`/`ReturnString`; the fork emits a **top-level unindented**
  `POINTER(c_char)`/`String`/`WideString` - no match (the single-line alternative requires
  `^[ \t]+`), and `POINTER(c_char)`/`WideString` are not even in the alternation.
- `_ERRCHECK_RE` (122) is moot for the fork (no errcheck lines exist).
- `_DOCSTRING_RE` (129) misses the fork's `R"""Auto-generated by:` docstring - cosmetic only
  (the `sub(..., count=1)` no-ops), artifact stays byte-stable.

Also note the current committed artifact is upstream-shaped and produced by the upstream checkout:
`python3 -m ctypesgen` on this machine resolves to
`Module/ctypesgen/ctypesgen/__init__.py` (an editable/dev install; reports version 0.0.0), and the
committed `plugins/aphrodite/_bindings.py` carries the upstream preamble (`UserString` at 30-334),
the rewritten `for _lib ... hasattr` loops (889+), and `_libs["__APHRODITE_DYLIB__"] = None` (862).

**Recommendation (dev agent):**

1. **Prefer: switch the pipeline to the fork and rewrite finalize for fork shapes.** The fork's output
   is structurally simpler and produces a _better_ artifact for this plugin: no UserString/String
   preamble to ship (argtypes become `POINTER(c_char)`, which accepts `bytes` natively in ctypes -
   the plugin passes `.encode("utf-8")` bytes everywhere), no errcheck lines, no loader adapter class.
   The plugin's `bind_to(dylib)` design maps 1:1: neutralize `_libs["__APHRODITE_DYLIB__"] =
_get_library(...)` → `_libs[PLACEHOLDER] = _dylib`, rewrite `N = _libs['LIB']['N']` →
   `N = _dylib['N']` (plain CDLL `__getitem__`), rewrite top-level
   `N.restype = POINTER(c_char)|String|WideString` → `c_void_p`.
2. **Or: fork-detection in finalize** - branch on the presence of `_get_library` / `_libs[` subscript
   (fork) vs `for _lib in _libs.values():` / `load_library` (upstream). Cheap and deterministic, but
   two code paths to maintain. Given the committed artifact and the installed tool are upstream today,
   a `--generator {upstream|fork}` flag is cleaner than content sniffing.
3. Either way, invoke the fork **without** `--string-template` (default raw `POINTER(c_char)`
   shapes are exactly what the plugin wants - it reads raw pointers and frees them through the same
   handle, `plugins/aphrodite/__init__.py:636-664`). Do NOT pass `--guard-symbols` (a silently
   skipped declaration would dodge the finalize declared==header-set check).
4. The restype regex must add fork forms: `^(\s*)(\w+)\.restype = (POINTER\(c_char\)|String|WideString)$`.

## 2. Consumer surface: hermes-agent / prime-agent vs current FFI exports (b)

### 2.1 How the plugin consumes the FFI (the only bridge)

`plugins/aphrodite/__init__.py` `register()` (1106-1236) wires every export:

- `get_hooks()` → `ctx.register_hook(hook_name, _dispatch)` per name (1164-1202). Rust returns
  `["on_session_start","pre_tool_call","transform_tool_result","transform_terminal_output",
"pre_llm_call","post_llm_call"]` (`crates/aphrodite-hermes/src/lib.rs:498-511`) - all six are in
  Hermes' VALID_HOOKS (`hermes-agent/hermes_cli/plugins.py:108-118`), so `register_hook` never warns
  (unknown names warn at `plugins.py:917-920`, `_track_callback` 927-940).
- `get_schemas()` → per schema `ctx.register_tool(schema["name"], "aphrodite", schema,
_make_handler(name))` (1204-1215). Hermes signature: `register_tool(name, toolset, schema: dict,
handler, ...)` (`hermes-agent/hermes_cli/plugins.py:460-470`); schema goes straight to
  `tools/registry.py` `registry.register`.
- handler → `aphrodite_hermes_dispatch_tool(tool_name_bytes, args_json_bytes)` (667-687).
- hook callback → `aphrodite_hermes_call_hook(hook_name_bytes, kwargs_json_bytes)` with
  `json.dumps(kwargs, default=str)` (1167-1186).
- `materialize_directives(b"")` at registration (1155-1161); `version()` handshake (697-712);
  `_call_json` frees every returned pointer through the SAME dylib handle via
  `aphrodite_hermes_free_string` (636-664, F4: handle must travel with the pointer).
- `_REQUIRED_VOID_P` runtime assertion (56-64) + `_manual_ffi_setup` (458-477) + `_ensure_ffi_argtypes`
  (480-489) guard the pointer-width invariant on every load.

Hermes tool-result contract: `registry.dispatch` (`hermes-agent/tools/registry.py:874-891`) calls
`entry.handler(args, **kwargs)` and normalizes: **str results pass through** (only oversized
`"error"` JSON fields are trimmed by `_bound_json_error_result`, `registry.py:41-55`); dict results
are REJECTED unless `_multimodal` (`registry.py:862-868`) - so the plugin's `json.dumps(...)` string
return is required, and `{"error": ...}` failures currently surface to the model as _successful_
tool output (known limitation, not a surface gap).

### 2.2 Hook payload shapes Hermes actually sends (what call_hook must tolerate)

- `pre_llm_call` (`hermes-agent/agent/turn_context.py:677-689`): `session_id, task_id, turn_id,
user_message, conversation_history=list(messages)` - a list of **message objects, not
  JSON-serializable** → `default=str` stringifies them; `is_first_turn, model, platform,
parent_session_id, sender_id`. Rust reads only `result/output/content/tool_name/args`
  (`lib.rs:303-320`) - safe, but anything beyond those keys arrives lossy/stringified.
- `on_session_start` (`agent/conversation_loop.py:773-776`): `session_id, model, platform`.
- `transform_terminal_output` (`tools/terminal_tool_result.py:144`): `command, output`.
- `transform_tool_result` (`model_tools.py:843-859`): `tool_name, args, result, duration_ms` - the
  `args` dict can hold non-serializable values too.
- `pre_tool_call` blocking semantics: Hermes treats the hook as a **gate** - timeout/still-running
  fails closed (`hermes_cli/plugins_dispatch.py:42-48`, `_HOOK_TIMEOUT_FAIL_CLOSED_HOOKS =
{"pre_tool_call"}`). Rust returns `{"action":"modify","args":{"background":true,...}}` for
  auto-backgrounding of terminal/process (lib.rs:326-339). Any other return must be JSON-null to pass.

### 2.3 Gap analysis: exports vs real call patterns

Current surface: `dispatch_tool, call_hook, get_hooks, get_schemas, list_tools, get_schema, version,
materialize_directives, proxy_health, free_string` (lib.rs:235-297, 489-520).

- **Everything the plugin actually calls is covered.** Runtime call set: dispatch_tool, call_hook,
  get_hooks, get_schemas, materialize_directives, version, free_string. Nothing missing.
- **`proxy_health` is a dead export**: the plugin only sets its restype (`__init__.py:473`) and
  never calls it - health is probed over HTTP GET `/health` with a body check `{"status":"healthy"}`
  (`__init__.py:858-902`). The Rust `proxy_health` (lib.rs:175-189) is a bare TCP-connect probe and
  would report "alive" for any foreign service on the port - strictly weaker. Recommendation: drop it
  from `_REQUIRED_VOID_P`/`_manual_ffi_setup`/build.rs required list, or keep only as a diagnostics
  export; do not route the plugin's launch decision through it.
- **`list_tools`/`get_schema` are dead exports** for the plugin (it registers from `get_schemas`
  only, 1204-1215). Keep them (lib.rs has unit tests exercising them, 659-665) but they add surface
  the consumers never touch; the finalize `declared == header set` check must keep them in the header
  regex set.
- **Hook-arg expressiveness is the real gap**: Hermes sends rich per-hook kwargs (message objects,
  session/turn coordinates) that the plugin flattens with `default=str` before `call_hook`. The Rust
  side cannot consume structured data beyond the handful of string keys it reads (lib.rs:310-320).
  If future hooks need structured payloads (e.g. real conversation history for context injection),
  the FFI contract must grow a structured-args channel - today it is string-keyed only.
- **prime-agent is NOT an FFI consumer**: it is a TypeScript agent suite (packages/agent, ai,
  coding-agent, tui) with its own tool-call loop (`packages/agent/src/agent-loop.ts:349-364`,
  toolCall/toolResult message envelope) and `koffi` (Node FFI) only inside packages/tui
  (packages/tui/package.json:46). It contains zero references to aphrodite/dispatch_tool/call_hook.
  Treat it as a _payload-shape reference_ (Anthropic-style tool_call/tool_result envelopes), not a
  binding consumer.
- **No streaming/async dispatch**: `register_tool`'s `is_async` (plugins.py:461) is unused by the
  plugin; handlers are synchronous and return JSON strings. Acceptable today; flag if long tools
  arrive.
- **`_HEADER_FN_RE` only matches `char *`/`void` returns** (`finalize_bindings.py:71-74`): a future
  int-returning or struct-returning export would be silently dropped from `parse_header` and trip the
  `declared != header_names` violation - the header parser must grow before the ABI grows.

## 3. roto: hot-reload / crash-isolation prior art (c)

Findings (Module/roto):

- **roto has no dylib loading and no crash isolation.** Grep of `src/` and `tooling/` for
  `dlopen|dylib|dlclose|subprocess|isolate|spawn|SIGSEGV|segfault|hot` returns nothing relevant.
  Roto is a JIT-compiled scripting engine (cranelift) embedded in Rust processes.
- Hot-reload is a **language-design affordance, not implemented machinery**: README.md:56
  ("Roto scripts are **hot-reloadable**. The host application can recompile scripts at any time.")
  is realized by the host holding a `Roto` runtime struct (`src/runtime/mod.rs:151` `compile`,
  `164` `new`, `183` `from_lib`) and simply dropping/recompiling it - full state swap at the
  interpreter boundary. No incremental reload, no version pinning, no subprocess fence.
- The design constraints that make this safe: registered types must be `Clone`/`Copy` or wrapped in
  `Rc`/`Arc` (README.md:63-65) - i.e. **all host↔script state is swappable by construction**.

Verdict and recommendations for the Aphrodite plugin:

- **The plugin's `_probe_dylib` subprocess isolation has NO prior art in roto - it is the correct
  pattern for dylibs and should stay.** A dylib cannot be recompiled or cloned; the only way to
  confine a faulting image (SIGSEGV kills the whole gateway, `__init__.py:373-383`) is loading it in
  a fresh `sys.executable` child. The once-per-unique-path memoization (`_state.probed_paths`,
  384-386, 558-563) is the right latency tradeoff (no subprocess spawn per mtime check).
- **Adopt from roto the "state is swappable at every reload" contract**: the plugin already mirrors
  it - unique-path copy per generation (`_load_fresh_copy`, 290-322) because a repeat dlopen of the
  same path returns stale cached pages; mtime check + process-global holder (`_process_state`,
  109-143) so multiple shim execs converge on one handle; unlink-while-mapped cleanup of the previous
  generation (571-578) and atexit + startup reaping (243-... , 1077-1103). Keep all of it.
- **One gap roto's design highlights**: roto forces `Clone/Copy` types so hot-reload can never strand
  stale state. The plugin's reload _wipes_ Rust-side session state (CCR markers die - 538-551
  warning), i.e. the reload is a hard reset, not a swap. If preserving session state across reloads
  ever matters, the Rust side must externalize it (it already persists ccr.db) rather than relying on
  per-image OnceLock state. Documented, not a blocker.
- No changes required; roto contributes no pattern the plugin lacks.

## 4. FFI safety contracts from the books (d)

### 4.1 Allocator symmetry - free where allocated

rust-interop, `book/src/c/transferring-ownership.md`:

- 211-213: "We cannot reliably free C allocated memory from Rust, and vice versa. It has to be
  free'd / dropped where it was created. In practice this means that if you transfer ownership of
  heap allocated data across the language border, [you must provide the free function]."
- 272-273: `pub extern "C" fn csv_free_merged_file(merged: *mut c_char) { unsafe { CString::from_raw(merged) }; }`
- 281-282: "like `CString::into_raw()` transfers ownership away from an object, `CString::from_raw()`
  will reclaim it."

big-book-ffi, `reference/strings.md`:

- 64: "The usual rules of memory management with FFI apply: **memory must be released in the same
  language it was allocated**, and using borrowed data is easier."
- 85-89: "If you must store the string in foreign code, then you must pass the owned type
  `String`... you must ensure the pointer remains unique... and pass it back to Rust for
  destruction. If you allocate memory for the string in foreign code, then you must not run its
  destructor in Rust, and you must pass the string back to foreign code for destruction."
- 72-75 (String rebuild invariants, incl.): "the memory must have be[en] allocated by the same
  allocator the standard library uses, with a required alignment of exactly 1..."

**Application (verified against Aphrodite source):** the free_string round-trip already complies.
Rust allocates every return with `CString::new(...).into_raw()` (`lib.rs:211-213`, `to_c_string`),
and `aphrodite_hermes_free_string` reclaims with `CString::from_raw` (`lib.rs:269-275`) - the crate
states the rule at `lib.rs:669-672`: "Every string returned across the C ABI is allocated with
`CString::new(...).into_raw()` (`to_c_string`) and reclaimed by `CString::from_raw` inside
`aphrodite_hermes_free_string` - never via a manual `libc::free`/`free()` or an allocator-specific
dealloc." The Python side frees through the SAME dylib handle that produced the pointer (F4,
`__init__.py:636-664`) - the same-image requirement matters once a custom global allocator is added
(`__init__.py:652-656`). The Python 3.14 `c_char_p` malloc-mismatch SIGABRT is already defused by
using `c_void_p` restypes throughout (`__init__.py:466`, `587`).

### 4.2 CDLL / library lifecycle

- rust-ffi-guide, `book/dynamic_loading.md`: the only lifecycle machinery is plugin-traits + explicit
  `unload()` with `on_plugin_unload` callbacks (185-187, 321-329, 401-403); no reload semantics.
  Lesson for Aphrodite: an unload hook is the missing piece on the Rust side - the dylib has no
  teardown entry point, so reload relies on the plugin's copy/remove dance alone. Not required
  (all state is persisted), but a `_hermes_shutdown()` export would make atexit cleanup authoritative.
- big-book-ffi, `reference/functions.md:33`: "If you will call a function from foreign code by name
  then you must use `no_mangle`" - satisfied (`#[no_mangle]` on every export, lib.rs:235+).
- big-book-ffi, `reference/functions.md:88`: arg/return type agreement between declaration and
  definition - this is exactly what finalize's `argtypes count == header param count` check enforces
  (`finalize_bindings.py:303-310`).
- rust-interop, `book/src/c/cbindgen.md:10-33`: canonical cbindgen-as-build-dependency flow
  (`cbindgen = "0.24"` build-dep, `cbindgen::Builder::new().with_language(Language::C).generate()`)
    - mirrors `build.rs:85-104`.

## 5. rust-ffi-example: fixture baseline reality check (e)

- **The repo contains NO cbindgen and NO pointer-returning function anywhere.** Grep over all
  subdirs for `cbindgen` (build.rs/Cargo.toml) and for `*mut c_char|Box<|char *|char*` in *.rs:
  zero hits. There is no generated `.h` in the tree.
- `python-to-rust/` (the closest analog) is a minimal scalar example:
  `src/lib.rs:1-4` `#[no_mangle] pub extern fn double_input(input: i32) -> i32 { input * 2 }`;
  `src/main.py:1-25` uses `cdll.LoadLibrary('target/debug/libdouble_input.{dylib|so|dll}')` with a
  platform branch; `Makefile` builds then runs. No ctypesgen either.
- `rust-to-c/` is C→Rust (the `cc` crate compiles `src/double.c`, `build.rs`).
- **Verdict**: rust-ffi-example cannot serve as the pointer-returning expected-output fixture.
  The canonical pointer-returning header baseline is the Aphrodite pipeline's own
  `aphrodite_hermes.h` shape, documented at `finalize_bindings.py:66-70`:
  `char *aphrodite_hermes_dispatch_tool(char const *tool_name, char const *args_json);`
  (cbindgen renders `char *` with NO space before the name - the `\s*` in `_HEADER_FN_RE` exists for
  this). For a fixture file, regenerate `OUT_DIR/aphrodite_hermes.h` and commit a copy under
  `codegen/fixtures/` as the expected-input; the expected _output_ is the current committed
  `plugins/aphrodite/_bindings.py` (upstream shape) until the fork switch lands.

## 6. rust-bindgen test corpus: pointer-returning stress cases (f)

From `rust-bindgen/bindgen-tests/tests/headers/` (621 headers). Five concrete cases the Aphrodite
pipeline (cbindgen→header→ctypesgen→finalize) should be tested against:

1. **`func_ptr_return_type.h:1`** - `int (*func(void))(int, int);`
   A function returning a **function pointer**. Stress: ctypesgen must parse a function-pointer
   restype; the fork's printer emits `CFUNCTYPE(...)` restypes (`ctypedescs.py:274-278`); finalize's
   `_RESTYPE_RE`/`_pointer_width` have no CFUNCTYPE handling → would need a decision (rewrite to
   c_void_p, or reject). `_HEADER_FN_RE` (finalize:71) doesn't match `int (*...)` returns at all.
2. **`issue-833.hpp:9`** - `extern "C" nsTArray<int>* func();`
   Opaque template-instantiated struct pointer return with `--generate functions
--allowlist-function func --raw-line` flags. Stress: opaque struct → ctypesgen emits
   `POINTER(nsTArray_int)` with an opaque struct definition; `_pointer_width` accepts it
   (issubclass of `ctypes._Pointer`, finalize:197) but the header regexes need the `nsTArray<int>*`
   shape added.
3. **`issue-1118-using-forward-decl.hpp:10`** - `nsTArray<nsIContent*> *Gecko_GetAnonymousContentForElement();`
   Pointer-in-template-argument **plus** pointer return (nested indirection inside the type). Stress:
   ctypesgen type-name mangling (`nsTArray_nsIContentPtr`) and the header parser seeing
   `* >*` punctuation.
4. **`issue-848-replacement-system-include.hpp:6`** - `nsTArray<int>* func();`
   Same opaque-template family with `--system-include` replacement flags; cheapest regression case
   for the parser.
5. **`vtable_recursive_sig.hpp:6-10`** - `virtual Derived* AsDerived();` (recursive self-pointer in
   a C++ class). **Negative case**: cbindgen never emits C++ virtuals, so the pipeline must never see
   this; feed it to ctypesgen and it must fail cleanly (C-only parser), not hang or emit garbage.

Bonus relevant cases: `issue-2966.h:1-3` (`typedef const char *pub_var1; ...`) - const-char-pointer
typedefs, exactly the family upstream maps to `c_char_p` (the branch the fork dropped); and the
fork's `va_list -> c_void_p` map (`ctypedescs.py:72-74`) should be exercised with
`va_list_aarch64_linux.h`.

---

## Recommendations (rollup for the dev agent)

1. **Fork switch is net-positive**: fork output (top-level declarations, raw `POINTER(c_char)`
   restypes, no errcheck, no preamble, `_get_library`) maps cleanly onto the plugin's `bind_to()`
   design and drops the embedded String-class baggage. Rewrite finalize's regexes for fork shapes
   (see §1.5) OR add fork-detection; a `--generator` flag is cleaner than content sniffing.
2. Invoke the fork with no `--string-template`, no `--guard-symbols`; keep `-l __APHRODITE_DYLIB__`.
3. Consumer surface is complete for what the plugin calls; drop or quarantine `proxy_health`
   (HTTP probe is stricter), keep `list_tools`/`get_schema` for tests, and grow the header parser
   before the ABI grows beyond `char *`/`void` returns.
4. Hook kwargs are flattened to strings by `default=str` - keep Rust's keyed reads; document that
   `pre_llm_call conversation_history` arrives stringified.
5. Keep the `_probe_dylib` subprocess fence and the unique-path copy/reload machinery exactly as-is
   (roto offers no competing pattern); consider a Rust teardown export to make unload authoritative.
6. Allocator symmetry is already compliant (CString into_raw/from_raw pair, same-handle free, F4);
   the books' rules are the regression criteria for the free_string round-trip test.
7. Fixture baseline: commit `OUT_DIR/aphrodite_hermes.h` as `codegen/fixtures/expected.h`; do not
   look to rust-ffi-example (scalar-only, no cbindgen).
8. Add the five rust-bindgen headers (§6) as pipeline stress fixtures after the fork switch.
