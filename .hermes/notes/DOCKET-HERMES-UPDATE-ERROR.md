# Docket - Hermes Update Plugin Failure (2026-09-17 20:33)

**Status:** OPEN - two-layer issue, plugin currently disabled on the default
gateway.

## Summary

`hermes update` (20:24, already at v0.21.3) triggered a fleet restart. The
Aphrodite plugin then failed with:

```
ERROR aphrodite: aphrodite-hermes dylib could not be loaded (Dylib not found.
Tried: [...6 paths...]) - plugin disabled; run download.sh ...
```

Verbatim at `~/.hermes/logs/errors.log:6148` (20:34:46), preceded by
`download.sh` curl 404 for
`Aphrodite/v1.4.6/aphrodite-aarch64-apple-darwin`.

## Root causes (two layers, both real)

### Layer 1 - native SIGSEGV in the 1.4.6 dylib (the trigger)

- `libaphrodite_hermes.dylib` v1.4.6 **deterministically SIGSEGVs the gateway
  ~2s after load** - 41 identical crash reports (`~/Library/Logs/DiagnosticReports`,
  19:27-20:33), e.g. `EXC_BAD_ACCESS KERN_INVALID_ADDRESS` faulting
  `_platform_strlen` via `z_get` during `PyObject_GenericGetAttr`/`builtin_exec`.
- Same signature in pre-update crashes (20:08-20:18) → corruption predates the
  update; the restart loop amplified it. Registration stops right after the
  layout warnings (no `registered N hooks/tools` lines) - gateway child dies.
- Not proven to be the dylib itself (stack is deep in CPython); needs ASAN/bisect
  repro (1.4.5 vs 1.4.6, plugin on vs off). Suspects: `materialize_directives`
  FFI (new), `free_string` across hot-reload images (documented F4 UB).

### Layer 2 - self-disabled + unrecoverable auto-fetch (the visible error)

- ~20:34 the plugin was added to `plugins.disabled` (config.yaml:566), the
  `~/.hermes/plugins/aphrodite` symlink was **removed**, and
  `~/.hermes/aphrodite` was **wiped** (binaries/ + hotreload/ now empty).
- `_load_dylib` found nothing → `_ensure_binaries` → `download.sh` → **404**:
  release `Aphrodite/v1.4.6` does not exist (latest published is v1.4.5), so
  BINARY_VERSION=1.4.6 can never auto-heal. Deterministic dead end.

## Verified good / not the cause

- Version handshake 1.4.6 vs 1.4.6 **passes silently** (`_check_version` warn-only).
- `allow_tool_override` legacy key → decision=allow, informational only.
- Storage/retrieval lossless; layout self-heal non-destructive to runtime home.
- Update itself exit 0; no Python traceback exists (segfault, not exception).

## Recovery checklist (in order)

1. **Publish release `Aphrodite/v1.4.6`** (Build.yml, 12 assets incl.
   `aphrodite-aarch64-apple-darwin` + `SHA256SUMS`) - or pin BINARY_VERSION to 1.4.5
   until the crash is fixed. Without a real release, auto-fetch can never recover.
2. **Re-create the plugin symlink**: `ln -s <repo>/plugins/aphrodite
~/.hermes/plugins/aphrodite`.
3. **Remove `aphrodite` from `plugins.disabled`** in `~/.hermes/config.yaml:566`.
4. **Restore binaries** to `~/.hermes/aphrodite/binaries/` (copy the verified
   target/release build or let download.sh fetch once v1.4.6 exists).
5. **Confirm registration completes**: watch for `registered N hooks`,
   `registered N tools` INFO lines (absent = registration stopped again).

## Fixes on the docket

- **A1 (crash):** bisect 1.4.5 vs 1.4.6 dylib, ASAN repro, audit
  `materialize_directives` FFI + cross-image `free_string` (F4). Suspect:
  startup-path FFI call. Do NOT ship another bump until the crash is cleared.
- **A2 (release):** publish v1.4.6 assets (or pin BINARY_VERSION to a published
  release) - BINARY_VERSION must never point at an unpublished tag.
- **A3 (resilience):** `_load_dylib`'s missing-dylib path should degrade to
  "plugin disabled, retry on next session" - already graceful; the dead end is
  the 404, fixed by A2.
- **A4 (guard):** `_check_version` should WARN LOUDLY when BINARY_VERSION names
  an unpublished release (defensive; currently silent pass on 1.4.6==1.4.6).

Reports: `HERMES-UPDATE-ERROR-FORENSICS.md` (log side),
`HERMES-UPDATE-ERROR-PLUGIN-SIDE.md` (code side).
