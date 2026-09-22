# Dylib Loading (load-once per process)

The Python shim loads the Rust dylib exactly once per process from the
resolved path (`ctypes.CDLL` on the canonical candidate). There is **no
hot-reload**: `dlopen` memoizes loaded images by canonical path, and the
plugin does not work around that with fresh copies. To pick up a newly
built dylib, **restart the Hermes session** - a new process loads the new
image; a running process keeps the image it loaded at startup.

## Load flow

```mermaid
flowchart TD
	A["any tool/hook call → _load_dylib()"] --> B["pick first existing candidate:<br/>APHRODITE_HERMES_DYLIB_PATH → shipped plugin-dir binaries/<br/>(immutable release assets) → canonical runtime home<br/>($HERMES_HOME/aphrodite/binaries) → legacy plugin-dir copy<br/>→ monorepo target/release"]
	B --> B1["_ensure_binaries(): presence-only check - NEVER downloads.<br/>Missing binaries → log the explicit setup step<br/>(bash download.sh / aphrodite setup); register() has no download path.<br/>APHRODITE_AUTO_DOWNLOAD=1 restores the legacy fetch for dev loops"]
	B1 --> C{"_state.dylib already set?"}
	C -->|yes| E["return cached handle - load-once-per-process<br/>(a rebuilt dylib is picked up only after a restart)"]
	C -->|no| H["first load"]
	H --> P["_probe_dylib(path) - smoke-test in a SUBPROCESS<br/>(once per unique path; a ctypes SIGSEGV cannot be caught by<br/>try/except and would kill the whole Hermes gateway)"]
	P -->|fail| PF["RuntimeError - plugin disabled, graceful log, never SIGSEGV"]
	P -->|ok| K["ctypes.CDLL(resolved_path) - direct load, no copies"]
	K --> L["_configure_ffi: generated _bindings.py bind_to(dylib) →<br/>manual fallback → _REQUIRED_VOID_P assertion (7 exports)<br/>(restype=c_void_p - prevents 64-bit pointer truncation)"]
	L --> N{"AttributeError (missing symbol)?"}
	N -->|yes| O["RuntimeError naming path+symbol - stale/mismatched dylib"]
	N -->|no| P2["_state.dylib set; return"]

	E --> Q["_call_json → FFI"]
```

## Notes

- `_state` is a process-global holder (a module registered in `sys.modules`).
  Hermes builds one plugin manager per home and executes the shim once per
  home under a distinct module name; without the holder, the second load
  would call `CDLL` again on the same canonical path and reuse the mapped
  image (or fail on Windows where a second open of a loaded library is
  denied). The early-return makes later shim copies reuse the mapped handle.
- `_dylib_lock` (a `threading.Lock`) guards the first-load window because
  ctypes releases the GIL during foreign calls - two Hermes threads could
  otherwise race through `_load_dylib`.
- Hooks and tools call `_load_dylib()` fresh inside each closure (not a
  registration-time handle); the holder early-return makes that cheap.
- The subprocess probe runs once per unique path per process - spawning a
  subprocess on every call would add latency to every hook/tool call.
- No `hotreload/` directory is ever created; no copies are written anywhere
  in the runtime home.

## State handling across processes

```mermaid
stateDiagram-v2
	[*] --> ImageA: CDLL load (startup, gen 0)
	ImageA --> ImageA: calls mutate AphroditeState<br/>(bridge OnceLock / core HANDLES)
	ImageA --> [*]: process exits
	[*] --> ImageB: next Hermes session loads the new dylib
	note right of ImageB
		ImageB starts with DEFAULT state
		(re-reads aphrodite.toml at dylib init).
		ImageA's inline_store + markers are GONE:
		old markers unresolvable against the new image.
	end note
	ImageB --> [*]
```
