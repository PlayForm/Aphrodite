---
name: aphrodite-operations
description: "Use when operating inside an aphrodite-compressed session or the Aphrodite repo. Compressed-session marker discipline, source-vs-installed rebuild diagnosis, degraded modes."
version: 2.0.0
platforms: [macos]
tags: [aphrodite, ccr, compression, operations, rebuild]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Compressed-session operating rules (marker discipline, retrieve-first)
    - Source-vs-installed rebuild diagnosis and mode-proving probes
    - Degraded-mode operations (inline-only when upstream unavailable)
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-engine-observability
supersedes: []
verification:
    source_of_truth:
        - ~/.hermes/aphrodite/ (runtime home layout: aphrodite.toml, binaries/, hotreload/)
        - crates/aphrodite/Cargo.toml (source-mode workspace marker)
        - aphrodite_rebuild / aphrodite_stats probes (dylib version, proxy health)
mutation_level: local
---

# Aphrodite Operations

Day-to-day operation inside an aphrodite-compressed session and the Aphrodite
repo. Supersedes the v1.1.0 operational prose; drives the **Observe** and
**Recover** phases of the unified lifecycle (full table in
`aphrodite-orientation`). It is `mutation_level: local` - it rebuilds local
binaries and edits local config/scratch, never Git history or releases.

A step must never advise `cargo build` until it has proved it is in source
mode. Conversely, an installed user should never be told to repair a missing
workspace that is not supposed to exist.

## Diagnose the mode before any rebuild advice

The same mode table is the setup branch in `aphrodite-development`; here it is
the diagnosis applied to rebuild, repair, and version truth. Choose exactly
one mode before any mutation.

| Check           | Source development                          | Installed/user diagnosis                       |
| --------------- | ------------------------------------------- | ---------------------------------------------- |
| Workspace       | Parent Cargo workspace must exist           | Cargo workspace may legitimately be absent     |
| Plugin source   | Direct repository symlink expected          | Installed package/layout expected              |
| Binary          | Build from source allowed                   | Release artifact download path used            |
| Code edits      | Local source reload/restart required        | Do not assume edits affect installed plugin    |
| Version truth   | Workspace manifests plus submodule metadata | Installed binary plus `BINARYVERSION` pairing  |
| Primary failure | Stale symlink/process                       | Missing, incompatible, or unavailable artifact |

## Compressed-session workflow

The engine compresses every read - never fight it by re-reading the same file
with different offsets or tools. The full tool-API doctrine lives in
`aphrodite-tool-testing`; the operational shape is:

1. **Plan reads ahead** - `aphrodite_prefetch(paths=[...])` reads and
   compresses files in the background; track progress with
   `aphrodite_prefetch_status`.
2. **Retrieve, don't re-read** - on `<<<CCR:hash|type|size>>>`, call
   `aphrodite_retrieve(hash)`. Never call `read_file` again on the same file.
3. **Write terminal output to files** - `cmd > .hermes/tmp/out.txt 2>&1`,
   then prefetch/retrieve the file instead of reading raw output (scratch
   belongs in `.hermes/tmp/`, never `/tmp`).
4. **Do other work while waiting** - dispatch prefetches and independent
   tasks, then poll readiness.

Anti-pattern: calling `read_file` 3+ times on the same file with different
offsets - each call returns a fresh compressed marker.

### When NOT to compress

Compression-safety boundaries are owned by `aphrodite-compression-safety` and
`aphrodite-boundaries` (context boundaries); the operating rules:

- Never compress a retrieval response or an Aphrodite diagnostic response -
  doing so can turn the retrieval path into a marker-resolution loop.
- Never split a tool call from its matching tool result when selecting a
  context-compression boundary.
- Never re-emit a CCR marker for a value that is already a resolved retrieval
  payload.

## Rebuild and sync workflow

### Step 1 - Prove the runtime mode (read-only)

**Purpose:** Classify the environment as source or installed before any build,
repair, or version-truth advice applies.

**Preconditions**

- Orientation gate ran (root, branch, submodule, remote captured; see
  `aphrodite-orientation`).
- Auto-committer may be active - `HEAD` and remote tip captured.

**Do**

```sh
git rev-parse --show-toplevel
test -f Cargo.toml && test -f crates/aphrodite/Cargo.toml
echo "WORKSPACE:$?"
test -d ~/.hermes/aphrodite/binaries && test -f ~/.hermes/aphrodite/aphrodite.toml
echo "RUNTIME_HOME:$?"
ls plugins/aphrodite/plugin.yaml plugins/aphrodite/BINARY_VERSION 2> /dev/null
echo "LAYOUT:$?"
```

**Verify**

```sh
readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite 2> /dev/null
```

**Expected**

- `WORKSPACE:0` at the Aphrodite root → **source mode**: rebuild from source
  is the correct path.
- `WORKSPACE` non-zero, `RUNTIME_HOME:0`, `LAYOUT:0` → **installed mode**: no
  workspace to repair; artifact download path is the correct path.
- Neither → **stop**: not a runnable Aphrodite environment.

**Stop if**

- Root or branch differs from scope; submodule state broken; a phantom gitlink
  (mode-160000) appears.

**Recovery**

- Permitted: re-run orientation; `git submodule update --init` only after the
  parent state is verified clean.
- Prohibited: creating a Cargo workspace in an installed home; `git
checkout`/`reset` to repair content.

**Produces**

- A declared mode (`source` | `installed`) that gates Steps 2-3.

### Step 2 - Rebuild and reload (source mode)

**Purpose:** Rebuild the binary and dylib from source and reload so the
session runs the new code.

**Preconditions**

- Step 1 declared **source mode**.
- `aphrodite-development` Step 3 confirmed the watch loop covers BOTH
  packages (`-p aphrodite` alone never rebuilds `libaphrodite_hermes.dylib`).

**Do**

```sh
cargo build -p aphrodite -p aphrodite-hermes
```

**Verify**

```sh
aphrodite_rebuild
```

**Expected**

- Dylib version reported matches the freshly built crate; proxy health
  reports healthy.

**Stop if**

- Build fails; version unchanged after rebuild (stale process); hot-reload
  dir did not pick up the new dylib.

**Recovery**

- Permitted: restart the session for a fresh process; re-check the plugin
  symlink (`aphrodite-development` Step 2).
- Prohibited: `git reset`/checkout to erase the change; declaring success on a
  banner alone.

**Produces**

- Updated dylib plus version evidence attached to the change.

### Step 3 - Verify or repair (installed mode)

**Purpose:** Validate the installed plugin against its release artifact
without touching a workspace.

**Preconditions**

- Step 1 declared **installed mode**.

**Do**

- Confirm the loader file set is present: `plugin.yaml`, `BINARY_VERSION`,
  `_bindings.py`, `download.sh` / `download.ps1`.
- Confirm the binary exists under `~/.hermes/aphrodite/binaries/`; if missing,
  incompatible, or unavailable, re-download from the release artifact path
  (the runtime home self-heals on start: re-copies binaries/directives/
  hotreload).

**Verify**

```sh
aphrodite_rebuild
cat ~/.hermes/aphrodite/aphrodite.toml | grep -iE "binary|version" | head -20
```

**Expected**

- Reported dylib version pairs with `BINARY_VERSION`; plugin starts against
  the installed binary.

**Stop if**

- Artifact missing, incompatible, or version pairing fails - the mode's
  primary failure.

**Recovery**

- Permitted: re-download the artifact and verify checksum/version pairing;
  record degraded state if it fails.
- Prohibited: `cargo build`; repairing a missing workspace that is not
  supposed to exist.

**Produces**

- A working installed plugin, or a recorded artifact-level failure.

## Degraded modes

- **Inline-only when upstream unavailable**: when the upstream API is
  unreachable, the engine retains local/raw behavior and exposes a degraded
  status - never assume an upstream-dependent recovery action. Local health
  endpoints succeed without upstream access; upstream reachability is a
  separate probe (Layer 4, `aphrodite-engine-observability`). Failure policy:
  degrade, per `aphrodite-boundaries`.
- **`--version` before config loading (source-derived)**: the Rust binary's
  `--version` is only parsed by clap when `Cli::parse()` runs - which never
  happens when `aphrodite.toml` exists. If `[BINARY, "--version"]` hangs,
  verify the current `main()` intercepts `--version`/`-V` before config
  loading. **Confidence:** source-derived; re-check in the checked-out source
  before relying on it.

## Plugin repo is the submodule (post-merge)

`plugins/aphrodite` IS the standalone repo `PlayForm/Aphrodite-Hermes` (a git
submodule, remote `Source`) - there is no separate copy to sync. Work lands
directly inside the submodule and is carried to Current by the release
ceremony (submodule-first, `aphrodite-release-flow`). End users install from
Aphrodite-Hermes; the binary is auto-downloaded from GitHub Releases on first
session start. The loader file set is `__init__.py`, `_bindings.py`,
`BINARY_VERSION`, `download.sh` / `download.ps1`, `layout_check.py`,
`layout_schema.json`, `plugin.yaml`, `README.md`, `tests/` - no `_core/`, no
`_hooks/`.

## Dep pins

Pin dependencies to exact versions - never semver ranges. The full decision
tree (inventory, upgrade one compatibility cluster, compile minimal targets,
classify failure, choose action, behavioral tests, record the decision) is
owned by `aphrodite-cargo-upgrade`; its breakpoint records live in its
`references/breakpoints.md`. Never pin "until it compiles" without recording
the removal condition.

## Lifecycle phases

This skill drives **Observe** (health and round-trip checks after a runtime
change; diagnostics must agree with the release/version claim) and **Recover**
(only documented repair operations; root cause and clean state verified) from
the unified lifecycle table in `aphrodite-orientation`. Hard stops:
diagnostics disagree with the version claim (Observe); destructive shortcut or
inferred recovery (Recover). Repair rules come from `aphrodite-boundaries`
(git repair taxonomy, failure-behavior policy).

## Local claim-to-test matrix

| Claim                                        | Evidence source                        | Test                                                  | Pass condition                           | Failure response                                  |
| -------------------------------------------- | -------------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------------------------------------------------- |
| Mode probe classifies source vs installed    | Cargo.toml + runtime home              | Step 1 in a source repo and in an installed-only home | Correct mode declared in both            | Fix the probe/interpretation table                |
| Marker resolves via canonical retrieval      | `aphrodite_retrieve`                   | Compress known payload, retrieve once                 | Original normalized content returned     | Stop; escalate (boundaries: fail-open)            |
| Retrieval/diagnostics never re-compressed    | Transform pipeline                     | Feed a retrieval response through the transform       | Payload stays raw; no nested marker      | Update skip classifier (compression-safety owner) |
| Source rebuild updates the dylib             | `aphrodite_rebuild`                    | Step 2 with both `-p` flags                           | Version matches fresh build              | Rebuild both packages; fresh-process restart      |
| Installed repair never advises `cargo build` | This skill                             | Step 3 in an installed-only home                      | No build advice; artifact path used      | Fix the step ordering                             |
| Upstream down → inline-only degraded mode    | `aphrodite-engine-observability`       | Disable upstream, probe local health                  | Degraded status; inline content retained | Fix the degrade path (boundaries: degrade policy) |
| `BINARYVERSION` pairing verified             | `aphrodite_rebuild` + `BINARY_VERSION` | Step 3 verify                                         | Pairing matches; plugin starts           | Stop release claims; fix artifact                 |
