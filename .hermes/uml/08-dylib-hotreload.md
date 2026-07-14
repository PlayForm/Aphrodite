# 08 - Dylib Hot-Reload

The Python shim reloads the Rust dylib when its mtime changes, working around
the fact that `dlopen` memoizes loaded images by canonical path. Each generation
is copied to a unique temp path before `ctypes.CDLL()`, so genuinely new code is
loaded. A reload wipes all Rust-side session state (fresh `OnceLock`/`HANDLES`).

## Hot-reload flow

```mermaid
flowchart TD
    A["any tool/hook call → _load_dylib() (__init__.py:74)"] --> B["pick first existing candidate path<br/>(env / binaries/ / monorepo target/release fallbacks)"]
    B --> C["current_mtime = os.path.getmtime(path)"]
    C --> D{"_dylib set AND current_mtime == _dylib_mtime?"}
    D -->|yes| E["return cached _dylib - no reload"]
    D -->|no| F{"_dylib already existed?"}
    F -->|yes| G["_log.warning: reload resets ALL session CCR state<br/>(every &lt;&lt;&lt;CCR:…&gt;&gt;&gt; marker becomes unresolvable)"]
    F -->|no| H["first load"]
    G --> I
    H --> I["_load_fresh_copy(path) (__init__.py:49)"]
    I --> J["copy → &lt;dir&gt;/.hotreload/&lt;name&gt;.&lt;pid&gt;.&lt;gen&gt;<br/>(unique path sidesteps dlopen path-cache)"]
    J --> K["ctypes.CDLL(load_path)"]
    K --> L["set argtypes/restype (restype=c_void_p - avoids 3.14 c_char_p SIGABRT)"]
    L --> M["unlink PREVIOUS generation's copy (POSIX unlink-while-mapped safe)"]
    M --> N{"AttributeError (missing symbol)?"}
    N -->|yes| O["RuntimeError naming path+symbol - stale/mismatched dylib"]
    N -->|no| P["_dylib, _dylib_mtime, _dylib_copy_path updated; return"]

    E --> Q["_call_json → FFI"]
    P --> Q
```

## State handling across images

```mermaid
stateDiagram-v2
    [*] --> ImageA: first CDLL load (gen 0)
    ImageA --> ImageA: calls mutate AphroditeState<br/>(bridge OnceLock / core HANDLES)
    ImageA --> Reload: dylib rebuilt on disk (mtime changes)
    Reload --> ImageB: fresh copy .hotreload/…gen1 → CDLL
    note right of ImageB
      ImageB starts with DEFAULT state
      (re-reads aphrodite.toml via shared()/aphrodite_init).
      ImageA's inline_store + markers are GONE:
      old markers unresolvable against new image.
      Old copy file unlinked; still-mapped pages stay valid until unload.
    end note
    ImageB --> [*]
```

Concurrency: `_dylib_lock` (a `threading.Lock`) guards the reload window because
ctypes releases the GIL during foreign calls - two Hermes threads could
otherwise race through `_load_dylib`. Hooks and tools call `_load_dylib()`
**fresh inside each closure** (not the registration-time handle), so a
hot-reloaded image is picked up on the very next call.

## Key call sites
- `_load_dylib` (mtime check, warning, reload) - `crates/aphrodite/templates/__init__.py:74`
- `_load_fresh_copy` (unique temp-path copy) - `crates/aphrodite/templates/__init__.py:49`
- `_dylib_lock` + module globals - `crates/aphrodite/templates/__init__.py:38`
- `_call_json` (FFI + same-dylib free) - `crates/aphrodite/templates/__init__.py:172`
- shim mirror (submodule) - `plugins/aphrodite/__init__.py` (byte-identical)
- Rust-side state homes: `aphrodite-hermes/src/lib.rs:60` (`STATE` OnceLock), `crates/aphrodite/src/lib.rs:73` (`HANDLES`)
