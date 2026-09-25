# Release, Observe, Recover - step walkthroughs

Full step walkthroughs for Phase Release (R1-R6), Phase Observe (O1-O3), and
Phase Recover (X1-X3). The publishing separation is MANDATORY - four
irreversible events, never combined in one opaque script invocation:
(1) create the release-sync commit (Step I4), (2) create and push the
immutable tag (Step R4), (3) publish or attach binary artifacts (Step R5,
Build.yml attaches), (4) publish immutable registry packages (Step R6,
cargo publish). Each irreversible event requires ALL of: identity
confirmation, version availability check, intended artifact/package list,
trigger audit (Gate R7), explicit human approval (`Ready for approval`
pause), and post-event consumer-perspective verification.

## Phase Release - tag, artifacts, registry

Entry: Current line contains EXACT release content. Exit: tag, artifacts,
consumer verification. Allowed: tag, artifact build, deliberate publish.
Hard stop: existing version, missing assets, wrong trigger.

### Step R1 - Re-run B4 + identity contract (third point: before tag creation)

**Purpose:** Prove identity is still clean and the tag will be created on the
exact release-sync commit.

**Preconditions**

- Step I4 pushed; the release-sync commit hash recorded.

**Do**

```sh
git fetch Source
git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Development Source/Current -- .github/workflows
git show Source/Development:.gitmodules | grep -E '^branch'
git show Source/Current:.gitmodules | grep -E '^branch'
git diff -- .gitmodules .github/workflows plugins/aphrodite < release-sync-commit > HEAD
```

**Verify**

- B4 scan clean; identity diff empty against the release-sync commit.

**Expected**

- Zero identity hits; the tag's target commit is the exact release-sync
  commit (a later style/cleanup tip must NOT move the tag - historical
  example: plugin v2.1.3 sat on its `release: sync v2.1.3` commit, not the
  reformat commit that followed it; version numbers and commits are read from
  git history at audit time, never assumed).

**Stop if**

- Any identity hit; the tag target is not the release-sync commit; the tree
  moved after the sync.

**Recovery**

- Permitted: fix the offending branch; re-run.
- Prohibited: creating the tag anywhere but the exact release-sync commit;
  re-tagging (re-fires Build/Publish).

**Produces**

- Pre-tag B4 + identity evidence.

### Step R2 - Gate R7: trigger audit at the exact commit to be tagged

**Purpose:** Convert implementation-dependent tag side effects into a
deliberate release decision. Definitions and template:
`aphrodite-release-workflow` §2. **Read the workflow files at the exact
commit to be tagged - never trust remembered or documented behavior.**

**Preconditions**

- Step R1 passed; tag commit identified.

**Do**

```sh
# Read the ACTUAL workflow files at the tag commit:
git show < release-sync-commit > :.github/workflows/Publish.yml | grep -nE '^on:|tags:|workflow_dispatch|publish_crates|if:'
git show < release-sync-commit > :.github/workflows/Build.yml | grep -nE '^on:|tags:|workflow_dispatch|if:'
git show < release-sync-commit > :.github/workflows/Check.yml | grep -nE '^on:|tags:|branches:'
git show < release-sync-commit > :.github/workflows/Auto.yml | grep -nE '^on:|schedule|branch:'
# build the trigger table: event -> workflows -> jobs -> publishing side effects
```

**Record**

- Which workflows trigger from this tag; which jobs publish GitHub assets;
  which jobs publish crates/packages; required secrets and manual inputs.

**Evidence snapshot (commit `a81acab6` - evidence, NOT a substitute for the
audit at tag time):**

| Event                                                | Triggered workflows        | Publishing side effects                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Tag push `Aphrodite/v*`                              | `Build.yml`, `Publish.yml` | Build.yml: GitHub release created + 12 assets attached (Release → Build×4 → Finalize). Publish.yml: **`cargo publish -p aphrodite`** and **`cargo publish -p aphrodite-hermes`** - no already-published version check, so a never-published version IS published by the tag push alone; `aphrodite-headroom-core` is dispatch-gated, NOT tag-reachable |
| `workflow_dispatch` (default `publish_crates=false`) | `Build.yml`, `Publish.yml` | Build-only; publish steps skipped (Build release/attach steps and Finalize are tag-gated)                                                                                                                                                                                                                                                              |
| `workflow_dispatch` with `publish_crates=true`       | `Build.yml`, `Publish.yml` | `aphrodite-headroom-core` (index-checked, skipped if already live) → `aphrodite` → `aphrodite-hermes`, hard `needs:` chain                                                                                                                                                                                                                             |

Full table (jobs, secrets, Release-environment approval boundary):
`aphrodite-release-workflow` §2.

**Pass**

- The release owner has EXPLICITLY accepted every triggered side effect -
  including that the tag push publishes `aphrodite` + `aphrodite-hermes` to
  crates.io (Publish jobs run in `environment: Release`; a required-reviewer
  rule there is a workflow-level approval boundary, not a substitute for this
  audit).

**Stop if**

- ANY unexpected publish job is reachable from the tag (e.g. a workflow
  change made `aphrodite-headroom-core` tag-reachable, or a new workflow
  publishes something the operator did not list).

**Recovery**

- Permitted: change the workflow gates at the tag commit and re-audit; or
  halt the tag.
- Prohibited: tagging with an unaccepted publish side effect.

**Produces**

- The trigger table for THIS tag (recorded in the approval summary).

### Step R3 - BINARY_VERSION bump LAST + gitlink float (after assets exist)

**Purpose:** Move the live distribution pointer only after the named release
assets exist - a bump ahead of asset availability 404s every download.

**Preconditions**

- Step R2 passed; the release-sync commit is final (no post-tag edits).

**Do**

```sh
# in the plugin submodule:
echo "<binary-version>" > plugins/aphrodite/BINARY_VERSION
git -C plugins/aphrodite commit -m "chore: bump BINARY_VERSION to <v>"
git -C plugins/aphrodite push Source Current
# float the parent gitlink to that plugin commit (LAST commits of the cycle):
git -C plugins/aphrodite checkout <plugin-commit>
git add plugins/aphrodite && git commit -m "chore: float plugin submodule to <sha>"
git push origin Current
```

**Verify**

```sh
git submodule status plugins/aphrodite # no '+'
git ls-tree HEAD plugins/aphrodite     # intended commit
```

**Expected**

- `BINARY_VERSION` names the release being tagged; the gitlink points at the
  plugin commit carrying it.

**Stop if**

- The release assets do not exist yet (verify with `gh release view --json
assets`, not optimism).
- The plugin's `_check_version_published` warns that the pinned version has
  no assets - treat that warning as a hard stop for tagging.

**Recovery**

- Permitted: defer the bump until assets exist; fix the file directly.
- Prohibited: bumping `BINARY_VERSION` before tag + Build completion;
  reusing a claimed version.

**Produces**

- Plugin `BINARY_VERSION` bump commit + parent gitlink float.

### Step R4 - Event 2: create and push the immutable tag (approval pause)

**Purpose:** Freeze history at the exact release-sync commit. The tag is
immutable historical evidence.

**Preconditions**

- Steps R1-R3 passed; every gate recorded; approval summary assembled.

**Do**

```text
Ready for approval:
- Current commit: <sha>
- Proposed tag: <tag>
- Plugin commit: <sha>
- Expected triggered workflows: <list from Gate R7>
- Expected external publications: <list from Gate R7>
- Verification gates passed: <list>
- Known degraded conditions: <list or none>
```

**Verify**

```sh
git tag -n1 <tag>                      # points at the release-sync commit
git ls-remote --tags origin <tag>      # tag does NOT exist remotely yet
```

**Expected**

- Human approval given; the tag is created on the exact release-sync commit
  and pushed.

**Stop if**

- The tag already exists (never re-tag - it re-fires Build/Publish and
  rewrites history's evidence).
- The tag would point anywhere but the release-sync commit.

**Recovery**

- Permitted: delete a LOCAL, never-pushed tag; fix and re-create before push.
- Prohibited: force-moving or deleting a pushed tag; re-tagging to "fix" a
  release.

**Produces**

- The immutable tag (pushed).

### Step R5 - Event 3: artifact attach + consumer-perspective verification

**Purpose:** Confirm the 12 release assets land and every consumer-required
name resolves - a successful build must not become a failed first-run setup.

**Preconditions**

- Step R4 pushed; Build.yml fired on the tag.

**Do**

```sh
# Build.yml auto-attaches: the Release job creates the release exactly once,
# the 4-matrix Build jobs attach their assets, and Finalize fails loudly if
# any of the 12 expected assets is missing (the old "no Windows release"
# timing race is killed by this job):
gh release view "<tag>" --repo PlayForm/Aphrodite --json assets -q '.assets[].name'
# clean-install simulation (Option A) or asset-name verification (Option B):
# matrix + commands: aphrodite-release-workflow §4
```

**Verify**

- All 12 assets present: 4 targets × (`aphrodite-<t>` +
  `libaphrodite_hermes-<t>.{so,dylib,dll}` + `SHA256SUMS-<t>.txt`).
- Optional artifacts (`libaphrodite.dylib` core cdylib, `SHA256SUMS-<t>.txt`
  on some consumer paths) degrade with a warning, never brick setup.

**Expected**

- Every consumer-required name in the artifact contract matrix
  (`aphrodite-release-workflow` §4) matches a published asset; checksums
  verify.

**Stop if**

- `Finalize` reports a missing target's assets; the release body was authored
  with inline backtick notes (shell command substitution - always
  `--notes-file`, see `aphrodite-release-workflow` §5).

**Recovery**

- Permitted: fix the failing build leg and let the matrix re-run; amend the
  release BODY (amendable) - never the tag.
- Prohibited: declaring the release complete with an incomplete matrix;
  bumping `BINARY_VERSION` pre-assets.

**Produces**

- Artifact evidence (asset list) from the consumer's perspective.

### Step R6 - Event 4: registry publish + consumer verification

**Purpose:** Confirm the immutable crates.io packages exist and resolve -
from the consumer's perspective, not the publisher's.

**Preconditions**

- Step R4 pushed; Gate R7 accepted the tag-reachable publish steps.

**Do**

```sh
# The tag push already ran cargo publish (per the accepted trigger audit) -
# do NOT re-dispatch for aphrodite / aphrodite-hermes; verify instead:
curl -A < ua > https://crates.io/api/v1/crates/aphrodite | grep max_version
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-hermes | grep max_version
# IF Step I5 carried a fork delta, dispatch the headroom publish now - the
# fork leg (crate + parent pin bumped, fork tag + gitlink float) MUST have
# run first, else the version check skips the stale version silently (the
# 1.5.0 published-version trap); needs chain: Test -> Publish-Headroom-Core
# -> Publish-Aphrodite -> Publish-Hermes (headroom publishes FIRST):
gh workflow run Publish -f publish_crates=true
# then verify the headroom publish landed (see references/headroom-publish.md):
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

**Verify**

- The crates.io index/API serves the new versions; a consumer `cargo add` /
  `cargo install` resolves them. For headroom: the index must serve the NEW
  fork version - seeing only the old 0.1.2 means the skip fired again
  (references/headroom-publish.md "Post-event consumer verification").

**Expected**

- `max_version` == the released version for the published crates; the
  headroom dispatch happens only when Step I5 carried a fork delta -
  deliberate, never accidental.

**Stop if**

- A publish failed (re-publish errors red - no already-published check); an
  UNACCEPTED crate was published; headroom-core published without dispatch.

**Recovery**

- Permitted: for a failed publish, fix and release a NEW version.
- Prohibited: re-publishing a burned version; re-tagging to trigger again.

**Produces**

- Registry verification evidence (release phase exit).

## Phase Observe - live-engine verification (post-release)

Entry: release or runtime change complete. Exit: engine, retrieve, search,
preview behavior verified. Allowed: health and round-trip checks. Hard stop:
diagnostics disagree with the release claim.

### Step O1 - Ground truth via aphrodite_stats, not the banner

**Purpose:** The plugin's orientation banner is NOT ground truth;
`aphrodite_stats` is.

**Do**

```sh
aphrodite_stats   # version, engine_enabled, thresholds, proxy liveness
aphrodite_rebuild # binary vs plugin version cross-check (must match)
```

**Expected**

- Stats version equals the released binary; engine enabled; `rebuild`
  agrees.

**Stop if**

- Diagnostics disagree with the release claim (banner says new, stats say
  old - stale dylib).

**Recovery**

- Permitted: restart the session to load the new dylib; re-probe.
- Prohibited: declaring the release verified on a banner alone.

**Produces**

- Post-release runtime evidence.

### Step O2 - Round-trip battery

**Do**

```sh
aphrodite_test # quick=1 sample; full=3: source_code/build/json
aphrodite_catalog
aphrodite_diff
aphrodite_directive list
```

**Expected**

- Compress → retrieve → search round trips pass. `catalog` populated +
  `diff` empty is EXPECTED (the turn-history layer tracks only
  conversation-turn compression; test-generated entries store at turn 1 but
  never register as turns). `files` empty is expected until the session reads
  a file. Session counters resetting after the bump is normal.
- Terminal output is CCR-compressed too: scan terminal results for
  `<<<CCR:` markers and retrieve them before reading on.

**Stop if**

- A round trip fails; a marker does not resolve; a preview misstates shape
  or size.

**Recovery**

- Permitted: bounded repairs from the reference runbooks
  (`references/engine-health-debugging.md`,
  `references/preview-quality-debugging.md`).
- Prohibited: "fixing" the engine because a preview lied, without first
  classifying the defect (runbook step 0).

**Produces**

- Round-trip + catalog evidence.

### Step O3 - Accept the degraded-mode contract

**Expected**

- Proxies down + inline-only compression is the user's ACCEPTED state; do
  not offer proxy restarts as the default fix. Missing API key → clear
  degraded-state message (key sourcing: `references/plugin-lifecycle.md`).
- A broken/misleading preview is a BUG, not cosmetic - the preview is the
  only thing the model sees inline; if it lies about size/shape the agent
  concludes the tool returned nothing. Before touching preview code, verify
  empirically and classify the defect (`references/preview-quality-debugging.md`).

## Phase Recover - bounded repair only

Entry: a defined failure condition occurred. Exit: root cause + clean state
verified. Allowed: only documented repair operations. Hard stop: destructive
shortcut or inferred recovery.

### Step X1 - Classify the failure (symptom first)

**Do**

- Choose ONE observed symptom class from the reference decision trees
  (`references/engine-health-debugging.md`,
  `references/plugin-lifecycle.md`,
  `references/preview-quality-debugging.md`) before any repair. Collect
  read-only evidence: orientation gate output, component evidence, source
  invocation sites, smallest reproducible input.

**Stop if**

- The "repair" starts from a suspected root cause instead of an observed
  symptom, or changes more than one dimension at a time (env vars,
  thresholds, source, branch, plugin install).

### Step X2 - Choose the permitted repair (taxonomy)

**Do**

- Match the situation to the git repair taxonomy (`aphrodite-boundaries`
  "Git repair taxonomy"): intended content wrong → edit the file directly,
  rerun verification; branch-owned identity crossed during a transplant →
  restore only named protected paths from that branch baseline; cherry-pick
  conflict → resolve semantics, scan ALL changed files for markers, then
  continue; empty cherry-pick → verify equivalence, skip as already
  contained; phantom gitlink → remove the indexed mode-160000 entry,
  validate recursively after later commits; wrong published version →
  release a new version.

**Verify**

```sh
git status --short
git submodule status --recursive
git log -1 --oneline
```

**Expected**

- The repair names its exact files/state, expected new observation, reversal
  method, and validation that succeeds before another repair is attempted.

**Stop if**

- The repair is a destructive shortcut (blanket checkout/reset, force-push,
  retag, hook re-creation, phantom-gitlink ignore).

### Step X3 - Re-run the affected gate and update the contract

**Do**

- Re-run the gate that failed. If source behavior differed from documented
  behavior, update the canonical owner skill + test matrix - never an
  unstructured note in a random reference file.

**Expected**

- Root cause recorded, clean state verified, affected gate green (phase
  exit).
