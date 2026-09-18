# plugin/ - Plugin Loader, Layout, Directives & Failure Forensics

The plugin as a pure loader: layout self-heal, binary-provided directives,
and the forensics of the 2026-09-17 plugin failure that started the SIGSEGV
hardening work. Root entry point: `../ARCHITECTURE.md`.

| File                                                                     | One-line description                                                                                                                                         |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [DIRECTIVES-RELOCATION.md](DIRECTIVES-RELOCATION.md)                     | Binary-provided directives: embedded `builtin_directives/` set, FFI materialize into the runtime home, resolution precedence, Python wiring as a later wave. |
| [LAYOUT-SELF-HEAL.md](LAYOUT-SELF-HEAL.md)                               | `layout_check.py` + `layout_schema.json`: self-heal of the runtime home layout at startup (idempotent, non-destructive, report-only on failure).             |
| [HERMES-UPDATE-ERROR-FORENSICS.md](HERMES-UPDATE-ERROR-FORENSICS.md)     | Log-side forensics of the 2026-09-17 20:33 plugin failure: the 404 + dylib-not-found ERROR and the SIGSEGV root cause.                                       |
| [HERMES-UPDATE-ERROR-PLUGIN-SIDE.md](HERMES-UPDATE-ERROR-PLUGIN-SIDE.md) | Plugin-side root-cause investigation: startup/registration/version-handshake paths, 3x-load analysis, recovery checklist.                                    |
| [DOCKET-HERMES-UPDATE-ERROR.md](DOCKET-HERMES-UPDATE-ERROR.md)           | Open docket for the two-layer failure (SIGSEGV trigger + self-disabled auto-fetch) with the fixes on the docket.                                             |

The SIGSEGV fix + FFI hardening that came out of these forensics are recorded
in `../ffi/DEVELOP.md`.
