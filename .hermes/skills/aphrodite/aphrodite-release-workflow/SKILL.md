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

This skill is the canonical owner of the release **gates**. The promotion/hotfix/tag/sync
**ceremony** is owned by `aphrodite-release-flow` - this skill defines and verifies
the checks that ceremony must pass (Gate R7 and the publishing separation are
referenced from there, never duplicated). This skill is not the ceremony, and a
ceremony step is not a gate result. This skill's own file writes are documentation
only; every release mutation it gates is executed by the ceremony under a human
approval boundary.

## When to Use

- Before any version bump, tag, artifact publish, or registry publish: verify the
  version ledger, audit tag triggers, validate the artifact contract.
- Writing or auditing release notes (live or retrospective mode).
- Publishing the owned `aphrodite-headroom-core` fork crate.

## Orientation gate (mandatory)

No mutation before this gate - run the 5 read-only commands from
`aphrodite-orientation` and record the output: repository root, branch, status,
`git submodule status --recursive`, remotes. Capture `HEAD` and the remote tip
first (`git rev-parse HEAD; git ls-remote origin <branch>`): the external
auto-committer sweeps working-tree changes, so `git status` alone is not stable
evidence. `git status` is not stable evidence; the remote tip captured before any
edit is the comparison base. Stop if the root, branch, or submodule state is not as
declared in scope. See `aphrodite-boundaries` for the stop/recovery rules.

## 1. Version Ledger (canonical, 5 rows)

The binary release version, the plugin package version, and `BINARY_VERSION`
are **three different values with three different owners** - never described
as one generic "version bump." A version bump is not one thing. This ledger is the
canonical 5-row contract; the same rows live in `.hermes/governance/VERIFICATION-MATRIX.md`
(this skill is their owner - keep them mirrored). Authority, earliest update, latest
safe update, and verification are per row:

| Field                | Meaning                                | Authority            | Earliest update                            | Latest safe update                | Verification                                 |
| -------------------- | -------------------------------------- | -------------------- | ------------------------------------------ | --------------------------------- | -------------------------------------------- |
| Binary version       | Published Rust artifact identity       | Parent release owner | Development prep                           | Release-sync commit               | Binary `--version`, manifests, artifact name |
| Plugin version       | Plugin package identity                | Plugin owner         | Plugin prep                                | Plugin Current release commit     | `plugin.yaml`, install metadata              |
| `BINARY_VERSION`     | Binary expected by plugin installer    | Plugin release owner | Local prep only if artifact already exists | After artifact/tag availability   | Consumer download resolution                 |
| Parent gitlink       | Exact plugin commit consumed by parent | Parent Current owner | Parent sync                                | Before parent release tag         | Submodule status/tree entry                  |
| README badge/example | Documentation claim                    | Documentation owner  | After authoritative value changes          | Before release notes finalization | Render/source scan                           |

The parent binary version bumps during Development prep and is safe to update last
at the release-sync commit; the plugin version bumps before the plugin's Current
release commit. Development carries the builds; Current carries the distribution -
those are not interchangeable update windows.

Snapshot values are **evidence, not authority**: re-derive every row from its
authority path at release time. A snapshot value is not a current value. If a row
disagrees with its authority, fix the manifest before claiming a release; report the
drift (known case `a81acab6`: `package.json` lagged at `1.4.6` vs `1.5.0`). The full
authority-path map with the 2026-09-25 snapshot values lives in
`references/version-ledger-authority.md`.

### Ledger rules

- **A local bump can be safe; a public distribution pointer must not be early.**
  A local `BINARY_VERSION` bump ahead of the tag is safe when the referenced
  binaries already exist locally (the plugin's `_ensure_binaries` no-ops and
  `download.sh` has nothing to fetch). The "bump LAST" ceremony rule applies
  at **tag time**: a plugin whose `BINARY_VERSION` names a release that does
  not exist yet errors on every download. The plugin's
  `_check_version_published` warns before `download.sh` when the pinned
  `BINARY_VERSION` points at a release with no assets - treat that warning as
  a hard stop for tagging.
- **Two version tracks, never conflated:** binary `1.6.x` (parent
  crates + `package.json` + README badge) vs plugin `2.2.x`
  (`plugin.yaml`). A binary bump and a plugin bump are separate ceremonies
  (separate repos), even though they ride the same release. The binary track
  is not the plugin track.
- **Bump together (binary track):** both parent crates + the
  `aphrodite-hermes` dep pin + `package.json` must move in one ceremony
  (`cargo check` fails otherwise). Then `plugin.yaml` + `install_message` +
  README badges (they drift - the badge may lag two minors), then
  `BINARY_VERSION` LAST. The ceremony in `aphrodite-release-flow` executes
  this order; this ledger is the check it runs against.
- **Never reuse a claimed version:** check the registry before claiming a
  number (Gate R7 / section 3); a burned crates.io version is gone forever.
  A claimed-but-unpublished number is not safe to reuse.

## 2. Pre-Publish Trigger Audit - Gate R7 (mandatory before ANY tag)

The audit rule: inspect the actual workflow at the exact commit to be tagged,
build a trigger table, never trust remembered or documented behavior. A workflow
file that exists is not a workflow that behaves as documented; the file's `if:`
conditions at the tag commit are the behavior.

Resolution of C-002 (stated explicitly): one release document claimed
`Publish.yml` only publishes crates after a deliberate `workflow_dispatch`,
and that a plain tag push triggers only Build.yml's GitHub Release artifacts.
That claim is **false at commit `a81acab6`** - a plain `Aphrodite/v*` tag push
DOES reach `cargo publish` for `aphrodite` and `aphrodite-hermes` (their
publish steps carry `|| startsWith(github.ref, 'refs/tags/Aphrodite/')`);
only `aphrodite-headroom-core` is truly dispatch-gated. A tag push is not a
build-only event. Re-audit at the tag commit with
`grep -A3 'Publish to crates.io' .github/workflows/Publish.yml`; the worked
trigger table for `a81acab6` is in `references/gate-r7-trigger-audit.md` -
it is evidence, not a substitute for the audit.

### Gate R7 template (the ceremony runs this; definitions owned here)

**Read**

- Workflow files at the intended release commit (`.github/workflows/*.yml`)
- Trigger clauses for tag push, push branch, `workflow_dispatch`, and
  reusable calls (`workflow_call`)

**Record**

- Which workflows trigger from this tag
- Which jobs publish GitHub assets
- Which jobs publish crates/packages
- Required secrets and manual inputs

**Pass**

- The release owner has explicitly accepted every triggered side effect.

**Stop**

- Any unexpected publish job is reachable from the tag.

### Notes that gate behavior

- Publish jobs run in `environment: Release` - if that environment has
  required reviewers/approval rules, jobs pause there; that is a workflow-level
  human-approval boundary, not a substitute for the pre-tag audit. The
  environment gate is not the audit.
- Secrets needed for the publish path: `CARGO_REGISTRY_TOKEN` (crates.io) and
  the default `GITHUB_TOKEN`. Manual input: `publish_crates` (boolean).
  `CARGO_REGISTRY_TOKEN` is named, never echoed.
- The tag push has no already-published version check: a re-publish errors
  red, and a never-published version IS published by the tag push alone. The
  tag push is a publish trigger for the two parent crates.
- Build.yml's `Finalize` job fails the release if any of the 12 expected
  assets is missing (all four targets × binary + dylib + `SHA256SUMS`).
  12 assets are not optional.
- `Publish.yml`'s `Test` job gates the publish chain: `cargo test -p aphrodite
  -p aphrodite-hermes --release` plus a packaging guard asserting all six
  `src/builtin_directives/*.md` are inside the `cargo package` tarball (the
  v1.3.8 regression: a recursive `*.md` exclude stripped them and
  `cargo install` failed to compile).

## 3. Publishing Separation - 4 Irreversible Events

Separate these four events; never combine them in one opaque script
invocation. `cargo publish` and a registry package version are **irreversible**
(never re-tag or reuse a burned version to fix); a GitHub release body is
amendable - the gates reflect that difference. Irreversible is not amendable.

1. **Create the release-sync commit**
2. **Create and push the immutable tag**
3. **Publish or attach binary artifacts**
4. **Publish immutable registry packages**

Every irreversible event requires ALL of:

- **Identity confirmation** - operator, repo, branch, remote verified against
  scope (orientation gate output).
- **Version availability check** - the proposed number is not live: crates.io
  index/API (`curl -A <ua> https://crates.io/api/v1/crates/<crate>` →
  `max_version`; index URL scheme: first 2 / next 2 chars of the crate name,
  e.g. `https://index.crates.io/ap/hr/aphrodite-headroom-core`); the GitHub
  tag is not already pushed (CLAIM - the source states this requirement
  without a probe command).
- **Intended artifact/package list** - the exact assets/packages this event
  will publish (section 4 matrix; section 2 trigger table).
- **Trigger audit** - Gate R7 at the exact commit (section 2).
- **Explicit human approval** - technical readiness never authorizes an
  external side effect (`aphrodite-boundaries`); pause with the summary below.
  Technical readiness is not authorization.
- **Post-event consumer verification** - verify from the consumer's
  perspective, not the publisher's: binary `--version`, plugin load, download
  resolution, registry index serving the new version. The publisher's claim
  is not proof.

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

Consumer paths are the ground truth for what a release must ship. **Consumer
download names must match published release assets exactly**, and the
**installer must not fail if an optional artifact is absent** (degrade with a
warning - `aphrodite-boundaries` failure-policy "degrade", not fail-closed).
Degrade is not fail-closed. Build.yml stages, per matrix target:
`aphrodite-<target>[.exe]`, `libaphrodite_hermes-<target>.{so,dylib,dll}`,
`SHA256SUMS-<target>.txt` (4 targets × 3 = 12 assets; `Finalize` enforces all
12). Download base URL:
`https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{version}`.

The full consumer-path matrix (macOS setup, plugin loader, Windows setup,
Linux setup - required vs optional assets and the missing-asset results) is in
`references/artifact-contract.md`. `download.sh`/`download.ps1` also validate
downloaded files (non-empty, native magic bytes ELF/Mach-O/PE, exact-match
SHA-256 when a sums file is present) and restore any prior copy on failure - a
checksum mismatch is a hard error.

### Verification before tag publication

Run a **clean-install simulation or an artifact-name verification against
staged output** - a successful build must not become a failed first-run
setup:

```bash
# Option A - clean-install simulation (macOS/Linux):
BINARY_DIR="$(mktemp -d)" bash plugins/aphrodite/download.sh <version> <target>
"$HOME/.hermes/aphrodite/binaries/aphrodite" --version   # or the mktemp dir binary

# Option B - artifact-name verification against the live release:
gh release view "Aphrodite/v<version>" --repo PlayForm/Aphrodite --json assets \
  -q '.assets[].name' | grep -E 'aphrodite-|libaphrodite_hermes-|SHA256SUMS-'
```

Every consumer-required name in the matrix must appear in the asset list.
`BINARY_VERSION` is NOT bumped until this passes (ledger rule + release
boundary in `aphrodite-boundaries`).

## 5. Release Notes - Content Standards

Every release MUST include Summary, Changes, Infrastructure (live) or
Verification (retrospective), What Ships, and Links. Canonical template:
`.hermes/release/RELEASE-TEMPLATE.md` (defines **Live** vs **Retrospective**
modes - retrospective never claims to have rebuilt or retested a shipped
release). Live mode is not retrospective mode.

- Drafts live in `.hermes/release-notes/` (`vNEXT-draft.md`,
  `headroom-fork-vNEXT-draft.md`). Stage the final notes file explicitly
  (`.hermes/release-notes/release-notes-vX.Y.Z.md`) - verify it is actually
  committed; it can drop out between `git add` and commit. A staged file is
  not a committed file.
- **Live mode** (cutting the release now) requires a real `### Infrastructure`
  section with commands you actually ran. **Retrospective** (rewriting an
  already-shipped release) replaces Infrastructure with `### Verification`
  describing what was analyzed (commit range, diffstat) - never re-test.
  Retrospective analysis is not a re-test.
- Draft placeholders (`{PENDING}` in Infrastructure, `{VERSION}` /
  `{PLUGIN_VERSION}` in title/compare link, `DO NOT PUBLISH` header) are
  by-design for drafts - never publish a note still containing `{PENDING}`.
  A draft is not a publishable note.
- Headroom-fork notes are retrospective and separate from the binary notes;
  they use the fork's `aphrodite-vX.Y.Z` tag scheme and the
  `aphrodite-headroom-core` package name.
- **What Ships** lists the full fixed 4-target matrix (per
  RELEASE-TEMPLATE.md), never a point-in-time asset snapshot; never write
  "no Windows release" - the slow Windows leg is a timing race (CLAIM - the
  source states the mechanism without a probe), and Build.yml's `Finalize`
  job fails the release if the matrix is incomplete. A point-in-time
  snapshot is not the matrix.
- **Contributor credit:** co-authored work carries `Co-authored-by: Name
  <email>` trailers; issue-fixing changes reference `Fixes #N` in the change
  bullet or commit so the note links back to the issue.
- Never ship a bare compare link with zero description. UNKNOWN - the source
  states the prohibition without a reason.
- Never use backticks with `gh release create --notes` - the shell interprets
  them as command substitution. Always `--notes-file` with a heredoc (write
  the notes scratch into `.hermes/tmp/`, never `/tmp`). The full heredoc
  template and the glob attach command are in
  `references/release-notes-template.md`.

## 6. Documentation Lint (release docs)

Every release-facing document (this skill, `aphrodite-release-flow`,
`RELEASE-TEMPLATE.md`, draft notes) is checked against this checklist before
a release claim:

- Exactly one `status` field; every `deprecated`/`archived` document names a
  successor.
- No active skill references an archived skill without a **Historical
  only - do not execute** label.
- Every mutation command appears in a step with `Preconditions`, `Verify`,
  `Stop if`, and `Recovery`.
- Every hard-coded source line is marked historical or paired with a
  path/function search (line numbers are observational evidence, never
  durable coordinates). A line number is not a coordinate.
- Every threshold is labeled live-read, default, or test fixture.
- Every environment variable has a defined consumer status: active,
  inactive, or removed.
- Every mention of a branch declares its permitted branch scope.
- Every release action has a human-approval boundary (section 3).
- No secret-like variable is printed in any sample command
  (`CARGO_REGISTRY_TOKEN` is named, never echoed).
- **Release-specific:** every version claim resolves to the version ledger
  (section 1) - no orphan numbers; every artifact name in prose matches
  Build.yml's staging names; every trigger claim points at the Gate R7 audit
  (section 2) with "re-verify at the tag commit"; no `{PENDING}` /
  `DO NOT PUBLISH` in anything publishable; README badge values match their
  ledger authority.

## 7. Headroom Fork Crate Publishing

The owned fork crate `aphrodite-headroom-core` is published from
`vendor/headroom/crates/headroom-core/Cargo.toml`; parent pin
`crates/aphrodite/Cargo.toml` line 55 - observational, re-derive live:
`package = "aphrodite-headroom-core", version = "0.1.2"`; `0.1.3` is the
planned next fork release (CLAIM - a plan, not a published value). Publish is
dispatch-gated (never a tag push), versions are immutable, and CI publishes
the **parent-recorded gitlink tree**, not the local submodule HEAD. The local
submodule HEAD is not what CI publishes. Full evidence note and checklist:
`references/headroom-publish.md`.

### Mandatory fork release-cycle tracking (every parent release)

The fork is a first-class, tracked part of **EVERY** parent release cycle -
never an optional afterthought. An optional afterthought is not a tracked
release leg. Before any parent release:

- Compare the fork's current HEAD against the last published commit
  (`git -C vendor/headroom log <last-published-commit>..HEAD --oneline`;
  last published = the commit carrying the version live on crates.io - the
  0.1.2 bump `c6b61470`).
- Carry **any** delta into the release: bump the fork crate version AND the
  parent pin (`crates/aphrodite/Cargo.toml` line 55) together, create the
  fork release tag (fork scheme `aphrodite-vX.Y.Z`, never the parent
  `Aphrodite/v*` scheme), then dispatch `Publish.yml` with
  `publish_crates=true` so the fork publishes FIRST in the `needs:` chain
  (Test → Publish-Headroom-Core → Publish-Aphrodite → Publish-Hermes).
  The fork scheme is not the parent scheme.
- The fork's change ledger is `vendor/headroom/CHANGELOG.md` +
  `vendor/headroom/RELEASE-CYCLE.md` - update both per cycle; they are the
  fork's release record and the delta-check tracking contract.

### Published-version trap

A stale fork version looks "already published", so CI happily skips it. The
1.5.0 release published `aphrodite` 1.5.0 + `aphrodite-hermes` 1.5.0 but
SKIPPED the headroom publish: `Publish.yml`'s version check
(Publish-Headroom-Core job, lines 143-161 - observational) saw `0.1.2` already
in the crates.io index → `published=true` → the publish step
(`workflow_dispatch && publish_crates && published == 'false'`) skipped -
while the fork held ~14 unpublished commits since the 0.1.2 bump
(`git -C vendor/headroom log c6b61470..HEAD --oneline` counts them).
**Nothing failed, no red job - a silent gap**: external consumers of
`aphrodite` 1.5.0 resolved the OLD 0.1.2 (the post-event index check in
`references/headroom-publish.md` shows which version the index serves).
"Already published" ≠ "fork tree published". Every release cycle MUST
therefore compare the fork tree, not just the version number. The version
number alone lies.

### Version source of truth (live-read, never a doc number)

- Authority: `vendor/headroom/crates/headroom-core/Cargo.toml` `version` +
  the parent pin `crates/aphrodite/Cargo.toml` line 55 - they move together
  in one ceremony (bump fork crate + pin together, never one alone). One
  alone is not a bump.
- The §1 ledger's 5 rows are unchanged by the fork; the fork crate rides the
  parent pin as its own authority path. Claim numbers via the availability
  check in `references/headroom-publish.md`; never reuse a burned version.

## Related

- `aphrodite-release-flow` - the ceremony that executes these gates
  (promotion, hotfix, tag, sync). This skill never duplicates its steps.
- `aphrodite-boundaries` - stop conditions, approval boundaries, failure
  policy, git repair taxonomy.
- `aphrodite-orientation` - the mandatory preflight gate.
- `.hermes/governance/VERIFICATION-MATRIX.md` - carries the mirror version
  ledger rows; this skill owns them.
- `.hermes/release/RELEASE-TEMPLATE.md` - the release-notes contract.
- `references/version-ledger-authority.md` - authority paths + 2026-09-25
  snapshot values.
- `references/gate-r7-trigger-audit.md` - worked trigger table (commit
  `a81acab6`).
- `references/artifact-contract.md` - consumer-path matrix.
- `references/release-notes-template.md` - notes heredoc + attach command.
- `references/headroom-publish.md` - fork crate evidence + checklist.

## Claim-to-Test Matrix

| Claim                                                               | Evidence source                                                                                                                                                         | Test                                                                                               | Pass condition                                                                                                                           | Failure response                                                                |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Ledger rows match their authority manifests                         | `crates/aphrodite/Cargo.toml`, `crates/aphrodite-hermes/Cargo.toml`, `plugins/aphrodite/plugin.yaml`, `plugins/aphrodite/BINARY_VERSION`, `package.json`, README badges | Read each authority path at release time                                                           | Each row equals its authority; no drift (the `package.json` 1.4.6-vs-1.5.0 lag is the known failure case)                                | Fix the manifest before claiming; report the drift                              |
| Tag push side effects are exactly as audited                        | Workflow files at the exact tag commit                                                                                                                                  | Gate R7: build the trigger table from the actual files                                             | Only accepted jobs reachable from the tag                                                                                                | Change the workflow or halt the tag                                             |
| Tag push reaches `cargo publish` for `aphrodite`/`aphrodite-hermes` | Publish.yml publish-step `if:` conditions                                                                                                                               | `grep -A3 'Publish to crates.io' .github/workflows/Publish.yml` at the tag commit                  | Conditions include `startsWith(github.ref, 'refs/tags/Aphrodite/')` on both jobs (current state) - or the audit table records the change | Accept the side effect explicitly or halt the tag                               |
| `aphrodite-headroom-core` is never published by a tag push          | Publish.yml Publish-Headroom-Core step                                                                                                                                  | Read the publish-step `if:` at the tag commit                                                      | `workflow_dispatch && inputs.publish_crates && published == 'false'` only                                                                | Treat any tag-reachable headroom publish as an unexpected side effect - stop    |
| Consumer download names match release assets                        | Build.yml staging names + download.sh/download.ps1 asset names                                                                                                          | `gh release view "Aphrodite/v<ver>" --json assets` vs matrix section 4                             | Every required name present (12 assets)                                                                                                  | Do NOT bump `BINARY_VERSION`; fix artifact build/attach                         |
| Missing optional asset degrades, never bricks setup                 | `download.sh` SUMS path, `setup/dylib.rs` optional arm                                                                                                                  | Simulate a missing `SHA256SUMS-<target>.txt` and a missing `libaphrodite.dylib` in a clean install | Warning + continue; setup completes                                                                                                      | Fix the consumer script; re-run the simulation                                  |
| Proposed registry version is available                              | crates.io index/API                                                                                                                                                     | `curl -A <ua> https://crates.io/api/v1/crates/<crate>` / index URL                                 | `max_version` (or `vers`) does not contain the proposed number                                                                           | Claim the next number; never re-publish                                         |
| Release note is publishable                                         | Draft file                                                                                                                                                              | Grep for `{PENDING}` / `DO NOT PUBLISH` / bare compare link                                        | Zero matches; Summary + Changes present                                                                                                  | Fix the note before publish                                                     |
| Every irreversible event pauses for human approval                  | This skill + `aphrodite-boundaries`                                                                                                                                     | Dry-run the 4-event separation with a simulated ceremony                                           | Workflow halts at each `Ready for approval` boundary                                                                                     | Enforce the gate; never chain events in one script                              |
| Fork delta is carried into every release                            | `git -C vendor/headroom log <last-published-commit>..HEAD` + `vendor/headroom/RELEASE-CYCLE.md`                                                                         | Step I5 fork-delta check (`aphrodite-release-flow`)                                                | Delta = 0, or fork crate + parent pin bumped together and fork tag exists before dispatch                                                | Publish the fork with the release; never ship a stale fork version (1.5.0 trap) |