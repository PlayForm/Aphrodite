---
name: aphrodite-testing-discipline
description: "Use when probing, testing, or verifying the Aphrodite plugin or dylib: always exercise the ACTUAL plugin source code, never hand-rolled ctypes probes; always test with real code paths; keep all scratch under .hermes/tmp."
version: 1.0.0
author: Aphrodite dev flow
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [aphrodite, testing, ffi, ctypes, probes, discipline]
        related_skills: [aphrodite-release-flow, aphrodite-operations]
---

# Aphrodite Testing Discipline

Hard rules for probing/testing/verifying the Aphrodite plugin and dylib.
Violating these caused real SIGSEGV crash dialogs and wasted sessions.

## Rule 1 - Always exercise the ACTUAL plugin source

Never hand-roll a raw `ctypes.CDLL` probe against the dylib. The raw-ctypes
path defaults `restype` to `c_int`, truncates 64-bit pointers to 32 bits,
and SIGSEGVs inside `strlen` - the exact bug class the FFI pipeline was
built to eliminate. Two separate probe scripts crashed the machine this way
(23:57 and 00:04, same `_platform_strlen <- string_at` signature).

Correct pattern - import the real plugin and drive its own functions:

```python
import sys, importlib.util
sys.path.insert(0, "plugins/aphrodite")
spec = importlib.util.spec_from_file_location("aphrodite_pkg", "plugins/aphrodite/__init__.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
dylib = mod._load_dylib()                    # real loader: hot-reload copy + mtime
# _configure_ffi is applied by _load_dylib: generated _bindings.py restype/argtypes
result = mod._call_json(dylib, "aphrodite_hermes_version")  # forces c_void_p, frees same handle
```

`_load_dylib` applies the generated `_bindings.py` (restype `c_void_p`,
argtypes `[c_char_p, c_char_p]`) and `_call_json` forces `c_void_p` and
frees through the same handle (F4). Everything the plugin itself does, you
inherit for free.

If you MUST use raw ctypes (never for the dylib - only for unrelated
libraries): set `restype = ctypes.c_void_p` AND `argtypes` BEFORE every
call, and free via the same handle that allocated.

## Rule 2 - Always test with REAL code

- Run the REAL dylib from `target/release/` (rebuild first:
  `cargo build --release -p aphrodite -p aphrodite-hermes`), not a stale
  copy and not the debug build.
- Verify the version handshake before trusting results:
  `mod._call_json(dylib, "aphrodite_hermes_version")` must equal
  `plugins/aphrodite/BINARY_VERSION` (currently 1.4.6).
- The plugin's own test suites are the ground truth:
    - `cargo test -p aphrodite` (lib + all bins)
    - `cargo test -p aphrodite-hermes`
    - `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`
    - `python3 Maintain/tests/test_check_ffi_contract.py`
    - `python3 Maintain/check_ffi_contract.py` -> PASS (0 violations)
    - repro: `python3 <scratch>/repro.py` -> SURVIVED
    - drift-guard: `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`
- Run them, record the ACTUAL numbers. Never report "should pass" - report
  what the command printed.
- A battery/probe harness must be a real executable script whose output IS
  the evidence (rows classified, exit codes), never a report generator that
  fabricates the table from memory.

## Rule 3 - Scratch lives in .hermes/tmp, NOT /tmp

All probe scripts, generated fixtures, research dumps, and intermediate
artifacts go in `.hermes/tmp/` (repo-local, gitignored contents, tracked
`.gitkeep`). Never write scratch to `/tmp` - it is cleaned by the OS,
invisible to the repo, and scattered artifacts there caused the "where is
the code?" confusion. The canonical scratch for session coordination
remains the sigserve dir under the Temporary tree.

## Rule 4 - Verify through the path the USER sees

The preview the model sees must be the FINAL, honest representation. When
verifying preview behavior, exercise both:

1. the direct path (`_call_json` -> `aphrodite_hermes_dispatch_tool` with
   the tool payload), and
2. the hook path (the plugin's `register()`-registered hook wrappers -
   `transform_tool_result` etc. - what the LLM actually reads).
   The hook path is the production path; a fix that only works direct is not
   done.

## Pitfalls

- Two raw-ctypes probe scripts (Sep 17 23:57, Sep 18 00:04) SIGSEGV'd:
  `string_at` -> `_platform_strlen` on a truncated pointer. Same signature:
  `EXC_BAD_ACCESS KERN_INVALID_ADDRESS` on a sign-extended low-32-bit value.
  The plugin path never crashed - the probes did.
- A "generated" battery report with no runner script is fabrication. The
  evidence is the runner's exit codes and row table.
- The battery threshold changed between versions (terminal compression
  threshold was 256B on 1.4.5, 512B on 1.4.6) - re-derive empirically, do
  not assume the old fixture sizes still compress.
- Tests that mutate process-global state (e.g. the preview_max_chars cap)
  must take the module's shared test guard, or parallel cargo test runs
  race and fail intermittently - the cap_guard lesson from preview.rs.

## Verification Checklist

- [ ] Dylib loaded via the plugin's `_load_dylib()` / `_call_json()`, zero
      raw `ctypes.CDLL` in the probe
- [ ] Version handshake: dylib version == BINARY_VERSION
- [ ] Release dylib rebuilt, not stale
- [ ] Real test suites run and actual numbers recorded
- [ ] Scratch in `.hermes/tmp/` (or the sigserve scratch dir), never `/tmp`
- [ ] No crash dialogs, repro SURVIVED, zero new SIGSEGV
