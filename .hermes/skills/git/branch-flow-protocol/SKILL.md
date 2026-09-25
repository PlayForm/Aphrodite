---
name: branch-flow-protocol
description: "Use when designing branch release flows in the Aphrodite monorepo. Development/Current two-line model, submodule gitlink quirks, auto-committer awareness."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: git
category_taxonomy: git/branch-flow-protocol
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, protocol, release, branches, cherry-pick, submodules, invariants]
        related_skills:
            [
                aphrodite-orientation,
                aphrodite-release-workflow,
                git-operations,
                submodule-fleet-management,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
        - Current
owns:
    - The two-line branch protocol (axioms, state machine, invariant set)
    - Submodule gitlink hygiene (float, bump, verify)
    - The release ritual on Current
depends_on:
    - aphrodite-orientation (preflight gate before any ritual)
    - aphrodite-release-workflow (instance release gates, version-sync)
---

# Branch Flow Protocol

The Aphrodite monorepo's release protocol: a two-line flow that keeps one
canonical branch pair per product, moves content by snapshot transplant
and cherry-pick, and treats submodule gitlinks as first-class release
artifacts. Concrete instances declare their topology and identity files
and inherit everything else; the Aphrodite monorepo is the canonical
instance (declared below).

## Lines

| Line        | Role                                              | Writer model                                  | Carries                      |
| ----------- | ------------------------------------------------- | --------------------------------------------- | ---------------------------- |
| Development | Long-lived integration line; all work accumulates | Multi-writer (devs + declared auto-committer) | Full history, picks manifest |
| Current     | Working line; released snapshots and tags         | Single-writer (release ritual only)           | Tags, published artifacts    |

## Axioms (invariant across every instance)

1. **Two lines per product.** A `Development` (integration) line and a
   `Current` (working) line. A third line is a _second independent
   product pair_, never a tier of the same product.
2. **Append-only lines.** No branch is ever rewritten (rebase banned).
   Development accumulates; Current only ever receives snapshot
   transplants.
3. **Content descends by snapshot transplant, ascends by cherry-pick.**
   Bulk (features/releases) moves Development → Current as one curated
   commit; only urgent fixes move Current → Development, each with `-x`
   provenance.
4. **Identity files are branch-owned.** Each line carries its own
   operational identity (workflow triggers, submodule branch fields,
   CI config). These never cross lines during a transplant.
5. **Tags are the only release artifacts.** Tags and published artifacts
   live exclusively on Current. Development never carries release tags.
6. **Versions are monotonic per product.** The bump ships where the
   change ships; picks propagate the bump; a claimed number is never
   reused.
7. **Current is single-writer.** Only the release ritual writes to
   Current; Development is multi-writer. Automated actors (the external
   auto-committer, `.githooks`) are declared, not ambient.

## Submodule topology (Aphrodite)

| Submodule         | Remote | Role          | Line behavior                                      |
| ----------------- | ------ | ------------- | -------------------------------------------------- |
| plugins/aphrodite | Source | First-party   | follows (tracks its own Current; floats on detach) |
| vendor/headroom   | -      | Pinned vendor | pinned (never floats during a release)             |
| vendor/rtk        | -      | Pinned vendor | pinned (never floats during a release)             |

- Submodule gitlinks are auto-bumped by `.githooks` (float-to-configured-
  branch + gitlink bump on the work copy).
- The parent gitlink is recorded in `git ls-tree HEAD <submodule>`;
  verify after every ritual that the parent points at the released
  submodule commit.

## State machine

Every release traverses: `PREPARED → SYNCED → VERIFIED → TAGGED →
PUBLISHED`, with failure transitions:

```
PREPARED ──(work done, bumped, gated)──► SYNCED ──(protected paths
restored, pins floated)──► VERIFIED ──(invariants hold)──► TAGGED
──(tag pushed, release created)──► PUBLISHED

        ┌─ CONFLICT (file changed on both lines) ──► RESOLVED ──► VERIFIED
        │      resolve keeping Current content, pick the
        │      change up immediately (superset rule)
        ├─ DIVERGED (missed pick) ──► PICK-UP ──► SYNCED
        ├─ DIRTY (uncommitted changes) ──► ABORT (never sweep into
        │      the release; auto-committer makes status unstable)
        └─ DETACHED (submodule) ──► FLOAT to configured branch ──► SYNCED
```

## Instance declaration (Aphrodite)

The canonical instance declares:

```yaml
product: Aphrodite
lines:
    integration: Development
    working: Current
tag_prefix: Aphrodite/v
topology:
    plugins/aphrodite: follows # remote 'Source'; tracks its own Current
    vendor/headroom: pinned
    vendor/rtk: pinned
identity_files:
    - .gitmodules
    - .githooks
    - Build.yml
    - Publish.yml
verify: <invariants I1-I10 below>
```

Identity files are the instance's only declaration that differs
materially; the ceremony is identical everywhere.

## Topology rules (multi-dimensional)

- **N submodules**: releases happen bottom-up. Sync any submodule whose
  pins change BEFORE the parent, so the parent gitlink references the
  released submodule commit. Pinned submodules (`vendor/headroom`,
  `vendor/rtk`) never float during a release.
- **Cross-repo ordering**: derived from the dependency graph, never
  hardcoded. Current is independent per product; fleet-wide releases run
  per product with zero interaction.
- **Concurrency**: one release ritual at a time on Current. Development
  accepts concurrent writers. The external auto-committer is a declared
  actor: allowed on Development, forbidden on Current outside the ritual.
- **Release candidates**: prerelease tags (`Aphrodite/vX.Y.Z-rc.N`),
  never branches.

## Invariant predicates (verifier)

Every invariant must be machine-checkable. Canonical set:

- **I1** All repos and submodules on a named branch; `.githooks` float on
  any detach. `git symbolic-ref -q HEAD` succeeds everywhere.
- **I2** No stale gitlinks: `git submodule status` shows no `+`; the
  parent gitlink matches `git ls-tree HEAD <submodule>`.
- **I3** Current tree == tagged released content.
- **I4** Development static: `git log` shows only adds since the fork
  point.
- **I5** Identity files match their line: `.gitmodules` branch fields,
  workflow triggers, CI config all reference the line they live on.
- **I6** Workflow triggers are branch-specific; `Build.yml` / `Publish.yml`
  never fan out to both lines.
- **I7** Every upward pick carries `-x` and a manifest entry.
- **I8** Versions monotonic: `git log --format=%s` bump history shows no
  reuse.
- **I9** Protected paths: `git diff HEAD -- <identity_files>` is empty
  after any transplant.
- **I10** The daily working copy lives on Development; Current is touched
  only inside the ritual and returned-from immediately.

Run the full set after every ritual; any failure is a release blocker.

## Data schemas (audit trail)

- **Sync commits**: `release: sync v<X.Y.Z> from Development` on Current.
- **Picks manifest**: `<current-sha> → <development-sha> | date | reason`
  appended on Development after every upward pick.
- **Tags**: `Aphrodite/v<X.Y.Z>` and `Aphrodite/v<X.Y.Z>-rc.N` only.

## Failure handling

- **Conflict**: resolve keeping Current content; immediately pick the
  change up (superset rule: Development must always contain Current, so
  the next transplant is a clean copy).
- **Diverged**: pick up the missed change with `-x`, re-sync.
- **Dirty**: abort. The auto-committer sweeps working-tree changes, so
  `git status` alone is not stable evidence - capture HEAD and the
  remote tip before starting (orientation gate) and never let a sweeper
  commit unreviewed dirt into the release.
- **Detached submodule**: `.githooks` float to the configured branch and
  bump the parent gitlink; verify with I2.
- **Promotion review**: the `promote/vX.Y.Z` branch is optional review
  scaffolding, not architecture. Direct promotion when review is not
  required; PR branch only when it is.

## Running a release (the ritual)

1. **Orient** (`aphrodite-orientation` gate): verify root, branch,
   submodule state, remotes; capture `HEAD` + remote tip as the
   auto-committer baseline.
2. **Prepare**: complete the change on Development, bump versions, run
   static checks.
3. **Sync**: bottom-up - sync changed submodules first, then transplant
   one curated commit Development → Current; restore protected paths;
   float pins.
4. **Verify**: run I1-I10 in full; check parent gitlinks via
   `git ls-tree`.
5. **Tag & publish**: tag on Current (`Aphrodite/vX.Y.Z`), push tag,
   create release; return the working copy to Development.

## Adopting the protocol

1. Declare the instance (product, lines, tag prefix, topology,
   identity files, verifier).
2. Wire hooks: float-to-configured-branch + gitlink bump on the work
   copy (the Aphrodite `.githooks` are the reference implementation).
3. Write the instance skill as a thin layer over this protocol: state
   the declaration, point at the axioms/state machine/verifier here.
4. Verify the full invariant set after the first ritual.

## Local test matrix

| Claim                                       | Evidence source          | Test                                                     | Pass condition                            | Failure response                |
| ------------------------------------------- | ------------------------ | -------------------------------------------------------- | ----------------------------------------- | ------------------------------- |
| Parent gitlink records the submodule commit | `git ls-tree HEAD <sub>` | Detach + float a submodule via `.githooks`, read gitlink | Parent entry == released submodule SHA    | Fix the hook or verify manually |
| Auto-committer never writes to Current      | `git log Current`        | Run a ritual under an active auto-committer              | No non-ritual commits on Current          | Enforce the single-writer rule  |
| Invariant set catches stale gitlinks        | `git submodule status`   | Modify a submodule, run the verifier                     | I2 fails                                  | Restore the gitlink             |
| Identity files do not cross lines           | `git diff HEAD -- <id>`  | Transplant, then diff identity files                     | Diff empty (I9)                           | Restore branch-owned files      |
| Upward picks carry provenance               | pick commits             | Inspect every upward pick after a ritual                 | All picks have `-x` + manifest entry (I7) | Re-pick with `-x`               |

## Related

- `aphrodite-release-workflow` - instance release gates, version-sync
  locations, release notes standards.
- `aphrodite-orientation` - the preflight gate every ritual starts with.
- `git-operations` / `submodule-fleet-management` - hook mechanics and
  gitlink hygiene.
