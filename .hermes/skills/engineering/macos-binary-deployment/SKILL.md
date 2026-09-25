---
name: macos-binary-deployment
description: "Use when deploying the freshly built Aphrodite binary and libaphrodite_hermes.dylib into ~/.hermes/aphrodite/binaries on macOS. Covers Gatekeeper, quarantine xattr, ditto copies, codesign, and spctl assessment."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: engineering
category_taxonomy: engineering/macos-binary-deployment
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, deploying, codesigning, gatekeeping, load-verifying]
        related_skills:
            - hermes-agent
            - playform-cargo-maintenance
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
owns:
    - The deploy flow into ~/.hermes/aphrodite/binaries (binary + dylib)
    - Signature state of the installed artifacts (ad hoc re-sign)
    - Load verification (probe the loaded image, not the file)
depends_on:
    - aphrodite-orientation (preflight gate before any mutation)
supersedes: []
verification:
    source_of_truth:
        - codesign/spctl output on the installed artifacts
        - The host's feature probe of the loaded image
mutation_level: mutates
---

# macOS Binary Deployment (Aphrodite)

Deploys the freshly built Aphrodite binary and `libaphrodite_hermes.dylib`
into `~/.hermes/aphrodite/binaries` on macOS and verifies the host actually
loads them. Core job: hand-installing locally built (cargo/rustc) artifacts
into the runtime dir and diagnosing the load crashes that follow a naive
`cp`.

## Preflight (orientation gate)

This skill mutates state, so run the `aphrodite-orientation` gate before the
first deploy step and record its five outputs (root, branch, status,
submodule, remote). No deploy begins from an unverified or dirty state.

## Deploy procedure

0. **Source the environment file first when rebuilding.** The private env
   file's `cargo()` wrapper syncs binary + dylib together; a fresh release
   build lands in `target/release`, and running `aphrodite setup` from THAT
   binary installs it. Skipping the env file leaves a stale PATH-installed
   copy shadowing the fresh build.
1. **Build first.** Produce the fresh artifacts (`aphrodite` binary and
   `libaphrodite_hermes.dylib`) from the current line, then confirm both
   exist before touching the runtime dir.
2. **Back up the installed artifacts first** (timestamped dir under
   `~/.hermes/tmp/`). Restoring a known-good image is instant; rebuilding is
   not.
3. **Stage the new artifacts** next to the install dir, then copy both into
   `~/.hermes/aphrodite/binaries` in ONE step. `ditto` preserves metadata
   (extended attributes, resource forks); plain `cp` is acceptable for the
   pair but leaves xattrs to manual handling.
4. **Strip quarantine.** Artifacts that arrived via download or transfer may
   carry `com.apple.quarantine`; Gatekeeper flags quarantined binaries on
   first launch. `xattr -d com.apple.quarantine <installed-file>` when the
   attribute is present.
5. **Re-sign ad hoc on the INSTALLED copy**: `codesign --force --sign -
<installed-file>` for the binary and the dylib. Mandatory for dev-built
   (linker-signed, ad hoc) dylibs on arm64, not optional polish.
6. **Assess Gatekeeper**: `spctl --assess --type execute
<installed-file>`. A `rejected` result means the quarantine or signature
   state is still wrong.
7. **Verify the LOAD, not the file.** Probe the host actually loads the
   new image from the installed path (ctypes `dlopen` + version/feature
   probe, engine `--version`, stats fields), not file mtime or `shasum`.
8. **On crash: restore the backup**, then confirm the same bytes load
   from another path before blaming the build.

## Pitfalls

- **Dev-built dylibs copied into a runtime dir can SIGKILL the host at
  load**: `EXC_BAD_ACCESS (SIGKILL)` / `Termination Reason: Namespace
CODESIGNING, Code 2, Invalid Page` at `dlopen`, even when `codesign
-dv` reports a valid `adhoc, linker-signed` signature. The one-step
  guard is `codesign --force --sign - <installed-file>` after every
  hand-copy.
- **An in-place `cp` overwrite of a previously loaded image can
  transiently trip the dyld signature cache** - the same bytes load fine
  from a fresh path. Re-sign after overwriting, or stage the new copy at
  a fresh path, before the host loads it again.
- **Byte-identical copy (matching `shasum`) is not proof the load will
  succeed** - signature state is per-path, not per-content.
- **`dlopen` memoizes by path**: a process that already loaded the old
  dylib keeps serving the old image until restart, and a restarted host
  may pick up a half-copied pair. Replace binary + dylib in one step and
  re-sign both; verify the LOADED image's version via a feature probe
  (new stats fields / new CLI flags), not file dates.
- **Quarantine + Gatekeeper**: an unidentified-developer block on first
  launch usually means the quarantine xattr survived or the ad hoc
  signature is stale. Strip the attribute and re-sign, then re-run
  `spctl`.
- **Don't leave the runtime dir half-updated** - a mismatch between
  binary and dylib versions is worse than neither updated.

## Verification matrix

| Claim                               | Evidence source | Test                                                 | Pass condition                       | Failure response                             |
| ----------------------------------- | --------------- | ---------------------------------------------------- | ------------------------------------ | -------------------------------------------- |
| Signature state is valid on install | `codesign`      | `codesign --verify --deep --strict <installed-file>` | Exit 0 on binary and dylib           | Re-sign ad hoc, re-assess                    |
| Gatekeeper accepts the artifact     | `spctl`         | `spctl --assess --type execute <installed-file>`     | `accepted`                           | Strip quarantine, re-sign, re-assess         |
| Host loads the new image            | Host probe      | ctypes `dlopen` + feature probe of loaded version    | New stats fields / CLI flags visible | Restore backup, verify bytes from fresh path |
| Runtime dir is not half-updated     | Directory       | Both artifacts present, versions match               | Pair updated together                | Re-copy the missing artifact, re-sign        |
