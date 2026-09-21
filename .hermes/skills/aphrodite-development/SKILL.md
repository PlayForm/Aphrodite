---
name: aphrodite-development
description: "Use when developing the aphrodite plugin. Mode-branched (source vs installed) session setup, verifiable dev-loop gates, inert auto-expand doctrine."
version: 2.0.0
platforms: [macos]
tags: [aphrodite, development, dev-loop, cargo-watch, source-mode]
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
    - The mode branch (source vs installed) that every setup step must pass before it advises a build
    - The dev-loop session setup (Pane 0 / Pane 1) and its verifiable gates
    - Source-mode and installed-mode acceptance definitions
    - The inert auto-expand doctrine as consumed by development (C-001 pointer only)
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-testing-discipline
supersedes:
    - aphrodite-development-lessons
verification:
    source_of_truth:
        - .hermes/AGENTS.md (repo facts, quality gates)
        - crates/aphrodite/Cargo.toml + crates/aphrodite-hermes/Cargo.toml (workspace manifests)
        - ~/.hermes/aphrodite/ (runtime home layout)
mutation_level: local
---

# Aphrodite Development

Local iteration and runtime readiness for the Aphrodite monorepo
(`PlayForm/Aphrodite` parent + `PlayForm/Aphrodite-Hermes` plugin submodule at
`plugins/aphrodite`). Supersedes `aphrodite-development-lessons` v1.x. This
skill drives the **Prepare** and **Validate** phases of the unified lifecycle
(full table in `aphrodite-orientation`); it is `mutation_level: local` - it
edits local source, config, and scratch, never Git history or releases.

Every setup step begins by proving which runtime mode it is in. A step must
never advise `cargo build` until it has proved source mode; conversely, an
installed user is never told to repair a workspace that is not supposed to
exist.

## Choose a mode before any build advice

| Check           | Source development                          | Installed/user diagnosis                       |
| --------------- | ------------------------------------------- | ---------------------------------------------- |
| Workspace       | Parent Cargo workspace must exist           | Cargo workspace may legitimately be absent     |
| Plugin source   | Direct repository symlink expected          | Installed package/layout expected              |
| Binary          | Build from source allowed                   | Release artifact download path used            |
| Code edits      | Local source reload/restart required        | Do not assume edits affect installed plugin    |
| Version truth   | Workspace manifests plus submodule metadata | Installed binary plus `BINARYVERSION` pairing  |
| Primary failure | Stale symlink/process                       | Missing, incompatible, or unavailable artifact |

## C-001: auto-expand is configuration observability only

> Auto-expand fields are **configuration observability only** until an active
> consumer is verified in the running binary. Do not treat them as a debugging
> guarantee. When a CCR marker appears, retrieve it through the canonical
> retrieval route; validate any claimed auto-expansion behavior using an
> explicit before/after test.

Contradiction C-001 ("enable auto-expand" vs "no active consumer") is
resolved: auto-expand is **inert configuration** - parsed and echoed, no
consumer. It is never a debugging guarantee and never a remediation for raw
CCR markers. The canonical owner of C-001 and of the inert-configuration
record is `aphrodite-auto-expand-testing` (v3.0.0); this skill only consumes
the doctrine. Do not configure auto-expand as a fix for raw markers; the only
working path is retrieval.

The verification for any auto-expansion claim is the fixture acceptance test:

1. Create a known payload larger than the compression threshold.
2. Read it once under the proposed auto-expand setting.
3. Record whether the response is inline content, a valid marker, or a
   malformed result.
4. If it is a marker, resolve it once through the canonical retrieval tool.
5. Compare bytes or normalized text with the source payload.
6. Record the runtime binary version and configuration source.

With the current source the expected observation is: a marker appears and
requires canonical retrieval - proving the configuration is inert. If the test
ever shows resolution without retrieval, a real consumer exists: run the
reactivation gate in `aphrodite-auto-expand-testing` and update the record
from `inactive` to `active`.

## Dev-loop gates

### Step 1 - Prove the runtime mode (read-only)

**Purpose:** Determine whether this environment is source or installed before
any build or edit advice applies.

**Preconditions**

- `aphrodite-orientation` preflight ran (or run it now: the 5 read-only
  commands). Repo root is `PlayForm/Aphrodite`, branch is `Development`.
- Auto-committer may be active - capture `HEAD` and remote tip first
  (`aphrodite-orientation`), because `git status` alone is not stable evidence.

**Do**

```sh
git rev-parse --show-toplevel
git branch --show-current
test -f Cargo.toml && test -f crates/aphrodite/Cargo.toml && test -d crates/aphrodite-hermes
echo "WORKSPACE:$?"
test -d ~/.hermes/aphrodite/binaries && test -f ~/.hermes/aphrodite/aphrodite.toml
echo "RUNTIME_HOME:$?"
```

**Verify**

```sh
readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite 2> /dev/null
```

**Expected**

- `WORKSPACE:0` at the repo root → **source mode**: cargo build is permitted,
  edits require reload/restart.
- `WORKSPACE` non-zero but `RUNTIME_HOME:0` → **installed mode**: never cargo
  build; edits do not affect the installed plugin.
- Both non-zero → **stop**: not an Aphrodite dev environment.

**Stop if**

- Root or branch differs from scope; submodule state is broken (see
  `aphrodite-orientation` submodule diagnosis).

**Recovery**

- Permitted: re-run the orientation gate; `git submodule update --init` only
  after the parent state is verified clean.
- Prohibited: switching branches or cleaning the working tree to "fix" the
  gate.

**Produces**

- A declared mode (`source` | `installed`) - the branch every later step
  takes.

### Step 2 - Verify the plugin source binding

**Purpose:** Confirm the running plugin resolves to this repository (source
mode) or to the installed layout (installed mode), so edits reach the runtime.

**Preconditions**

- Step 1 declared a mode.

**Do**

```sh
# source mode: the profile plugin must symlink directly to the repo
readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite
# installed mode: the installed package must carry the loader file set
ls plugins/aphrodite/plugin.yaml plugins/aphrodite/BINARY_VERSION plugins/aphrodite/_bindings.py
```

**Verify**

```sh
test -L ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite && [ "$(readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite)" = "$PWD/plugins/aphrodite" ]
echo "SYMLINK:$?"
```

**Expected**

- source: symlink resolves to `$PWD/plugins/aphrodite`; installed: loader file
  set present.

**Stop if**

- source mode and the plugin is not symlinked - background workers keep
  running stale code.

**Recovery**

- Permitted: fix the symlink to point at the repo source.
- Prohibited: editing installed copies instead of repo source.

**Produces**

- Verified plugin-to-source binding, or a declared installed layout.

### Step 3 - Start the dev loop (Pane 0 + Pane 1)

**Purpose:** Launch the watch build and a live Hermes session so every save
compiles and is testable in production.

**Preconditions**

- Source mode proved (Step 1). This step is a no-op in installed mode.
- Scratch available at `.hermes/tmp/` (Step 6).

**Do**

```sh
# Pane 0 - watch BOTH packages. `-p aphrodite` alone never rebuilds
# libaphrodite_hermes.dylib (a sibling package), so the plugin keeps
# running old code while the proxy looks alive.
cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'
```

```sh
# Pane 1 - test in production
hermes --profile dev-aphrodite
```

**Verify**

- `cargo watch` compiles both packages on save.
- `aphrodite_rebuild` / `aphrodite_stats` report the freshly built dylib
  version and healthy proxy.

**Expected**

- A save in `crates/aphrodite-hermes/` rebuilds `libaphrodite_hermes.dylib`;
  the runtime home hot-reload dir (`~/.hermes/aphrodite/hotreload/`) picks it
  up on mtime.

**Stop if**

- The dylib is not rebuilt after a hermes-crate save - the watch command was
  single-package.

**Recovery**

- Permitted: re-run watch with both `-p` flags; restart Pane 1.
- Prohibited: blaming the plugin for code the watch never rebuilt.

**Produces**

- Running watch build + live session (the joyful loop).

### Step 4 - Verify the proxy starts (env_passthrough / API key)

**Purpose:** Ensure `APHRODITE_API_KEY` reaches proxy subprocesses; an empty
`env_passthrough: []` silently blocks it and proxies fail to start.

**Preconditions**

- Pane 1 session running.

**Do**

```sh
hermes config set terminal.env_passthrough '["APHRODITE_API_KEY","PATH","HOME"]' --profile dev-aphrodite
```

**Verify**

```sh
aphrodite_stats
```

**Expected**

- Proxy health endpoints report healthy; no missing-key degraded state.
- Never print the key - verify only presence, source name, and redacted
  fingerprint (`aphrodite-boundaries` secrets model).

**Stop if**

- Proxy fails to start with no visible error - check passthrough first.

**Recovery**

- Permitted: set `env_passthrough` explicitly; restart the session.
- Prohibited: hardcoding the key into source or fixtures.

**Produces**

- A proxy that starts against the real key without echoing it.

### Step 5 - Verify hot reload / fresh process after source edits

**Purpose:** Attribute results to the current source, not a stale dylib
process or symlink target.

**Preconditions**

- A source edit has been made (Validate phase).

**Do**

- Confirm the dylib hot-reloaded on mtime
  (`~/.hermes/aphrodite/hotreload/`), or restart the session for a fresh
  process.

**Verify**

```sh
aphrodite_rebuild
```

**Expected**

- Reported dylib version matches the freshly built crate; behavior changed as
  intended.

**Stop if**

- The version is unchanged after a rebuild - stale process/symlink suspected.

**Recovery**

- Permitted: restart Pane 1 (fresh process); re-check the symlink (Step 2).
- Prohibited: declaring a plugin change verified without a fresh-process or
  reload test.

**Produces**

- Evidence the running binary is the one just built.

### Step 6 - Keep scratch and environment hermetic

**Purpose:** Keep probe artifacts and env vars from leaking into the repo or
the live config.

**Do**

- Scratch belongs in `.hermes/tmp/` (gitignored contents, tracked `.gitkeep`),
  NEVER `/tmp`; gzip/tar.gz bulky fixtures in place.
- Env vars override config - keep test env hermetic (full doctrine:
  `aphrodite-testing-discipline`).

**Verify**

```sh
test -d .hermes/tmp
```

**Expected**

- All scratch under `.hermes/tmp/`; `git status --short` shows no probe
  artifacts (auto-committer aware).

**Stop if**

- A probe wrote into `/tmp` or the repo tree.

**Recovery**

- Permitted: move artifacts to `.hermes/tmp/`.
- Prohibited: `git reset`/checkout to erase probe artifacts
  (`aphrodite-boundaries` git repair taxonomy).

**Produces**

- A clean, hermetic working environment.

## Retrieve-first doctrine

In a compressed session every read returns `<<<CCR:hash|type|size>>>`. When a
marker appears, `aphrodite_retrieve(hash)` it - never re-read the source file
behind it and never try alternative tools (they produce markers too). Full
tool doctrine: `aphrodite-tool-testing`. Compression-safety boundaries (what
may never be compressed, retrieval exemptions): `aphrodite-compression-safety`.

## Acceptance definitions

A change is done only when its mode's acceptance list passes with recorded
output - report what commands printed, never "should pass" (AGENTS.md quality
gates).

### Source-mode acceptance

- Workspace discovered.
- The intended crate builds from source.
- Plugin points at the repository source.
- Restart/reload has occurred.
- Updated binary/version is observed.
- Targeted contract test passes.

### Installed-mode acceptance

- No workspace is required.
- Plugin layout validates.
- Compatible release artifact resolves.
- Download/checksum/version pairing succeeds.
- Plugin starts against the installed binary.
- Targeted contract test passes.

## Validate-phase pitfalls

- The repo's dev skills live in `.hermes/skills/` (Development branch only,
  never shipped with the plugin) - edit the files directly with
  `write_file`/`patch`, never via `skill_manage`.
- Never assume a new import is safe - a symbol the target module lacks
  silently kills the plugin at session start. After adding imports, test
  `python3 -c "import aphrodite"`.
- Version bump locations and release notes are owned by
  `aphrodite-release-flow` / `aphrodite-release-workflow` - never bump or
  announce versions from this skill.
- Dependency pins: `aphrodite-cargo-upgrade` owns the pin/migrate/abort
  decision tree.
- Context-engine activation/registration is owned by
  `aphrodite-context-engine-contract` - verify selection there, not from a
  remembered env var.

## Lifecycle phases

This skill drives **Prepare** (local code/docs/config edits, targeted static
checks) and **Validate** (builds, tests, isolated runtime probes) from the
unified lifecycle table in `aphrodite-orientation`. Hard stops: unverified
assumptions about live behavior (Prepare); test failures or stale-process
uncertainty (Validate). On any stop, recover per `aphrodite-boundaries`
(stop/recovery semantics, git repair taxonomy).

## Local claim-to-test matrix

| Claim                             | Evidence source                 | Test                                 | Pass condition                                    | Failure response                     |
| --------------------------------- | ------------------------------- | ------------------------------------ | ------------------------------------------------- | ------------------------------------ |
| Mode gate precedes build advice   | This skill                      | Run Step 1 in an installed-only home | Mode declared `installed`; no cargo build advised | Fix the gate ordering                |
| Watch rebuilds both packages      | `cargo watch` output            | Save in `crates/aphrodite-hermes/`   | dylib mtime/version changes                       | Re-add `-p aphrodite-hermes`         |
| Plugin resolves to repo source    | `readlink`                      | Step 2 check                         | Symlink == `$PWD/plugins/aphrodite`               | Re-symlink; re-verify                |
| Auto-expand is inert (C-001)      | `aphrodite-auto-expand-testing` | 6-step fixture test                  | Marker appears; retrieval required                | Reactivation gate in the owner skill |
| env_passthrough reaches proxy     | `aphrodite_stats`               | Step 4                               | Proxy healthy, no key error                       | Set passthrough; restart session     |
| Stale process does not mask edits | `aphrodite_rebuild`             | Step 5                               | Version matches fresh build                       | Fresh-process restart                |
| Scratch stays hermetic            | `git status --short`            | Step 6                               | No probe artifacts outside `.hermes/tmp/`         | Move artifacts; edit directly        |
