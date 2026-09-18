# Aphrodite Architecture (current state)

Date: 2026-09-18 (EEST) - binary **1.4.6** / plugin **2.1.4** / branch
`Development`. Every claim below is verified against the live source; the
full flow traces live in `../uml/` (01-11 + README).

## 1. Two runtime worlds, one shared core

- **The proxy binary** (`crates/aphrodite`): dual loopback listeners
  (`:9797` cache mode, `:9798` token mode), HTTP response compression,
  `proxy_detect_content_type` + `proxy_build_preview`, its own marker layout
  (`proxy_format_ccr_output` puts the marker last).
- **The Hermes plugin** (`plugins/aphrodite` + `crates/aphrodite-hermes`):
  Python ctypes shim -> C-ABI dylib -> shared core crate. Uses
  `build_preview` + `render_marker` (marker first).

The two pipelines share `compute_key` (BLAKE3) + the marker wire format
(`<<<CCR:hash|type|size>>>`) but otherwise diverge - parity (WS3) is a known
1.5.0 item.

## 2. The FFI codegen pipeline

```
cbindgen 0.29 -> aphrodite_hermes.h
  -> ctypesgen (upstream 2.7.4-27202 installed)
  -> crates/aphrodite-hermes/codegen/finalize_bindings.py (409-line AST finalizer)
  -> committed plugins/aphrodite/_bindings.py (copy-on-change, byte-stable, 135 lines)
```

- `finalize_bindings.py`: AST-based `validate()` (no exec), strips the
  ctypesgen preamble/dead code, `__all__ = ["bind_to"]`, rewrites restypes to
  `c_void_p`, argtypes to `c_char_p` (dispatch/call_hook) + `c_void_p`
  (`free_string` - the runtime fix that prevented the SIGSEGV), fork
  detection (`classify_shape`: upstream-loop / fork-flat / unknown; non-upstream
  -> `CTYPESGEN_FORK_WARNING` + graceful skip).
- `crates/aphrodite-hermes/build.rs`: ctypesgen variant probe + `cargo:warning`
  relay; missing tools -> warn + skip (committed artifact stays in effect);
  contract violation with tools present -> build panic (by design).
- Static checker: `Maintain/check_ffi_contract.py` (AST-based, exit 0/1/2,
  self-tests 13/13, 50 asserts), CI workflow `.github/workflows/ffi-check.yml`
  (trigger paths cover build.rs, cbindgen.toml, codegen/**, aphrodite-hermes/**,
  plugins/aphrodite/**), package.json `Check:FFI` script.

## 3. The plugin is a pure loader

- `plugins/aphrodite/__init__.py` (mirrored byte-identical at
  `crates/aphrodite/templates/__init__.py`, drift-guarded): thin ctypes shim -
  `_load_dylib` (probe + mtime hot-reload), `_call_json` (forced
  `restype=c_void_p`, same-handle free), `_REQUIRED_VOID_P` assertion,
  `_probe_dylib` subprocess sentinel (a faulting image degrades to graceful
  "plugin disabled", never kills the gateway), `_read_str` NULL guard,
  `_check_version_published` warn, per-hook/per-tool try/except registration.
- FFI setup: generated `_bindings.py` `bind_to(dylib)` -> manual
  `_manual_ffi_setup` fallback -> `_REQUIRED_VOID_P` assertion.
- Skills are dev-side (`.hermes/skills/`, never shipped); the plugin registers
  tools and hooks only. `aphrodite_hermes_list_skills` was removed.

## 4. Runtime home + layout self-heal

- Canonical runtime home: `~/.hermes/aphrodite/` - `aphrodite.toml`,
  `binaries/`, `directives/`, `hotreload/`.
- `plugins/aphrodite/layout_check.py` + `layout_schema.json` self-heal the
  layout at startup: create missing dirs, repair symlinks, relocate displaced
  files (byte-verified, never overwrites user data), idempotent, never raises.
- Directives are embedded in the binary (`crates/aphrodite/src/builtin_directives/`,
  7 files incl. lazy-eval.md), materialized by
  `aphrodite_hermes_materialize_directives` into `~/.hermes/aphrodite/directives/`
  at registration - idempotent, never overwrites user-modified files.
- Hot-reload: mtime detection -> subprocess probe -> unique-path copy into
  `~/.hermes/aphrodite/hotreload/` -> `CDLL` reload -> per-image state reset;
  `_reap_stale_hotreloads` cleans dead-PID copies.

## 5. Preview system (post-Issue #11 WS1/2/4)

- `build_preview` (`crates/aphrodite/src/preview.rs`) is the shared builder
  for every hook/FFI/C-ABI/bridge path; `proxy_build_preview` is the proxy's
  parallel builder (WS3 parity deferred).
- WS4 wired `[previews] preview_max_chars` end-to-end: env
  `APHRODITE_PREVIEW_MAX_CHARS` > TOML > default 120, enforced at the single
  choke point (end of `build_preview`), char-boundary-safe truncation that
  preserves the closing `]` + `…`; applied at startup (`main.rs`), on
  `/reload` (`proxy.rs`), in dylib init (`crates/aphrodite-hermes/src/lib.rs`),
  and by `config_loader::apply_previews`.
- `tools::unwrap_hermes_result` (`crates/aphrodite-hermes/src/tools.rs`): the
  Hermes-envelope heuristic; WS1 removed the `ok` collapse, guarded the
  success-string + priority-key arms (single-key only), and made the caller
  hint win over the unwrap on the explicit compress path (full-content preview
  for JSON payloads).
- Still-inert knobs (out of scope): `model_family`, `code_structure_map`,
  `rust_preview_lines`.

## 6. Removed in the 1.4.6 cycle

Shipped skills (dev-side only), `aphrodite_hermes_list_skills` FFI, S2
navigation (`s2-probe`/`s2-navigate` crates, `aphrodite_navigate` tool,
`navigation` feature), install scripts (`Maintain/install.sh/.ps1/.bat`),
`profiles/`, `.githooks/`.

## 7. Where the details live

- Flow traces: `../uml/` (01 startup, 02 chat compression, 03 retrieve,
  04 hook-ffi, 05 ccr-lifecycle, 06 sse, 07 config resolution, 08 hotreload,
  09 release-ci, 10 component, 11 data-model).
- FFI pipeline implementation + research: `../notes/ffi/`.
- Plugin loader/directives/forensics: `../notes/plugin/`.
- Issue #11 preview system: `../notes/issue11/`.
