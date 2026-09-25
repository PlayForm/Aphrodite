---
name: aphrodite-release-workflow
description: "Use when releasing Aphrodite. Canonical version ledger, pre-publish trigger audit (Gate R7), publishing gates, artifact contract matrix, release-notes standards."
version: 2.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-release-workflow
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, release, gates, version-ledger, crates-io, release-notes]
        related_skills: [aphrodite-boundaries, aphrodite-orientation, aphrodite-release-flow]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
    runtime_modes:
        - source
        - installed
owns:
    - Version ledger (canonical 5 rows)
    - Artifact contract matrix
    - Publishing gates (Gate R7 trigger audit + 4-event publishing separation)
    - Release-notes standards + documentation lint
    - Headroom fork crate publishing (references/headroom-publish.md)
depends_on:
    - aphrodite-boundaries (stop conditions, approval boundaries, git repair taxonomy)
    - aphrodite-orientation (mandatory preflight before any mutation)
    - aphrodite-release-flow (promotion/hotfix/tag/sync ceremony - the gates below are its checks)
supersedes: none
verification:
    source_of_truth:
        - Workflow files at the exact release commit (.github/workflows/Publish.yml, Build.yml)
        - Version manifests (crates/aphrodite/Cargo.toml, crates/aphrodite-hermes/Cargo.toml, plugins/aphrodite/plugin.yaml, plugins/aphrodite/BINARY_VERSION, package.json, README.md)
        - crates.io index (https://index.crates.io/ap/hr/aphrodite-headroom-core)
mutation_level: publish
---

# Aphrodite Release Workflow

Canonical owner of the release **gates**; the promotion/hotfix/tag/sync **ceremony** is owned
by `aphrodite-release-flow` (Gate R7 and the publishing separation are referenced from there,
never duplicated). This skill's file writes are documentation only; every release mutation it
gates is executed by the ceremony under a human approval boundary.

## When to Use

- Before any version bump, tag, artifact publish, or registry publish: verify the version
  ledger, audit tag triggers, validate the artifact contract.
- Writing or auditing release notes (live or retrospective mode).
- Publishing the owned `aphrodite-headroom-core` fork crate.

## Orientation gate (mandatory)

No mutation before this gate: run the 5 read-only commands from `aphrodite-orientation`
(root, branch, status, `git submodule status --recursive`, remotes). Capture `HEAD` and the
remote tip first (`git rev-parse HEAD; git ls-remote origin <branch>`): the external
auto-committer sweeps working-tree changes, so `git status` alone is not stable evidence.
Stop if root/branch/submodule state is not as declared in scope. Stop/recovery rules:
`aphrodite-boundaries`.

## 1. Version Ledger (canonical, 5 rows)

The binary release version, the plugin package version, and `BINARY_VERSION` are **three
different values with three different owners** - never one generic "version bump." The
canonical 5-row ledger and the authority-path map with 2026-09-25 snapshot values live in
`references/version-ledger-authority.md`; the mirror rows in
`.hermes/governance/VERIFICATION-MATRIX.md` are owned here - keep them mirrored.

Development = build window; Current = distribution window; not interchangeable. Snapshot
values are **evidence, not authority**: re-derive every row from its authority path at
release time (known drift case `a81acab6`: `package.json` lagged at `1.4.6` vs `1.5.0`).

### Ledger rules

- **Local bump safe; public pointer never early.** Ahead of the tag, a local `BINARY_VERSION`
  bump is safe when the referenced binaries exist locally (`_ensure_binaries` no-ops;
  `download.sh` has nothing to fetch). At **tag time** "bump LAST": a `BINARY_VERSION` naming
  a not-yet-existing release errors on every download; `_check_version_published` warns
  before `download.sh` when the pinned `BINARY_VERSION` points at a release with no assets -
  treat that warning as a hard stop for tagging.
- **Two version tracks, never conflated:** binary `1.6.x` (parent crates + `package.json` +
  README badge) vs plugin `2.2.x` (`plugin.yaml`); separate ceremonies in separate repos,
  even on the same release.
- **Bump together (binary track):** both parent crates + `aphrodite-hermes` dep pin +
  `package.json` in one ceremony (`cargo check` fails otherwise); then `plugin.yaml` +
  `install_message` + README badges (the badge may lag two minors); then `BINARY_VERSION`
  LAST. `aphrodite-release-flow` executes this order; this ledger is the check.
- **Never reuse a claimed version:** check the registry before claiming (section 3); a
  burned crates.io version is gone forever.

## 2. Pre-Publish Trigger Audit - Gate R7 (mandatory before ANY tag)

Rule: inspect the actual workflow at the exact commit to be tagged, build a trigger table,
never trust remembered or documented behavior. A workflow file that exists is not a workflow
that behaves as documented; the `if:` conditions at the tag commit are the behavior.

Resolution of C-002: one release document claimed `Publish.yml` only publishes crates after
a deliberate `workflow_dispatch` and that a plain tag push triggers only Build.yml's GitHub
Release artifacts. **False at commit `a81acab6`**: a plain `Aphrodite/v*` tag push DOES
reach `cargo publish` for `aphrodite` and `aphrodite-hermes` (publish steps carry
`|| startsWith(github.ref, 'refs/tags/Aphrodite/')`); only `aphrodite-headroom-core` is truly
dispatch-gated. A tag push is not a build-only event. Re-audit at the tag commit:
`grep -A3 'Publish to crates.io' .github/workflows/Publish.yml`; worked trigger table for
`a81acab6` + environment/Finalize/Test-job notes: `references/gate-r7-trigger-audit.md`
(evidence, not a substitute for the audit).

### Gate R7 template (the ceremony runs this; definitions owned here)

**Read:** workflow files at the intended release commit (`.github/workflows/*.yml`); trigger
clauses for tag push, push branch, `workflow_dispatch`, `workflow_call`.
**Record:** workflows triggered by this tag; jobs publishing GitHub assets; jobs publishing
crates/packages; required secrets and manual inputs.
**Pass:** the release owner has explicitly accepted every triggered side effect.
**Stop:** any unexpected publish job is reachable from the tag.

### Notes that gate behavior

- Secrets: `CARGO_REGISTRY_TOKEN` (crates.io) + default `GITHUB_TOKEN`; manual input
  `publish_crates` (boolean). Named, never echoed.
- The tag push has no already-published version check: a re-publish errors red, a
  never-published version IS published by the tag push alone.

## 3. Publishing Separation - 4 Irreversible Events

Separate these four events; never combine them in one opaque script invocation. `cargo
publish` and a registry package version are **irreversible** (never re-tag or reuse a burned
version); a GitHub release body is amendable - the gates reflect that difference.

1. **Create the release-sync commit**
2. **Create and push the immutable tag**
3. **Publish or attach binary artifacts**
4. **Publish immutable registry packages**

Every irreversible event requires ALL of:

- **Identity confirmation** - operator, repo, branch, remote verified against scope
  (orientation gate output).
- **Version availability check** - the proposed number is not live: `curl -A <ua>
  https://crates.io/api/v1/crates/<crate>` → `max_version`; index URL = first 2 / next 2
  chars of the crate name, e.g. `https://index.crates.io/ap/hr/aphrodite-headroom-core`;
  tag not already pushed (CLAIM - no probe command in source).
- **Intended artifact/package list** - the exact assets/packages this event will publish
  (section 4 matrix; section 2 trigger table).
- **Trigger audit** - Gate R7 at the exact commit (section 2).
- **Explicit human approval** - technical readiness never authorizes an external side effect
  (`aphrodite-boundaries`); pause with the template below.
- **Post-event consumer verification** - consumer perspective, not the publisher's: binary
  `--version`, plugin load, download resolution, registry index serving the new version.

### Ready for approval (mandatory pause template)

```text
Ready for approval:
- Current commit: <sha>
- Proposed tag: <tag>
- Plugin commit: <sha>
- Expected triggered workflows: <list>
- Expected external publications: <list>
- Verification gates passed: <list>
- Known degraded conditions: <list or none>
```

## 4. Artifact Contract Matrix

Consumer paths are the ground truth for what a release must ship. **Consumer download names
must match published release assets exactly**; the **installer must not fail if an optional
artifact is absent** (degrade with a warning - "degrade", not fail-closed). Build.yml stages
per target: `aphrodite-<target>[.exe]`, `libaphrodite_hermes-<target>.{so,dylib,dll}`,
`SHA256SUMS-<target>.txt` (4 targets × 3 = 12 assets; `Finalize` enforces all 12). Download
base URL: `https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{version}`.
Consumer-path matrix (macOS setup, plugin loader, Windows setup, Linux setup) + clean-install
simulation: `references/artifact-contract.md`.

### Verification before tag publication

A successful build must not become a failed first-run setup; verify the asset list against
the live release:

```bash
gh release view "Aphrodite/v<version>" --repo PlayForm/Aphrodite --json assets \
  -q '.assets[].name' | grep -E 'aphrodite-|libaphrodite_hermes-|SHA256SUMS-'
```

Every consumer-required name must appear in the asset list. `BINARY_VERSION` is NOT bumped
until this passes (ledger rule + release boundary in `aphrodite-boundaries`).

## 5. Release Notes - Content Standards

Every release MUST include Summary, Changes, Infrastructure (live) or Verification
(retrospective), What Ships, and Links. Template: `.hermes/release/RELEASE-TEMPLATE.md`
(retrospective never claims to have rebuilt or retested a shipped release).

- Drafts in `.hermes/release-notes/` (`vNEXT-draft.md`, `headroom-fork-vNEXT-draft.md`);
  stage the final file (`.hermes/release-notes/release-notes-vX.Y.Z.md`) and verify it is
  committed - it can drop out between `git add` and commit.
- Live mode: a real `### Infrastructure` section with commands you actually ran.
  Retrospective: `### Verification` (commit range, diffstat) - never re-test.
- `{PENDING}`, `{VERSION}`, `{PLUGIN_VERSION}`, `DO NOT PUBLISH` are draft placeholders -
  never publish a note still containing `{PENDING}`.
- Headroom-fork notes: retrospective, separate from the binary notes; fork scheme
  `aphrodite-vX.Y.Z`, package `aphrodite-headroom-core`.
- **What Ships** = full fixed 4-target matrix, never a point-in-time snapshot; never write
  "no Windows release" - the slow Windows leg is a timing race (CLAIM - no probe in source)
  and `Finalize` fails the release if the matrix is incomplete.
- Credit: `Co-authored-by: Name <email>` trailers; `Fixes #N` in the change bullet or commit.
- Never ship a bare compare link with zero description. UNKNOWN - reason not stated in source.
- Never use backticks with `gh release create --notes` - the shell interprets them as
  command substitution. Always `--notes-file` with a heredoc (scratch in `.hermes/tmp/`,
  never `/tmp`). Heredoc template + glob attach command: `references/release-notes-template.md`.

## 6. Documentation Lint (release docs)

Every release-facing document (this skill, `aphrodite-release-flow`, `RELEASE-TEMPLATE.md`,
draft notes) passes this checklist before a release claim:

- Exactly one `status` field; every `deprecated`/`archived` document names a successor; no
  active skill references an archived skill without a **Historical only - do not execute**
  label.
- Every mutation command appears in a step with `Preconditions`, `Verify`, `Stop if`,
  `Recovery`; line numbers are observational evidence, never durable coordinates - mark
  hard-coded source lines historical or pair them with a path/function search.
- Every threshold labeled live-read, default, or test fixture; every env var has a consumer
  status (active, inactive, removed); every branch mention declares its branch scope.
- Every release action has a human-approval boundary (section 3); no secret-like variable
  printed in sample commands (`CARGO_REGISTRY_TOKEN` named, never echoed).
- Release-specific: version claims resolve to the ledger (section 1) - no orphan numbers;
  artifact names match Build.yml staging names; trigger claims point at Gate R7 (section 2)
  with "re-verify at the tag commit"; no `{PENDING}`/`DO NOT PUBLISH` in anything publishable;
  README badge values match their ledger authority.

## 7. Headroom Fork Crate Publishing

The owned fork crate `aphrodite-headroom-core` is published from
`vendor/headroom/crates/headroom-core/Cargo.toml`; parent pin `crates/aphrodite/Cargo.toml`
line 55 (observational, re-derive live: `package = "aphrodite-headroom-core", version = "0.1.2"`;
`0.1.3` planned next - CLAIM, a plan). Publish is dispatch-gated (never a tag push), versions
immutable, CI publishes the **parent-recorded gitlink tree**, not the local submodule HEAD.
Full evidence note + checklist: `references/headroom-publish.md`.

### Mandatory fork release-cycle tracking (every parent release)

The fork is a tracked part of **EVERY** parent release cycle - never an optional afterthought.
Before any parent release: compare fork HEAD vs last published commit (`git -C vendor/headroom
log <last-published-commit>..HEAD --oneline`; last published = the 0.1.2 bump `c6b61470`);
carry **any** delta - bump the fork crate version AND the parent pin (line 55) together,
create the fork tag (scheme `aphrodite-vX.Y.Z`, never parent `Aphrodite/v*`), dispatch
`Publish.yml` with `publish_crates=true` so the fork publishes FIRST in the `needs:` chain
(Test → Publish-Headroom-Core → Publish-Aphrodite → Publish-Hermes); update
`vendor/headroom/CHANGELOG.md` + `vendor/headroom/RELEASE-CYCLE.md` per cycle.

### Published-version trap

A stale fork version looks "already published", so CI skips it. 1.5.0: the version check
(Publish-Headroom-Core job, lines 143-161 - observational) saw `0.1.2` in the index →
`published=true` → the publish step (`workflow_dispatch && publish_crates && published ==
'false'`) skipped - consumers resolved the OLD `0.1.2`, nothing failed. "Already published"
≠ "fork tree published" - compare the fork tree, never just the version number. Full case +
post-event index check: `references/headroom-publish.md`.

### Version source of truth (live-read, never a doc number)

Authority: `vendor/headroom/crates/headroom-core/Cargo.toml` `version` + the parent pin
(line 55) - move together in one ceremony; never one alone. The §1 ledger's 5 rows are
unchanged by the fork; claim numbers via the availability check in
`references/headroom-publish.md`; never reuse a burned version.

## Related

`aphrodite-release-flow` (ceremony; never duplicated), `aphrodite-boundaries` (stop/recovery,
approval boundaries, failure policy), `aphrodite-orientation` (preflight).
`.hermes/governance/VERIFICATION-MATRIX.md` (mirror ledger; owned here),
`.hermes/release/RELEASE-TEMPLATE.md` (notes contract). References:
`references/version-ledger-authority.md`, `references/gate-r7-trigger-audit.md`,
`references/artifact-contract.md`, `references/release-notes-template.md`,
`references/headroom-publish.md`.

## Claim-to-Test Matrix

| Claim | Evidence source | Test | Pass condition | Failure response |
| --- | --- | --- | --- | --- |
| Ledger rows match their authority manifests | `crates/aphrodite/Cargo.toml`, `crates/aphrodite-hermes/Cargo.toml`, `plugins/aphrodite/plugin.yaml`, `plugins/aphrodite/BINARY_VERSION`, `package.json`, README badges | Read each authority path at release time | Each row equals its authority (1.4.6-vs-1.5.0 lag = known failure) | Fix the manifest before claiming; report the drift |
| Tag push side effects are exactly as audited | Workflow files at the exact tag commit | Gate R7: build the trigger table from the actual files | Only accepted jobs reachable from the tag | Change the workflow or halt the tag |
| Tag push reaches `cargo publish` for `aphrodite`/`aphrodite-hermes` | Publish.yml publish-step `if:` conditions | `grep -A3 'Publish to crates.io' .github/workflows/Publish.yml` at the tag commit | Both publish steps carry `startsWith(github.ref, 'refs/tags/Aphrodite/')` - or audit table records change | Accept the side effect explicitly or halt the tag |
| `aphrodite-headroom-core` is never published by a tag push | Publish.yml Publish-Headroom-Core step | Read the publish-step `if:` at the tag commit | `workflow_dispatch && inputs.publish_crates && published == 'false'` only | Tag-reachable headroom publish = unexpected side effect - stop |
| Consumer download names match release assets | Build.yml staging names + download.sh/download.ps1 asset names | `gh release view "Aphrodite/v<ver>" --json assets` vs section 4 | Every required name present (12 assets) | Do NOT bump `BINARY_VERSION`; fix artifact build/attach |
| Missing optional asset degrades, never bricks setup | `download.sh` SUMS path, `setup/dylib.rs` optional arm | Simulate a missing `SHA256SUMS-<target>.txt` and a missing `libaphrodite.dylib` in a clean install | Warning + continue; setup completes | Fix the consumer script; re-run the simulation |
| Proposed registry version is available | crates.io index/API | `curl -A <ua> https://crates.io/api/v1/crates/<crate>` / index URL | `max_version` (or `vers`) does not contain the proposed number | Claim the next number; never re-publish |
| Release note is publishable | Draft file | Grep for `{PENDING}` / `DO NOT PUBLISH` / bare compare link | Zero matches; Summary + Changes present | Fix the note before publish |
| Every irreversible event pauses for human approval | This skill + `aphrodite-boundaries` | Dry-run the 4-event separation with a simulated ceremony | Workflow halts at each `Ready for approval` boundary | Enforce the gate; never chain events in one script |
| Fork delta is carried into every release | `git -C vendor/headroom log <last-published-commit>..HEAD` + `vendor/headroom/RELEASE-CYCLE.md` | Step I5 fork-delta check (`aphrodite-release-flow`) | Delta = 0, or fork crate + parent pin bumped together and fork tag exists before dispatch | Publish the fork with the release; never ship a stale fork version (1.5.0 trap) |