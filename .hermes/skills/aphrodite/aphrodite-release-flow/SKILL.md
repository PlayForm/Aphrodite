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
and its plugin submodule S (`plugins/aphrodite` → PlayForm/Aphrodite-Hermes,
remote `Source`). Supersedes `aphrodite-branch-release-flow` (archived,
historical only - do not execute).

## Ownership boundary

- **This skill owns** release promotion, hotfix, tag creation, version
  sync-back, artifact verification.
- `aphrodite-release-workflow` owns the version ledger, artifact contract
  matrix, Gate R7 definitions, 4-event publishing separation - never
  duplicate its tables.
- `aphrodite-boundaries` owns the git repair taxonomy, branch-owned identity
  contract, human approval boundaries.
- `aphrodite-orientation` owns the Orient phase and the mandatory preflight
  gate.

## Topology & identity

- **Development ≠ Current.** Development is the workshop: it accumulates, is
  never rewritten, runs all tests + CI, carries no release tags. Current is
  the distributed line: tags, GitHub releases, and registry publishes happen
  ONLY here. Different products, not mergeable twins.
- **Branch-owned identity ≠ merge convenience.** Protected: `.gitmodules`
  branch fields, `.github/workflows/*` triggers and push targets
  (`[Development]` vs `[Current]`), the `plugins/aphrodite` gitlink. Only a
  contract-declared identity path qualifies for restore. Contract invoked at
  THREE points: I1 (pre-stage), I4 (post-restore), R1 (pre-tag).
- **Binary version ≠ plugin version ≠ `BINARY_VERSION`.** Binary `1.6.x` in
  parent Cargo.tomls + `package.json` + README badge; plugin `2.2.x` in
  `plugins/aphrodite/plugin.yaml`. `BINARY_VERSION` (S) is the live download
  pointer (`download.sh`/`download.ps1` fetch it). Last recorded (CLAIM -
  re-derive live): binary `1.6.2`, plugin `2.2.2`, `BINARY_VERSION` `1.6.2`.
  Drift example: references/compressed-detail.md.
- **Bump order (one ceremony):** parent crates + `aphrodite-hermes` dep pin
  `aphrodite = { path = .., version = "X" }` + `package.json` TOGETHER
  (cargo check fails otherwise); then `plugin.yaml` + `install_message` +
  README badges; `BINARY_VERSION` LAST at tag time. Exemptions + grep:
  references/compressed-detail.md.
- **Gitlink ≠ branch.** A superproject tracks each submodule by gitlink, a
  pinned commit SHA, never a branch. The parent gitlink is branch-owned:
  each branch floats its own - a gitlink-only bump (`chore: bump plugin
  submodule to <sha>`) is never cherry-picked back.
- **Submodule-first ordering is mandatory** in every phase (release AND
  sync-back): plugin first, then parent.
- **File classification:** `.hermes/classification/` (TAXONOMY.md + pass
  files, HPC codes) decides what crosses a transplant; record new removals
  as ceremony rules; re-derive counts live (`find .hermes/classification
  -type f | wc -l`). Legend: references/ceremony-steps.md.

## Unified lifecycle (state machine)

Phase order: Orient → Prepare → Validate → Integrate → Release → Observe;
**Recover** entered from any hard stop. No phase skipped/reordered; each has
entry condition, allowed changes, exit evidence, hard stop (table:
references/ceremony-steps.md).

- Phase transition requires the previous phase's exit evidence.
- Hard stop → Recover (or stop entirely); never "fix forward" past a failed
  verification.
- Every mutation command sits in a step carrying Preconditions, Verify,
  Expected, Stop if, Recovery; a step never silently combines mutation,
  validation, publishing, and recovery.

## Orientation gate (before ANY mutation)

The five read-only commands from `aphrodite-orientation` (`git rev-parse
--show-toplevel`, `git branch --show-current`, `git status --short`, `git
submodule status --recursive`, `git remote -v`), interpreted against scope
(PlayForm/Aphrodite + PlayForm/Aphrodite-Hermes, Development + Current,
source + installed). STOP on dirty/unexpected state.

**Auto-committer baseline (mandatory):** the auto-committer sweeps
working-tree changes into commits and pushes, so `git status` is NOT stable
evidence - verify with `git log`/`git submodule status`, never `git status`.
Capture before the first mutation and re-check at every boundary (B4,
pre-tag, post-sync):

```sh
git rev-parse HEAD
git ls-remote origin <branch>
```

Dirty tree when a sync step is about to run → **abort the sync** (a staged
squash set may be swept mid-ceremony; content usually survives as a commit -
verify with `git log`).

## Phase rules (walkthroughs: references/ceremony-steps.md, references/release-steps.md, references/hotfix.md)

### Prepare (Development)

- Version must be free: a burned crates.io version is gone forever - claim
  the NEXT number; never tag before free. Ledger rows = authority path
  (Step P1).
- Binary track moves in ONE ceremony (cargo check fails if one crate moved
  alone); `BINARY_VERSION` moves LAST at tag time.
- Templates match live config: `plugins/aphrodite/__init__.py` byte-identical
  to `crates/aphrodite/templates/__init__.py` (setup.rs asserts it). Crate
  README links render as blob/HEAD - write absolute to the file's OWN branch.

### Validate

- Record ACTUAL gate output ("406 passed, 0 failed") - never "should pass";
  a red gate voids the release claim.
- Runtime evidence from a FRESH process: a stale dylib is not evidence;
  restart, then re-probe (`aphrodite_stats`/`aphrodite_test`/
  `aphrodite_rebuild`).

### Integrate (Development → Current)

- **B4 branch-identity audit - MANDATORY before ANY sync or tag.** Scans
  workflow triggers + push targets, `.gitmodules` branch fields, gitlink
  resolution - zero hits required; ANY hit ABORTS (record in
  `.hermes/notes/release/CEREMONY-AUDIT.md`; never checkout the other
  branch). Commands: references/ceremony-steps.md Step I1.
- **Submodule-first (bottom-up):** plugin FIRST, then parent; validate the
  plugin commit before the parent gitlink moves; never float the gitlink to
  a non-released plugin commit.
- **Controlled restore is identity-only:** `git checkout HEAD -- .gitmodules
  .github/workflows plugins/aphrodite` - ceremony invariant, NOT repair;
  never blanket checkout/reset.
- **Headroom fork leg (mandatory):** fork delta → fork crate + parent pin
  TOGETHER (cargo check fails otherwise), fork tag BEFORE dispatch
  (`aphrodite-vX.Y.Z`, never `Aphrodite/v*`), gitlink to the TAGGED fork
  commit (CI publishes the parent-recorded gitlink tree). Stale fork version
  → CI skips publish silently (1.5.0 trap). Tracking: references/compressed-detail.md.

### Release

- **4 irreversible events, never combined:** (1) release-sync commit (I4),
  (2) immutable tag (R4), (3) artifacts (R5, Build.yml attaches), (4)
  registry (R6, cargo publish). Each requires identity confirmation, version
  availability, intended list, Gate R7, human approval (`Ready for approval`
  pause), consumer verification.
- **Gate R7 at the exact tag commit:** read the ACTUAL workflow files -
  never trust remembered behavior. Tag push publishes `aphrodite` +
  `aphrodite-hermes` (no already-published check); `aphrodite-headroom-core`
  is dispatch-gated, NOT tag-reachable. Unexpected tag-reachable publish →
  stop. Commands: references/release-steps.md Step R2.
- **Never re-tag; never move the tag.** Immutable evidence on the exact
  release-sync commit; re-tagging re-fires Build/Publish.
- **`BINARY_VERSION` bumps LAST (after assets exist):** a pre-asset bump
  404s every download; `_check_version_published` warning = hard stop.
- **Registry:** the tag push already ran `cargo publish` - never
  re-dispatch; verify via crates.io API `max_version` (commands:
  references/release-steps.md Step R6); failed publish → release a NEW
  version.
- **Artifacts:** Finalize fails loudly on missing assets; 12 = 4 targets ×
  (`aphrodite-<t>` + `libaphrodite_hermes-<t>.{so,dylib,dll}` +
  `SHA256SUMS-<t>.txt`). BODY amendable, tag not; `--notes-file`, never
  inline backticks.

### Observe

- `aphrodite_stats` is ground truth; the banner is NOT - never declare the
  release verified on a banner alone (stale dylib).
- Round trips: `aphrodite_test` (quick=1/full=3), `aphrodite_catalog`,
  `aphrodite_diff`, `aphrodite_directive list`; `catalog` populated + `diff`
  empty is EXPECTED; counter reset after the bump is NORMAL. Terminal output
  is CCR-compressed - scan for `<<<CCR:` and retrieve before reading on.
- Proxies down + inline-only is the user's ACCEPTED state - no proxy-restart
  default fix. Misleading preview = BUG, not cosmetic (references/preview-quality-debugging.md).

### Recover

- Classify by ONE observed symptom first (runbooks:
  references/engine-health-debugging.md, references/plugin-lifecycle.md,
  references/preview-quality-debugging.md); never change more than one
  dimension at a time.
- Repair taxonomy (`aphrodite-boundaries`): wrong content → edit directly;
  identity crossed → restore named protected paths; conflict → resolve +
  marker sweep; empty pick → verify + skip; phantom → remove the indexed
  mode-160000 entry; wrong version → release a new version.
- Destructive shortcuts prohibited: blanket checkout/reset, force-push,
  retag, hook re-creation, phantom-gitlink ignore.
- Behavior ≠ docs → update the canonical owner skill + test matrix.

## Hotfix on Current - the fast path

Worked DIRECTLY on Current - no Development round-trip, no full ceremony.
Same state machine, Current as workspace: Prepare → Validate on Current,
Release on Current, then Integrate = sync-back to Development. The user
drives every commit/tag/push decision; never commit, tag, or push unasked.
Steps H1-H6: references/hotfix.md.

- Claim the NEXT patch number, never a burned one - check crates.io
  `max_version` before the bump (Step H1).
- `BINARY_VERSION` bumps LAST + gitlink floats LAST (rule R3): only after
  release assets exist (tag pushed AND Build completed).
- Run B4 (I1) + Gate R7 (R2) + approval pause (R4) before the tag; a
  cleanup commit does NOT move the tag.
- **Sync-back = selective `git cherry-pick -x`, CHRONOLOGICAL, submodule
  FIRST then parent.** PICK real fixes; SKIP gitlink-only bumps, style-only
  commits, snapshots re-adding removed content or deleting test files (full
  lists: references/compressed-detail.md). Never merge Current wholesale;
  never rebase picks.
- **Empty cherry-pick ≠ error:** `nothing to commit, working tree clean` =
  already contained - `git diff <HEAD> <source> -- <paths>`, skip.
- **Marker sweep after EVERY `--continue`:** a second conflict region gets
  COMMITTED by `--continue`; grep the committed set for `<<<<<<<`, fix +
  `git commit --amend --no-edit` (references/hotfix.md Step H4).
- **Identity re-expression guard (Step H5)** after any gitlink pick:
  controlled restore, then identity diffs empty (I9) and no `+` in `git
  submodule status` - never transplant Current's identity onto Development.
- Verify remote tip, not push output: `git log Source/Development` -
  "Everything up-to-date" is not evidence.

## Pitfalls (mechanics + probes: references/pitfalls.md)

- **Never re-create the git hooks.** `.githooks/` was removed because the
  auto-bump hooks were the resurrection vector for the phantom
  self-referential gitlink (`post-commit`/`post-checkout` re-staged a 160000
  entry; `package.json`'s `prepare` re-installed the hooks). Re-creating any
  hook or `prepare` is a ceremony violation.
- **Never ignore a phantom self-referential gitlink.** mode-160000 entry
  inside the submodule pointing at its OWN commit (no `.gitmodules` there);
  detect: `git -C <submodule> ls-files -s | grep 160000`. Auto-committer
  re-stages a fresh phantom - re-check after EVERY subsequent submodule
  commit; clean check at session end is the pass criterion.
- **Never trust `git status` while the auto-committer runs.** Verify with
  `git log`/`git submodule status`.
- **Removing a subsystem = sweep the whole tree, then record the absence.**
  A removed `#[no_mangle]` export without the dylib expected-symbol-list
  update kills `aphrodite_rebuild` ("missing an expected symbol"). State the
  absence as a ceremony rule.
- **Never let the shim or templates drift.** setup.rs asserts byte-identity
  of `templates/__init__.py` vs the live plugin `__init__.py`; stale
  templates write configs missing engine keys.
- **The setup flow needs a key, not the plugin.** No `APHRODITE_API_KEY` /
  toml `api_key` → "no API key configured"; Hermes' provider config is NOT
  reused (references/plugin-lifecycle.md).

## Related

- `aphrodite-release-workflow` - version ledger, artifact contract matrix,
  Gate R7 definitions, publishing separation.
- `aphrodite-boundaries` - git repair taxonomy, identity contract, approval
  boundaries.
- `aphrodite-orientation` - mandatory preflight gate; owns Orient.
- `.hermes/notes/release/RELEASE-METHODOLOGY.md` - canonical ceremony +
  invariants I1-I11 (I9 protected-paths, I11 branch-identity) + `### B4`.
- `.hermes/notes/CEREMONY.md` - condensed ceremony spec (Phase A/B, B4).
- `.hermes/governance/VERIFICATION-MATRIX.md` - global claims → probes.
- `vendor/headroom/CHANGELOG.md` + `vendor/headroom/RELEASE-CYCLE.md` - fork
  ledger / tracking contract (updated per cycle in Step I5).
- references/ceremony-steps.md - lifecycle table, classification legend,
  P1-P3, V1-V2, I1-I5.
- references/release-steps.md - R1-R6 (Gate R7 snapshot), O1-O3, X1-X3.
- references/hotfix.md - H1-H6.
- references/pitfalls.md - scar catalog with probes.
- references/headroom-publish.md - fork dispatch + verification.
- references/plugin-lifecycle.md - install layouts, key sourcing.
- references/engine-health-debugging.md - engine probe battery (runbook).
- references/preview-quality-debugging.md - preview defect classes (runbook).
- references/compressed-detail.md - worked detail moved out of SKILL.md.
- `aphrodite-branch-release-flow` - superseded; archived; **Historical
  only - do not execute**.

## Claim-to-Test Matrix

| Claim | Evidence source | Test | Pass condition | Failure response |
| --- | --- | --- | --- | --- |
| Tag push side effects are exactly as audited | Workflow files at the exact tag commit | Gate R7: build the trigger table from the actual files at the tag commit | Only accepted jobs reachable from the tag | Change the workflow or halt the tag |
| Tag push publishes `aphrodite` + `aphrodite-hermes` crates | Publish.yml publish-step `if:` conditions at the tag commit | `git show <tag>:.github/workflows/Publish.yml \| grep -n 'startsWith(github.ref'` | Both publish steps carry `\|\| startsWith(github.ref, 'refs/tags/Aphrodite/')` (current state) or the audit records the change | Accept the side effect explicitly or halt the tag |
| `aphrodite-headroom-core` is never tag-published | Publish.yml Publish-Headroom-Core publish-step `if:` | Read the publish-step `if:` at the tag commit | `workflow_dispatch && publish_crates && published == 'false'` only | Treat any tag-reachable headroom publish as unexpected - stop |
| Identity never crosses a transplant | `.gitmodules`, workflow triggers, plugin gitlink | B4 scan + I9 diff at the 3 contract points (I1, I4, R1) | Zero identity hits; identity diffs empty | ABORT the ceremony; fix the offending branch |
| Auto.yml heartbeat does not leak (audited at `a81acab6`) | Auto.yml `branch:` on both refs | B4 scan step 1 | Each copy pushes its OWN branch | Fix the leaking copy; re-run B4 |
| `BINARY_VERSION` moves LAST | `plugins/aphrodite/BINARY_VERSION` + release asset list | Compare bump-commit order against tag + Build completion; `_check_version_published` silent | Bump commit is after tag/assets; no download pointer to missing assets | Defer the bump; never tag with a pointer to missing assets |
| Plugin commit validated before parent gitlink update | Plugin gitlink + fresh-process probe | Step I3: load the plugin commit in a fresh process, version pair check | Plugin loads; `aphrodite_rebuild` matches | Fix the plugin commit; do not float the gitlink |
| Empty cherry-pick is already-contained | `git diff <HEAD> <source> -- <paths>` | Step H4 | Diff empty → skip the commit | Treat as success, not failure |
| Cherry-pick never commits leftover markers | Staged file set after every `--continue` | `git diff HEAD~1 HEAD --name-only \| xargs grep -l '<<<<<<<'` | Zero matches | Fix + `git commit --amend --no-edit` |
| Protected paths unchanged after any transplant | `git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite` | I9 at post-restore (I4) + post-sync (H5) | Empty | Controlled restore; never blanket checkout |
| Runtime evidence matches the release claim | `aphrodite_stats` / `aphrodite_rebuild` / `aphrodite_test` | Observe phase battery | Versions agree; round trips pass; previews honest | Stale dylib → restart + re-probe; classify preview defects before touching code |
| Every irreversible event pauses for human approval | Steps R4-R6 + `aphrodite-boundaries` | Dry-run the 4-event separation with a simulated ceremony | Workflow halts at each `Ready for approval` boundary | Enforce the gate; never chain events in one script |
| `.githooks` stay removed | `git config core.hooksPath`; `ls .githooks`; `package.json` prepare | Grep parent + submodule for hooks/prepare remnants | Unset/absent everywhere; no phantom vector re-introduced | Re-create nothing; report the violation |
| Version numbers match their ledger authority | Cargo.tomls, dep pin, `package.json`, `plugin.yaml`, `BINARY_VERSION`, gitlink | Step P1 ledger read | Each row equals its authority; known `package.json` lag recorded | Fix the manifest before claiming; report the drift |
| Fork delta is carried into every release | `git -C vendor/headroom log <last-published-commit>..HEAD` + `vendor/headroom/RELEASE-CYCLE.md` | Step I5 fork-delta check | Delta = 0, or fork crate + parent pin bumped together, fork tag before dispatch | Publish the fork with the release; never ship a stale fork version (1.5.0 trap) |