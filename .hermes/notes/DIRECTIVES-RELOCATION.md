# DIRECTIVES-RELOCATION: binary-provided directives

**Status:** mechanism built (embed + FFI materialize), Python-shim wiring is a
**later wave**. Plugin dir must end up a pure loader; the binary is the
provider of the directives set; the runtime home stores them.

## The problem

`plugins/aphrodite/directives/` (submodule, 5 files) is the current source of
truth, but the plugin dir is a replaceable loader - any runtime content it
holds is lost or duplicated when the plugin updates. The set must ship WITH
the binary (release dylib / crates.io build) and live at runtime in the user
data folder `~/.hermes/aphrodite/directives/`.

## New flow

1. **Embed** - the directive set is compiled into the binary:
   `crates/aphrodite/src/builtin_directives/*.md` (7 files: `ccr-handling`,
   `cleanup`, `explore`, `focus`, `foresight`, `lazy`, `lazy-eval`), pulled in
   by `include_str!` in `crates/aphrodite/src/directives.rs`
   (`builtin_directives()`, exposed via the public
   `aphrodite::directives::loaded_builtins()`). `lazy-eval` was added to the
   embed set from the plugin's `lazy-eval.md` (it is distinct from the builtin
   `lazy.md`); `lazy.md` stays as-is.
2. **Provision** - the dylib materializes the embedded set into the runtime
   home at startup/setup time:
   `crates/aphrodite-hermes/src/directives.rs` +
   `aphrodite_hermes_materialize_directives(home_dir)` in `lib.rs`.
   Idempotent, non-destructive: missing → write; byte-identical → skip;
   differs → skip + warning (**never** overwrites user data). Always returns
   `status:"ok"` JSON; failures degrade to warnings; never panics (`guarded()`).
3. **Read** - the core directive loader (`config_loader.rs`) already reads
   `~/.hermes/aphrodite/directives/` as its home-namespace candidate, so
   provision and load agree with no plugin-dir involvement.

### FFI + JSON contract

```
pub extern "C" fn aphrodite_hermes_materialize_directives(home_dir: *const c_char) -> *mut c_char
```

Free the returned string with `aphrodite_hermes_free_string`.
Response: `{"status":"ok","dir":"<target>","written":["..."],"skipped":["..."],"warnings":["..."]}`

- `written`/`skipped` sorted; `skipped` = filenames, `warnings` carry the
  reason (e.g. `"directives/focus.md exists with different content; leaving
user-modified file as-is (not overwritten)"`).

### Home resolution precedence (materialize target)

1. `home_dir` FFI arg (non-empty) → `<arg>/directives/`
2. `$APHRODITE_DIRECTIVES_DIR` (non-empty) → used as the exact directives dir
   (this is the loader's own candidate-0 override - materialize where it reads)
3. `$APHRODITE_HOME` (non-empty) → `<value>/directives/`
4. `$HOME/.hermes/aphrodite` → `<HOME>/.hermes/aphrodite/directives/`
5. `.` + warning (degraded, never fails)

Env-var choice: added **`APHRODITE_HOME`** as the home-level override.
`APHRODITE_CONFIG_PATH` was NOT reused: it names a _file_ (the toml), not the
home dir, so deriving a directory from it would be ambiguous.
`APHRODITE_DIRECTIVES_DIR` already existed and is honored because it is the
loader's exact-dir override (candidate 0) - the materialize target must match
where the loader reads.

## **init**.py wiring recommendation (later wave - do not edit now)

Call site: **after the dylib handle exists**, i.e. right after the first
`dylib = _load_dylib()` in the registration/session-start path
(`plugins/aphrodite/__init__.py`, `_load_dylib()` defined ~line 290; the
directives-discovery env block is ~lines 39-44). Recommended snippet:

```python
# Binary-provided directives: materialize the embedded set into the
# runtime home (~/.hermes/aphrodite/directives). Idempotent; never
# overwrites user-modified files. Resolve failures degrade to warnings.
try:
    _raw = dylib.aphrodite_hermes_materialize_directives(None)  # or str(home)
    _report = json.loads(ctypes.string_at(_raw).decode("utf-8", "replace"))
    dylib.aphrodite_hermes_free_string(_raw)
    for w in _report.get("warnings", []):
        _log.warning("aphrodite: %s", w)
except Exception as e:  # defensive: never abort registration
    _log.warning("aphrodite: directives materialization failed: %s", e)
```

**Also required (same wave):** drop/repurpose the `APHRODITE_DIRECTIVES_DIR`
export at lines 39-44 (`os.environ.setdefault("APHRODITE_DIRECTIVES_DIR",
str(_PLUGIN_DIR / "directives"))`). While it points at the plugin dir, the
loader's candidate 0 keeps reading the plugin copy and - because materialize
honors candidate 0 - materialize would write into the plugin dir too. Once the
plugin dir set is gone, the loader falls through to the home candidate and
reads the materialized set; no env export is needed at all.

## setup.rs wiring TODO (later pass - owned by another pair, do NOT edit)

`crates/aphrodite/src/setup.rs` (`aphrodite setup` CLI flow) should also
materialize the builtins into `~/.hermes/aphrodite/directives/` so
`cargo install → aphrodite setup` and `download.sh → aphrodite setup`
provision the runtime home without a Hermes session. Exact call site to be
decided by the setup.rs owner; suggested anchor: after config/state init in
the setup flow, write `aphrodite::directives::loaded_builtins()` entries into
`~/.hermes/aphrodite/directives/` with the same write/skip/never-overwrite
semantics (or shell out to the same dylib FFI).

## Later-wave TODO (owned by other pairs)

- Remove `plugins/aphrodite/directives/` from the plugin submodule (deletion
  only after the shim wiring + setup wiring land, so no gap).
- Update `plugins/aphrodite/plugin.yaml`, `plugins/aphrodite/README.md`, and
  download scripts to stop referencing the plugin-dir directives set.
- Consider documenting `APHRODITE_HOME` in `docs/config/env-vars.md` (owned by
  the docs pair) - the established `~/.hermes/aphrodite` home default is
  already documented there.

## Caveats

- Materialize writes `loaded_builtins()` content, which is capped at
  `MAX_DIRECTIVE_CHARS` (2000) with an ellipsis. All 7 shipped files are
  well under the cap today, so materialized bytes match the embedded sources
  verbatim; if a future directive exceeds the cap, the materialized copy
  would be the truncated form - keep shipped directives under 2000 chars.
- `crates/aphrodite/src/builtin_directives/lazy-eval.md` was copied from the
  plugin's `lazy-eval.md` (byte-identical). If the plugin copy is ever
  edited, the embed copy must be updated to match - the embed copy is the
  shipped artifact now.

## Tests

- Rust: `cargo test -p aphrodite-hermes directives` (module tests: files
  land, idempotent re-run, user-modified file NOT overwritten, resolution
  precedence, FFI round-trip) + `cargo test -p aphrodite builtins` for the
  core 7-builtins test.
- Python: `python3 tests/test_directives_materialize.py` (self-contained,
  no pytest; skips gracefully with a warning if the dylib is missing/stale -
  run `cargo build -p aphrodite-hermes` first).
