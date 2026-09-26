---
name: aphrodite-development
description: "Use when developing the aphrodite plugin. Mode-branched (source vs installed) session setup, verifiable dev-loop gates, inert auto-expand doctrine."
version: 2.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-development
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, development, dev-loop, cargo-watch, source-mode, build]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-testing-discipline,
                aphrodite-operations,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
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
supersedes: []
verification:
    source_of_truth:
        - .hermes/AGENTS.md (repo facts, quality gates)
        - crates/aphrodite/Cargo.toml + crates/aphrodite-hermes/Cargo.toml (workspace manifests)
        - ~/.hermes/aphrodite/ (runtime home layout)
mutation_level: local
---

# Aphrodite Development

Local iteration and runtime readiness for the Aphrodite monorepo
(`PlayForm/Aphrodite`, plugin submodule at `plugins/aphrodite`, remote
`Source`). Supersedes the legacy lessons skill (v1.x). It drives the
**Prepare** and **Validate** phases of the unified lifecycle (full table in
`aphrodite-orientation`); it is `mutation_level: local` - it edits local
source, config, and scratch, never Git history or releases.

## Choose a mode before any build advice

Every setup step begins by proving which runtime mode it is in. A step never
advises `cargo build` until it has proved source mode; an installed user is
never told to repair a workspace that the mode gate proved absent. The full
source/installed comparison matrix and the pinned versions are the worked
matrix in `references/mode-decision.md`; Step 1 below is the probe that fills
it in.

Current pins (verify against the manifests, never memorize): binary **1.6.2**
(`crates/aphrodite` + `crates/aphrodite-hermes`), plugin **2.2.2**
(`plugins/aphrodite/plugin.yaml`), `BINARY_VERSION` **1.6.2**
(`~/.hermes/aphrodite/BINARY_VERSION`).

## C-001: auto-expand is configuration observability only

Auto-expand fields are **configuration observability only** until an active
consumer is shown in the running binary - never a debugging guarantee. When a
CCR marker appears, retrieve it through the canonical retrieval route;
validate any claimed auto-expansion behavior using an explicit before/after
test.

Contradiction C-001 ("enable auto-expand" vs "no active consumer") is
resolved: auto-expand is **inert configuration** - parsed and echoed, no
consumer; never a debugging guarantee, never a remediation for raw CCR
markers. The canonical owner of C-001 and of the inert-configuration record
is `aphrodite-auto-expand-testing` (v3.0.0); this skill only consumes the
doctrine. Do not configure auto-expand as a fix for raw markers; the only
working path is retrieval.

The verification for any auto-expansion claim is the fixture acceptance test -
the 6-step probe procedure in `references/auto-expand-fixture.md`. With the
current source the expected observation is: a marker appears and requires
canonical retrieval, which shows the configuration is inert. If the test ever
shows resolution without retrieval, a real consumer is active: run the
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
ls ~/.hermes/plugins/aphrodite/plugin.yaml ~/.hermes/plugins/aphrodite/__init__.py 2> /dev/null
echo "LOADER:$?"
cat ~/.hermes/aphrodite/BINARY_VERSION
```

**Expected**

- `WORKSPACE:0` at the repo root → **source mode**: `cargo build` is
  permitted; edits require reinstall + restart.
- `WORKSPACE` non-zero but `RUNTIME_HOME:0` → **installed mode**: never
  `cargo build`; edits do not affect the installed plugin.
- Both non-zero → **stop**: not an Aphrodite dev environment.
- `LOADER:0` + `BINARY_VERSION` == the crate version (1.6.2) → the hooks-only
  loader layout is intact.

**Stop if**

- Root or branch differs from scope; submodule state is broken (see
  `aphrodite-orientation` submodule diagnosis).

**Recovery**

- Permitted: re-run the orientation gate; run `git submodule update --init`
  only after the orientation preflight reports the parent state clean.
- Prohibited: switching branches or cleaning the working tree to "fix" the
  gate - that rewrites Git state to mask a misdiagnosis; this skill never
  edits Git history (`mutation_level: local`).

**Produces**

- A declared mode (`source` | `installed`) - the branch every later step
  takes.

### Step 2 - Verify the plugin loader binding

**Purpose:** Confirm the installed loader resolves to this repository (source
mode) or is the shipped hooks-only layout (installed mode), so edits reach the
runtime.

**Preconditions**

- Step 1 declared a mode.

**Do**

```sh
# source mode: the loader source lives in the repo
ls plugins/aphrodite/plugin.yaml plugins/aphrodite/__init__.py plugins/aphrodite/BINARY_VERSION
# installed mode: the loader layout is hooks-only
ls ~/.hermes/plugins/aphrodite/plugin.yaml ~/.hermes/plugins/aphrodite/__init__.py
```

**Verify**

```sh
test -f ~/.hermes/plugins/aphrodite/plugin.yaml && test -f ~/.hermes/plugins/aphrodite/__init__.py
echo "LOADER:$?"
grep -q '^1.6.2$' ~/.hermes/aphrodite/BINARY_VERSION
echo "PIN:$?"
```

**Expected**

- source: loader source present in `plugins/aphrodite/`; installed: only
  `plugin.yaml` + `__init__.py` under `~/.hermes/plugins/aphrodite` (no
  `_core/`, no `_hooks/`).
- `PIN:0` - the `BINARY_VERSION` pin matches the built binary.

**Stop if**

- source mode and the installed loader is stale - background workers keep
  running old code. Refresh it by re-running `aphrodite setup` from the
  freshly built binary (setup removes stale plugin-dir symlinks, writes the
  loader, writes the `BINARY_VERSION` pin, registers via
  `hermes plugins enable`, and prints both locations - there is no `ln -s`
  step anymore).

**Recovery**

- Permitted: re-run `aphrodite setup` after a build to refresh loader + pin.
- Prohibited: editing the installed loader copy directly instead of repo
  source - the next `aphrodite setup` overwrites the loader, so the edit is
  lost and the repo stays unchanged.

**Produces**

- A declared loader-to-source binding, or a declared installed layout.

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

- `cargo watch` compiles both packages on save (probe:
  `aphrodite_rebuild` / `aphrodite_stats` report the freshly built dylib
  version and a healthy proxy).

**Expected**

- A save in `crates/aphrodite-hermes/` rebuilds `libaphrodite_hermes.dylib`
  (probe: `aphrodite_rebuild` reports the fresh dylib version).
- There is no mtime hot-reload dir in the runtime home - after a build, source
  the environment file first (its `cargo()` wrapper syncs binary + dylib),
  then run `aphrodite setup` from the fresh `target/release` binary to install
  binary + dylib into `~/.hermes/aphrodite/binaries/`, refresh the
  `BINARY_VERSION` pin, and rewrite the loader; then restart Pane 1 for a
  fresh process.

**Stop if**

- The dylib is not rebuilt after a hermes-crate save - the watch command was
  single-package.

**Recovery**

- Permitted: re-run watch with both `-p` flags; restart Pane 1.
- Prohibited: blaming the plugin for code the watch never rebuilt - a
  single-package watch never built the dylib, so the behavior is not the
  plugin's.

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
- The proxy dies with `no API key configured - set APHRODITE_API_KEY env var`:
  the key is absent OR commented out in the environment file - a commented-out
  line behaves exactly like an absent var. Verify `env | grep APHRODITE_API_KEY`
  shows it exported; never assume from the file's text.
- You expect `aphrodite setup --api-key/--api-url/--model` to write into the
  TOML: it does not - the template substitutes ONLY the proxy ports (cache
  9797 / token 9798). `api_url`/`model` are env-driven (`APHRODITE_API_URL` /
  `APHRODITE_MODEL`); the key comes from `APHRODITE_API_KEY` or `[defaults]
api_key` in the TOML.

**Recovery**

- Permitted: set `env_passthrough` explicitly; export the key; restart the
  session.
- Prohibited: hardcoding the key into source or fixtures - repo-tracked files
  would leak it; the secrets model allows only presence checks and redacted
  fingerprints.

**Produces**

- A proxy that starts against the real key without echoing it.

### Step 5 - Verify hot reload / fresh process after source edits

**Purpose:** Attribute results to the current source, not a stale dylib
process or symlink target.

**Preconditions**

- A source edit has been made (Validate phase).

**Do**

- Reinstall the fresh build and restart the session for a fresh process - the
  rebuild path is Step 3 (Expected); there is no mtime hot-reload dir in the
  runtime home.

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
  reload test - a stale process reports old behavior; only a fresh process
  shows the running binary is the one just built.

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
- Prohibited: `git reset`/checkout to erase probe artifacts - erasing via Git
  rewrites the worktree instead of removing the artifact; the
  `aphrodite-boundaries` git repair taxonomy forbids that path.

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
gates). The per-mode lists are the worked criteria in
`references/acceptance.md`.

## Validate-phase pitfalls

- The repo's dev skills live in `.hermes/skills/` (Development branch only,
  never shipped with the plugin) - edit the files directly with
  `write_file`/`patch`, never via `skill_manage`; `skill_manage` writes to the
  profile skill store, not the repo tree, so the change would not land in
  `.hermes/skills/`.
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
- **The dylib path has NO tracing subscriber** - `tracing::warn!`/`info!`
  are silent no-ops inside the Hermes host (a Python process never installs
  a Rust subscriber), and the engine binary loads config before `main()`
  installs one. Any diagnostic that must be seen (config parse failures,
  degraded states) needs a stderr fallback
  (`tracing::dispatcher::has_been_set()` -> `eprintln!`) or a state field
  surfaced in `aphrodite_stats` (e.g. `config_error` for found-but-broken
  aphrodite.toml). Verify with `cargo run --example` in a subscriber-less
  process, not with unit tests (the harness installs a subscriber).

## Build/test/CI gates

- Build: `cargo build --release -p aphrodite -p aphrodite-hermes`.
- Tests: `cargo test -p aphrodite --lib setup::tests` (and module-scoped
  variants).
- CI: `Build.yml` (4-target matrix, 12 assets) + `Publish.yml` (cargo publish
  chain: headroom-core → aphrodite → aphrodite-hermes); release tags use the
  `Aphrodite/v*` scheme.

## Lifecycle phases

This skill drives **Prepare** (local code/docs/config edits, targeted static
checks) and **Validate** (builds, tests, isolated runtime probes) from the
unified lifecycle table in `aphrodite-orientation`. Hard stops: unverified
assumptions about live behavior (Prepare); test failures or stale-process
uncertainty (Validate). On any stop, recover per `aphrodite-boundaries`
(stop/recovery semantics, git repair taxonomy).

## Local claim-to-test matrix

| Claim                             | Evidence source                 | Test                                                | Pass condition                                    | Failure response                     |
| --------------------------------- | ------------------------------- | --------------------------------------------------- | ------------------------------------------------- | ------------------------------------ |
| Mode gate precedes build advice   | This skill                      | Run Step 1 in an installed-only home                | Mode declared `installed`; no cargo build advised | Fix the gate ordering                |
| Watch rebuilds both packages      | `cargo watch` output            | Save in `crates/aphrodite-hermes/`                  | dylib mtime/version changes                       | Re-add `-p aphrodite-hermes`         |
| Plugin resolves to repo source    | `readlink`                      | Step 2 check                                        | Symlink == `$PWD/plugins/aphrodite`               | Re-symlink; re-verify                |
| Auto-expand is inert (C-001)      | `aphrodite-auto-expand-testing` | Fixture test in `references/auto-expand-fixture.md` | Marker appears; retrieval required                | Reactivation gate in the owner skill |
| env_passthrough reaches proxy     | `aphrodite_stats`               | Step 4                                              | Proxy healthy, no key error                       | Set passthrough; restart session     |
| Stale process does not mask edits | `aphrodite_rebuild`             | Step 5                                              | Version matches fresh build                       | Fresh-process restart                |
| Scratch stays hermetic            | `git status --short`            | Step 6                                              | No probe artifacts outside `.hermes/tmp/`         | Move artifacts; edit directly        |
