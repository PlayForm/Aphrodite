---
name: aphrodite-release-flow
description: "Use when releasing or hotfixing Aphrodite (parent + plugin submodule), or verifying the live CCR engine after a bump. Sole owner of the release/hotfix/tag/version-sync ceremony."
version: 2.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-release-flow
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, release, branch, cherry-pick, submodule, hotfix, state-machine]
        related_skills: [aphrodite-boundaries, aphrodite-orientation, aphrodite-release-workflow]
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
    - Release promotion (Development -> Current snapshot transplant)
    - Hotfix on Current (direct fix + cherry-pick -x sync-back)
    - Tag creation (immutable, Current-only, after the exact release-sync commit)
    - Version sync-back (selective cherry-pick Current -> Development)
    - Artifact verification (post-release consumer-perspective checks)
depends_on:
    - aphrodite-boundaries (git repair taxonomy, branch-owned identity contract, approval boundaries)
    - aphrodite-orientation (mandatory preflight gate; owns the Orient phase)
    - aphrodite-release-workflow (version ledger, artifact contract matrix, Gate R7 definitions, publishing separation)
supersedes: []
verification:
    source_of_truth:
        - Workflow files at the exact release commit (.github/workflows/*.yml)
        - Runtime probes (aphrodite_stats / aphrodite_test / aphrodite_rebuild)
mutation_level: publish
---

# Aphrodite Release Flow

Sole owner of the release/hotfix ceremony for PlayForm/Aphrodite (parent P)

- plugin submodule S (`plugins/aphrodite` → PlayForm/Aphrodite-Hermes,
  remote `Source`). Supersedes `aphrodite-branch-release-flow` (archived -
  historical only, do not execute).

## Ownership boundary

- **This skill owns** release promotion, hotfix, tag creation, version sync-back, artifact verification.
- `aphrodite-release-workflow`: version ledger, artifact contract matrix, Gate R7 definitions, 4-event publishing separation - never duplicate its tables.
- `aphrodite-boundaries`: git repair taxonomy, branch-owned identity contract, approval boundaries.
- `aphrodite-orientation`: Orient phase + mandatory preflight gate.

## Topology & identity

- **Development ≠ Current.** Development is the workshop: it accumulates, is never rewritten, runs all tests + CI, carries no release tags. Current is the distributed line: tags, GitHub releases, and registry publishes happen ONLY here. Different products, not mergeable twins.
- **Branch-owned identity ≠ merge convenience.** Protected: `.gitmodules` branch fields, `.github/workflows/*` triggers and push targets (`[Development]` vs `[Current]`), the `plugins/aphrodite` gitlink. Only a contract-declared identity path qualifies for restore. Contract invoked at THREE points: I1 (pre-stage), I4 (post-restore), R1 (pre-tag).
- **Binary version ≠ plugin version ≠ `BINARY_VERSION`.** Binary `1.6.x` in parent Cargo.tomls + `package.json` + README badge; plugin `2.2.x` in `plugins/aphrodite/plugin.yaml`. `BINARY_VERSION` (S) is the live download pointer (`download.sh`/`download.ps1` fetch it). Last-recorded claim + drift example: references/compressed-detail.md.
- **Bump order (one ceremony):** parent crates + `aphrodite-hermes` dep pin `aphrodite = { path = .., version = "X" }` + `package.json` TOGETHER (cargo check fails otherwise); then `plugin.yaml` + `install_message` + README badges; `BINARY_VERSION` LAST at tag time. Exemptions + grep: references/compressed-detail.md.
- **Gitlink ≠ branch.** A superproject tracks each submodule by gitlink, a pinned commit SHA, never a branch. The parent gitlink is branch-owned: each branch floats its own - a gitlink-only bump (`chore: bump plugin submodule to <sha>`) is never cherry-picked back.
- **Submodule-first ordering is mandatory** in every phase (release AND sync-back): plugin first, then parent.
- **File classification:** `.hermes/classification/` (TAXONOMY.md + pass files, HPC codes) decides what crosses a transplant; re-derive counts live (`find .hermes/classification -type f | wc -l`). Legend: references/ceremony-steps.md.

## Unified lifecycle (state machine)

Phase order: Orient → Prepare → Validate → Integrate → Release → Observe; **Recover** entered from any hard stop. No phase skipped/reordered; each has entry condition, allowed changes, exit evidence, hard stop (table: references/ceremony-steps.md).

- Phase transition requires the previous phase's exit evidence; hard stop → Recover or stop entirely - never "fix forward" past a failed verification.
- Every mutation command sits in a step carrying Preconditions, Verify, Expected, Stop if, Recovery; a step never silently combines mutation, validation, publishing, and recovery.

## Orientation gate (before ANY mutation)

The five read-only commands (`git rev-parse --show-toplevel`, `git branch --show-current`, `git status --short`, `git submodule status --recursive`, `git remote -v`); STOP on dirty/unexpected state.

**Auto-committer baseline (mandatory):** `git status` is NOT stable evidence; verify with `git log`/`git submodule status`; capture before the first mutation, re-check at every boundary (B4, pre-tag, post-sync):

```sh
git rev-parse HEAD
git ls-remote origin <branch>
```

Dirty tree when a sync step is about to run → **abort the sync** (a staged squash set may be swept mid-ceremony; content usually survives).

## Phase rules

Each phase: entry condition, allowed changes, exit evidence, hard stop; every step carries Preconditions, Verify, Expected, Stop if, Recovery. Walkthroughs: references/ceremony-steps.md (P1-P3, V1-V2, I1-I5), references/release-steps.md (R1-R6, O1-O3, X1-X3); full text: references/compressed-detail.md.

- **Prepare:** version must be free - claim the NEXT number, never tag before free (a burned crates.io version is gone forever); ledger rows = authority path; binary track in ONE ceremony (cargo check fails if one crate moved alone); `BINARY_VERSION` moves LAST at tag time; templates byte-identical (setup.rs asserts).
- **Validate:** record ACTUAL gate output ("406 passed, 0 failed") - never "should pass"; a red gate voids the claim; fresh-process runtime evidence only (stale dylib is not evidence - restart, then re-probe `aphrodite_stats`/`aphrodite_test`/`aphrodite_rebuild`).
- **Integrate:** B4 branch-identity audit MANDATORY before ANY sync or tag - zero identity hits (workflow triggers + push targets, `.gitmodules` branch fields, gitlink resolution); ANY hit ABORTS (record `.hermes/notes/release/CEREMONY-AUDIT.md`; never checkout the other branch); submodule-first (plugin FIRST, then parent); controlled restore identity-only (`git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite`); headroom fork leg (`aphrodite-vX.Y.Z`, never `Aphrodite/v*`, 1.5.0 trap).
- **Release:** 4 irreversible events never combined - (1) release-sync commit (I4), (2) immutable tag (R4), (3) artifacts (R5), (4) registry (R6, cargo publish); Gate R7 at the exact tag commit - read the ACTUAL workflow files, never trust remembered behavior; never re-tag; `BINARY_VERSION` bumps LAST (after assets exist; `_check_version_published` warning = hard stop); registry verify via crates.io `max_version`; Finalize fails loudly on missing assets; BODY amendable, tag not; `--notes-file`.
- **Observe:** `aphrodite_stats` is ground truth - the banner is NOT (stale dylib); round trips: `aphrodite_test` (quick=1/full=3), `aphrodite_catalog`, `aphrodite_diff`, `aphrodite_directive list`; CCR-compressed output: scan for `<<<CCR:` and retrieve before reading on; preview bug ≠ cosmetic.
- **Recover:** classify by ONE observed symptom first; never change more than one dimension at a time; repair taxonomy (`aphrodite-boundaries`): wrong content → edit directly, identity crossed → restore protected paths, conflict → resolve + marker sweep, empty pick → verify + skip, phantom → remove indexed mode-160000 entry, wrong version → release a new version; destructive shortcuts prohibited (blanket checkout/reset, force-push, retag, hook re-creation, phantom-gitlink ignore); behavior ≠ docs → update owner skill + test matrix.

## Hotfix on Current - the fast path

Worked DIRECTLY on Current - no Development round-trip, no full ceremony; same state machine, Current as workspace; Integrate = sync-back to Development. The user drives every commit/tag/push decision; never commit, tag, or push unasked. Walkthrough H1-H6: references/hotfix.md.

- Claim the NEXT patch number, never a burned one - crates.io `max_version` first (H1); `BINARY_VERSION` + gitlink float LAST (rule R3); B4 (I1) + Gate R7 (R2) + approval pause (R4) before the tag; a cleanup commit does NOT move the tag; verify remote tip, not push output: `git log Source/Development` (H2-H6).
- **Sync-back = selective `git cherry-pick -x`, CHRONOLOGICAL, submodule FIRST then parent.** PICK real fixes; SKIP gitlink-only bumps, style-only commits, snapshots re-adding removed content/deleting test files (full lists: references/compressed-detail.md). Never merge Current wholesale; never rebase picks (H4).
- **Empty cherry-pick ≠ error:** `nothing to commit, working tree clean` = already contained - `git diff <HEAD> <source> -- <paths>`, skip; **marker sweep after EVERY `--continue`** - grep for `<<<<<<<`, fix + `git commit --amend --no-edit`; **identity re-expression guard (H5)** - controlled restore, identity diffs empty (I9), no `+` in `git submodule status`.

## Pitfalls (mechanics + probes: references/pitfalls.md)

- **Never re-create the git hooks** - `.githooks/` removed; the auto-bump hooks were the phantom-gitlink resurrection vector; re-creating any hook or `prepare` is a ceremony violation.
- **Never ignore a phantom self-referential gitlink** - mode-160000 entry inside the submodule pointing at its OWN commit; detect `git -C <submodule> ls-files -s | grep 160000`; auto-committer re-stages a fresh phantom - re-check after EVERY subsequent submodule commit.
- **Never trust `git status` while the auto-committer runs** - verify with `git log`/`git submodule status`.
- **Removing a subsystem = sweep the whole tree, then record the absence** - a removed `#[no_mangle]` export without the dylib expected-symbol-list update kills `aphrodite_rebuild` ("missing an expected symbol"); state the absence as a ceremony rule.
- **Never let the shim or templates drift** - setup.rs asserts byte-identity of `templates/__init__.py` vs the live plugin `__init__.py`; stale templates write configs missing engine keys.
- **The setup flow needs a key, not the plugin** - no `APHRODITE_API_KEY`/toml `api_key` → "no API key configured"; Hermes' provider config is NOT reused (references/plugin-lifecycle.md).

## Related

- Notes: `.hermes/notes/release/RELEASE-METHODOLOGY.md` (invariants I1-I11 + `### B4`); `.hermes/notes/CEREMONY.md` (condensed spec); `.hermes/governance/VERIFICATION-MATRIX.md` (global claims → probes).
- Vendor: `vendor/headroom/CHANGELOG.md` + `vendor/headroom/RELEASE-CYCLE.md` (fork ledger; updated per cycle in Step I5).
- References: ceremony-steps.md (lifecycle table, classification legend, P1-P3, V1-V2, I1-I5); release-steps.md (R1-R6, Gate R7 snapshot, O1-O3, X1-X3); hotfix.md (H1-H6); pitfalls.md (scar catalog with probes); headroom-publish.md (fork dispatch + verification); plugin-lifecycle.md (install layouts, key sourcing); engine-health-debugging.md (engine probe battery); preview-quality-debugging.md (preview defect classes); compressed-detail.md (worked detail moved out of SKILL.md).
- `aphrodite-branch-release-flow` - superseded; archived; **Historical only - do not execute**.

## Claim-to-Test Matrix

| Claim                                                      | Evidence source                                                                                 | Test                                                                                        | Pass condition                                                                                                                 | Failure response                                                                |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| Tag push side effects are exactly as audited               | Workflow files at the exact tag commit                                                          | Gate R7: build the trigger table from the actual files at the tag commit                    | Only accepted jobs reachable from the tag                                                                                      | Change the workflow or halt the tag                                             |
| Tag push publishes `aphrodite` + `aphrodite-hermes` crates | Publish.yml publish-step `if:` conditions at the tag commit                                     | `git show <tag>:.github/workflows/Publish.yml \| grep -n 'startsWith(github.ref'`           | Both publish steps carry `\|\| startsWith(github.ref, 'refs/tags/Aphrodite/')` (current state) or the audit records the change | Accept the side effect explicitly or halt the tag                               |
| `aphrodite-headroom-core` is never tag-published           | Publish.yml Publish-Headroom-Core publish-step `if:`                                            | Read the publish-step `if:` at the tag commit                                               | `workflow_dispatch && publish_crates && published == 'false'` only                                                             | Treat any tag-reachable headroom publish as unexpected - stop                   |
| Identity never crosses a transplant                        | `.gitmodules`, workflow triggers, plugin gitlink                                                | B4 scan + I9 diff at the 3 contract points (I1, I4, R1)                                     | Zero identity hits; identity diffs empty                                                                                       | ABORT the ceremony; fix the offending branch                                    |
| Auto.yml heartbeat does not leak (audited at `a81acab6`)   | Auto.yml `branch:` on both refs                                                                 | B4 scan step 1                                                                              | Each copy pushes its OWN branch                                                                                                | Fix the leaking copy; re-run B4                                                 |
| `BINARY_VERSION` moves LAST                                | `plugins/aphrodite/BINARY_VERSION` + release asset list                                         | Compare bump-commit order against tag + Build completion; `_check_version_published` silent | Bump commit is after tag/assets; no download pointer to missing assets                                                         | Defer the bump; never tag with a pointer to missing assets                      |
| Plugin commit validated before parent gitlink update       | Plugin gitlink + fresh-process probe                                                            | Step I3: load the plugin commit in a fresh process, version pair check                      | Plugin loads; `aphrodite_rebuild` matches                                                                                      | Fix the plugin commit; do not float the gitlink                                 |
| Empty cherry-pick is already-contained                     | `git diff <HEAD> <source> -- <paths>`                                                           | Step H4                                                                                     | Diff empty → skip the commit                                                                                                   | Treat as success, not failure                                                   |
| Cherry-pick never commits leftover markers                 | Staged file set after every `--continue`                                                        | `git diff HEAD~1 HEAD --name-only \| xargs grep -l '<<<<<<<'`                               | Zero matches                                                                                                                   | Fix + `git commit --amend --no-edit`                                            |
| Protected paths unchanged after any transplant             | `git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite`                              | I9 at post-restore (I4) + post-sync (H5)                                                    | Empty                                                                                                                          | Controlled restore; never blanket checkout                                      |
| Runtime evidence matches the release claim                 | `aphrodite_stats` / `aphrodite_rebuild` / `aphrodite_test`                                      | Observe phase battery                                                                       | Versions agree; round trips pass; previews honest                                                                              | Stale dylib → restart + re-probe; classify preview defects before touching code |
| Every irreversible event pauses for human approval         | Steps R4-R6 + `aphrodite-boundaries`                                                            | Dry-run the 4-event separation with a simulated ceremony                                    | Workflow halts at each `Ready for approval` boundary                                                                           | Enforce the gate; never chain events in one script                              |
| `.githooks` stay removed                                   | `git config core.hooksPath`; `ls .githooks`; `package.json` prepare                             | Grep parent + submodule for hooks/prepare remnants                                          | Unset/absent everywhere; no phantom vector re-introduced                                                                       | Re-create nothing; report the violation                                         |
| Version numbers match their ledger authority               | Cargo.tomls, dep pin, `package.json`, `plugin.yaml`, `BINARY_VERSION`, gitlink                  | Step P1 ledger read                                                                         | Each row equals its authority; known `package.json` lag recorded                                                               | Fix the manifest before claiming; report the drift                              |
| Fork delta is carried into every release                   | `git -C vendor/headroom log <last-published-commit>..HEAD` + `vendor/headroom/RELEASE-CYCLE.md` | Step I5 fork-delta check                                                                    | Delta = 0, or fork crate + parent pin bumped together, fork tag before dispatch                                                | Publish the fork with the release; never ship a stale fork version (1.5.0 trap) |
