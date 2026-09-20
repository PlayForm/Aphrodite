---
name: aphrodite-release-flow
description: "Use when releasing or hotfixing Aphrodite (parent + plugin submodule), or verifying the live CCR engine after a bump. Sole owner of the release/hotfix/tag/version-sync ceremony."
version: 2.1.0
platforms: [macos]
tags: [aphrodite, release, branch, cherry-pick, submodule, hotfix, state-machine]
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
supersedes: aphrodite-branch-release-flow
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

## Ownership boundary (read this first)

- **This skill owns** release promotion, hotfix, tag creation, version
  sync-back, and artifact verification - _how_ the ceremony runs.
- `aphrodite-release-workflow` owns the version ledger, the artifact contract
  matrix, Gate R7's definitions, and the 4-event publishing separation -
  _what_ the ceremony must verify. Cross-reference it; never duplicate its
  tables.
- `aphrodite-boundaries` owns the git repair taxonomy, the branch-owned
  identity contract, and the human approval boundaries.
- `aphrodite-orientation` owns the Orient phase and the mandatory preflight
  gate.

## Topology & identity

- **Development** = workshop (accumulates, never rewritten; all tests + CI;
  no release tags). **Current** = distributed line (tags + GitHub releases +
  registry publishes ONLY here). The two branches are different products,
  not mergeable twins.
- **Branch-owned identity (protected, NEVER cross a transplant):**
  `.gitmodules` branch fields, `.github/workflows/*` triggers and push
  targets (`[Development]` vs `[Current]`), and the `plugins/aphrodite`
  gitlink. Contract table: `aphrodite-boundaries` "Branch-owned identity
  contract". This ceremony invokes it at THREE points: **before staging**
  (Step I1), **after the controlled restore** (Step I4), and **before tag
  creation** (Step R1). A path is never restored merely because it is
  inconvenient to merge; only a contract-declared identity path qualifies.
- **Two version tracks, never conflated:** binary `1.5.x` (parent
  Cargo.tomls + `package.json` + README badge) vs plugin `2.2.x`
  (`plugins/aphrodite/plugin.yaml`). `BINARY_VERSION` (S) is a third value:
  the live download pointer naming which binary release the plugin pairs
  with (what `download.sh`/`download.ps1` fetch). Canonical ledger:
  `aphrodite-release-workflow` §1 (5 rows, per-row authority /
  earliest-update / latest-safe-update / verification). Known drift at
  `a81acab6` (2026-09-20): `package.json` lags at `1.4.6` while both crates
  read `1.5.0` - fix the manifest before claiming a release and report the
  drift.
- **Bump order (verified):** the TWO parent crates + the `aphrodite-hermes`
  Cargo.toml's `aphrodite = { path = .., version = "X" }` dep pin +
  `package.json` move TOGETHER in one ceremony (`cargo check` fails
  otherwise); then `plugin.yaml` + `install_message` + README badges (they
  drift - the badge may lag two minors); then `BINARY_VERSION` LAST at tag
  time. A LOCAL `BINARY_VERSION` bump ahead of the tag is safe only when the
  referenced binaries already exist in `~/.hermes/aphrodite/binaries`
  (`_ensure_binaries` no-ops) - the "bump LAST" ceremony rule applies at TAG
  time, not local prep. Two agents with disjoint ownership (parent vs plugin
  submodule) do this cleanly; grep for the OLD version strings after, and
  update any test that pins them (the plugin's tests rarely do).
- **Every file is classified by phase/kind/layer** - `.hermes/classification/`
  (TAXONOMY.md + pass files, HPC codes `{K}{P}{L}-{N}` with ceremony
  annotations `→C`/`→D` destination, `+tag`, `+bump`, `+float`, `+guard`,
  `∅` never-crosses, `@R` ritual-only, `✝` absence-as-ceremony-rule, `@A`
  archival, `∅ (untracked)` regenerated-per-line) encodes the dual-line ship
  table. Consult it when deciding what crosses a transplant instead of
  re-deriving the ship-table; a new removal/absence gets recorded there as a
  ceremony rule. Re-derive file counts live (`find .hermes/classification
-type f | wc -l`), never assume.

## The unified lifecycle (this skill's state machine)

All Aphrodite work fits one lifecycle. This skill drives six phases;
`aphrodite-orientation` owns **Orient**. Each phase has an entry condition,
allowed changes, exit evidence, and a hard stop; the ceremony below walks the
phases in order - a phase may not be skipped or reordered.

| Phase                                         | Entry condition                             | Allowed changes                                | Exit evidence                                          | Hard stop                                       |
| --------------------------------------------- | ------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------- |
| **Orient** (owned by `aphrodite-orientation`) | Repository may be unknown                   | Read-only inspection                           | Correct repository, branch, submodule state identified | Dirty/unexpected repository state               |
| **Prepare**                                   | Scope and ownership known                   | Local code/docs/config edits                   | Targeted static checks pass                            | Unverified assumptions about live behavior      |
| **Validate**                                  | Changes are locally coherent                | Builds, tests, isolated runtime probes         | Evidence attached to the change                        | Test failures or stale-process uncertainty      |
| **Integrate**                                 | Validated change and clean ownership        | Controlled commits, cherry-picks, bounded sync | Protected paths unchanged or deliberately handled      | Unexpected conflict, extra staged files         |
| **Release**                                   | Current line contains exact release content | Tag, artifact build, deliberate publish        | Tag, artifacts, consumer verification                  | Existing version, missing assets, wrong trigger |
| **Observe**                                   | Release or runtime change complete          | Health and round-trip checks                   | Engine, retrieve, search, preview behavior verified    | Diagnostics disagree with release claim         |
| **Recover**                                   | A defined failure condition occurred        | Only documented repair operations              | Root cause and clean state verified                    | Destructive shortcut or inferred recovery       |

Machine rules:

- A phase transition requires the previous phase's exit evidence.
- A **hard stop** inside any phase sends the workflow to Recover (or stops
  entirely); never "fix forward" past a failed verification.
- Every mutation command sits in a step that carries Preconditions, Verify,
  Expected, Stop if, and Recovery. A step never silently combines mutation,
  validation, publishing, and recovery.

## Orientation gate (before ANY mutation)

No ceremony step below may run before the orientation gate: the five
read-only commands from `aphrodite-orientation` (`git rev-parse
--show-toplevel`, `git branch --show-current`, `git status --short`,
`git submodule status --recursive`, `git remote -v`), interpreted against
this skill's declared scope (repositories PlayForm/Aphrodite +
PlayForm/Aphrodite-Hermes, branches Development + Current, runtime modes
source + installed). STOP on any dirty/unexpected state.

**Auto-committer baseline (mandatory):** an external auto-committer sweeps
working-tree changes into commits and pushes, so `git status` is NOT stable
evidence. Capture before the first mutation and re-check at every boundary
(B4, pre-tag, post-sync):

```sh
git rev-parse HEAD
git ls-remote origin <branch>
```

If the tree is dirty when a sync step is about to run, **abort the sync** -
the auto-committer may sweep a staged squash set mid-ceremony (the content
usually survives as a commit; verify with `git log`, never `git status`).

---

## Phase Prepare (Development) - scope and ownership known

Entry: Orient passed, scope known. Exit: targeted static checks pass.
Allowed: local code/docs/config edits. Hard stop: unverified assumptions
about live behavior.

### Step P1 - Confirm version availability and ledger state

**Purpose:** Prove the proposed version numbers are free and every ledger
row matches its authority path before any bump.

**Preconditions**

- Orientation gate passed on Development; HEAD + remote tip captured.

**Do**

```sh
curl -A <ua> https://crates.io/api/v1/crates/aphrodite | grep max_version
curl -A <ua> https://crates.io/api/v1/crates/aphrodite-hermes | grep max_version
# ledger authority paths (aphrodite-release-workflow §1):
grep '^version' crates/aphrodite/Cargo.toml crates/aphrodite-hermes/Cargo.toml
grep '"version"' package.json
grep -m1 '^version' plugins/aphrodite/plugin.yaml
cat plugins/aphrodite/BINARY_VERSION
git submodule status plugins/aphrodite
```

**Verify**

```sh
git status --short   # only the pre-bump state; no surprise files
```

**Expected**

- `max_version` for both crates does NOT contain the proposed numbers.
- Every ledger row equals its authority path; known drift (`package.json`
  lag) is recorded and fixed in Step P2.

**Stop if**

- The proposed version is already published (a parallel release won the
  race) - claim the NEXT number instead; a burned crates.io version is gone
  forever.
- Any ledger row contradicts its authority path beyond the known drift.

**Recovery**

- Permitted: fix the manifest in place; re-run this step.
- Prohibited: reusing a claimed version; tagging before the version is free.

**Produces**

- Registry `max_version` output + ledger snapshot (evidence).

### Step P2 - Bump the binary and plugin version tracks in ledger order

**Purpose:** Move all version values to the proposed numbers in the only
order that keeps the workspace compiling and the distribution pointer safe.

**Preconditions**

- Step P1 passed; the proposed numbers are free.

**Do**

```sh
# 1. Binary track, ONE ceremony (parent repo): crates/aphrodite/Cargo.toml,
#    crates/aphrodite-hermes/Cargo.toml (version AND the `aphrodite = { path
#    = .., version = "X" }` dep pin), package.json
# 2. Plugin track (submodule repo): plugins/aphrodite/plugin.yaml version +
#    install_message, README badges
# 3. BINARY_VERSION stays UNCHANGED here - it moves LAST at tag time
```

**Verify**

```sh
cargo check -p aphrodite -p aphrodite-hermes
grep -rn "<OLD-VERSION>" crates/ plugins/aphrodite/package.json README.md || true
```

**Expected**

- `cargo check` passes (proves the crates + dep pin moved together).
- No old version strings remain in the owned locations.
- `BINARY_VERSION` still names the previous release.

**Stop if**

- `cargo check` fails because only one crate moved.
- A track was bumped alone (binary without plugin, or vice versa).
- `BINARY_VERSION` was touched - it is a live distribution pointer.

**Recovery**

- Permitted: edit the lagging file directly; re-run the check.
- Prohibited: `git reset`/`git checkout` to erase the bump; bumping
  `BINARY_VERSION` early because the tag is "soon".

**Produces**

- Local version bump (uncommitted until the user drives the commit).

### Step P3 - Refresh embedded templates and README link branches

**Purpose:** Prevent a stale-template release (fresh `aphrodite setup` writes
a config missing keys the engine reads) and wrong-branch README links on
crates.io.

**Preconditions**

- Step P2 passed; working changes are the intended ones.

**Do**

```sh
# Embedded templates: crates/aphrodite/templates/* are baked into the binary
# via include_str! (setup.rs CONFIG_TEMPLATE, shim). Diff against live config:
diff crates/aphrodite/templates/aphrodite.toml ~/.hermes/aphrodite/aphrodite.toml
# Shim drift-guard (setup.rs asserts byte-identity):
diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py
# Crate READMEs (crates/<crate>/README.md - what cargo auto-includes and
# crates.io renders): relative links render as blob/HEAD. Write every link
# absolute to the file's OWN branch context - infer it per file (which branch
# owns the file AFTER the merge): Development files -> tree/Development,
# files staged for the Current distribution line -> tree/Current.
```

**Verify**

```sh
git diff --stat   # scoped to the intended template/docs edits
```

**Expected**

- Template diff shows only intended key additions; shim `diff -q` exits 0
  (identical).
- README links resolve to the branch that owns the file after the merge.

**Stop if**

- The shim drifted (the release would fail setup.rs's assertion).
- A template references a key the engine no longer reads, or misses a key it
  does read (poll_worker, chain_split, navigation, preview tables).

**Recovery**

- Permitted: refresh the template from the live config; re-verify.
- Prohibited: disabling the drift-guard assertion.

**Produces**

- Refreshed templates + README edits; static checks pass (phase exit).

---

## Phase Validate (Development) - evidence attached to the change

Entry: changes locally coherent. Exit: evidence attached. Allowed: builds,
tests, isolated runtime probes. Hard stop: test failures or stale-process
uncertainty.

### Step V1 - Run the full gate battery and record ACTUAL numbers

**Purpose:** Attach real command output to the release claim - never "should
pass".

**Preconditions**

- Phase Prepare exit evidence exists (static checks passed).

**Do**

```sh
cargo test -p aphrodite
cargo test -p aphrodite-hermes
python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py
python3 Maintain/tests/test_check_ffi_contract.py
python3 Maintain/check_ffi_contract.py
ruff check plugins/aphrodite/
cargo clippy -p aphrodite -- -D warnings
npx pyright plugins/aphrodite/
npx prettier --check .hermes/**/*.md
```

**Verify**

- Record pass/fail counts exactly as printed (e.g. "406 passed, 0 failed").
  Baselines: AGENTS.md "Quality gates" table - re-derive live, never assume.

**Expected**

- All gates green with recorded numbers.

**Stop if**

- Any gate fails - the release claim is void until it passes.
- A gate's numbers contradict the previous release's baselines without a
  documented reason.

**Recovery**

- Permitted: fix the failing code/test; re-run that gate.
- Prohibited: shipping with a red gate because "the release is due".

**Produces**

- Gate evidence (recorded output) attached to the change.

### Step V2 - Fresh-process runtime probe (stale-dylib rule)

**Purpose:** Prove the running engine reflects THIS change - results from a
stale symlink target or an already-loaded dylib are not evidence.

**Preconditions**

- Step V1 passed; source mode confirmed (workspace manifests exist).

**Do**

```sh
cargo build --release -p aphrodite -p aphrodite-hermes
# restart the Hermes session / plugin process so the NEW dylib is loaded;
# then, in the fresh session:
aphrodite_stats        # record loaded version
aphrodite_test         # quick=1 sample, then full=3 (source_code/build/json)
```

**Verify**

```sh
aphrodite_rebuild      # binary vs plugin version cross-check; must match
```

**Expected**

- Loaded dylib version equals the freshly built version; `aphrodite_test`
  round trips pass.

**Stop if**

- The loaded version still reports the OLD build (stale process) - re-probe
  after a real restart; never conclude from the old process.
- Session counters reset on the bump (turn:0, entries 0) - that reset is
  NORMAL, not data loss.

**Recovery**

- Permitted: restart the session/process; re-run the probe.
- Prohibited: attributing behavior to the new build while the old dylib is
  still loaded.

**Produces**

- Runtime evidence: fresh-process stats + round-trip results.

---

## Phase Integrate (Development → Current) - controlled transplant, submodule FIRST

Entry: validated change + clean ownership. Exit: protected paths unchanged
or deliberately handled. Allowed: controlled commits, cherry-picks, bounded
sync. Hard stop: unexpected conflict, extra staged files.

### Step I1 - B4 branch-identity audit (MANDATORY before ANY sync or tag)

**Purpose:** Prove zero branch-identity statements live on the wrong branch
BEFORE anything is staged, merged, or tagged. First contract point: before
staging. Full canonical text: `.hermes/notes/release/RELEASE-METHODOLOGY.md`
`### B4` (invariant I11).

**Preconditions**

- Orientation gate passed; HEAD + remote tip captured on BOTH sides.

**Do** (read-only ref access only - NEVER checkout the other branch)

```sh
git fetch Source
# 1. Workflow triggers + push targets must match the branch they live on
#    (Auto.yml's `branch:` is branch-owned identity; a push target of the
#    OTHER branch = LEAK - the Auto.yml leak class):
git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Development -- .github/workflows
git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Current -- .github/workflows
# 2. .gitmodules branch fields match their branch:
git show Source/Development:.gitmodules | grep -E '^branch'
git show Source/Current:.gitmodules | grep -E '^branch'
# 3. Gitlink targets resolve to the branch they belong on:
git -C plugins/aphrodite branch --contains "$(git ls-tree Source/Development plugins/aphrodite | awk '{print $3}')"
git -C plugins/aphrodite branch --contains "$(git ls-tree Source/Current plugins/aphrodite | awk '{print $3}')"
# 4. Keyword scan for branch-identity statements on the wrong branch:
git grep -n -E 'Current|Development' Source/Development Source/Current -- .github/workflows .gitmodules
```

**Verify**

- `git status --short` on both sides still matches the pre-ceremony state.

**Expected**

- Zero hits. Verified at `a81acab6` (2026-09-20): Development's copy of
  Auto.yml pushes `branch: Development`, Current's copy pushes `branch:
Current` - the 2026-09-18 leak finding (Current pushed Development) is
  FIXED; the audit stays mandatory because state drifts. Docs/README links
  and release notes describing the dual-line model are content, not identity
    - do not flag them.

**Stop if**

- ANY hit - **ABORT the ceremony**; record in
  `.hermes/notes/release/CEREMONY-AUDIT.md`, fix the offending branch
  separately, re-run clean, then proceed.

**Recovery**

- Permitted: fix the offending branch's identity files directly and commit
  them on that branch.
- Prohibited: proceeding past a hit "because it is only the heartbeat";
  checking out the other branch to "look around".

**Produces**

- B4 audit evidence (scan output) - the gate record for this ceremony.

### Step I2 - Plugin sync FIRST (bottom-up rule)

**Purpose:** Release the plugin commit so the parent's gitlink references the
released plugin in one pass - never a Development-only commit.

**Preconditions**

- Step I1 clean on BOTH refs; plugin submodule clean (no phantom gitlink).

**Do**

```sh
git -C plugins/aphrodite checkout Current
git -C plugins/aphrodite merge --squash Development
# selectively keep: version bump, docs, real fixes; discard dev-only scaffolding
git -C plugins/aphrodite commit -m "release: sync vX.Y.Z"
git -C plugins/aphrodite push Source Current
```

**Verify**

```sh
git -C plugins/aphrodite log -1 --oneline
git submodule status plugins/aphrodite    # parent side: no '+'
```

**Expected**

- The plugin's Current tip is the `release: sync vX.Y.Z` commit; parent
  gitlink unchanged (it floats in Step I4).

**Stop if**

- The plugin squash brings branch-owned identity across (`.gitmodules`,
  workflow triggers) - unstage and leave them branch-owned.
- A phantom mode-160000 entry reappears after the commit (auto-committer
  vector) - clear before proceeding.

**Recovery**

- Permitted: `git -C plugins/aphrodite rm --cached <phantom-path>`; edit
  files directly.
- Prohibited: blanket `git checkout`/`git reset` of the plugin tree.

**Produces**

- Plugin Current release commit (pushed).

### Step I3 - Validate the plugin commit before the parent gitlink moves

**Purpose:** Prove the plugin commit is loadable and its `BINARY_VERSION`
target exists BEFORE the parent consumes it (bottom-up: validate the plugin
commit before updating the parent gitlink).

**Preconditions**

- Step I2 pushed; plugin commit hash recorded.

**Do**

```sh
git -C plugins/aphrodite log -1 --format=%H
aphrodite_stats; aphrodite_test   # in a session pointing at this plugin tree
# artifact availability for the named BINARY_VERSION (if it names a release):
gh release view "Aphrodite/v<ver>" --repo PlayForm/Aphrodite --json assets -q '.assets[].name'
```

**Verify**

```sh
aphrodite_rebuild   # binary vs plugin version pair
```

**Expected**

- Plugin loads in a fresh process; version pair matches; any named release's
  required assets exist (or the `BINARY_VERSION` bump is deferred to tag
  time).

**Stop if**

- The plugin fails to load or the version pair mismatches - the parent must
  not float its gitlink to a broken plugin commit.
- `BINARY_VERSION` names a release whose assets do not exist yet.

**Recovery**

- Permitted: fix the plugin commit and re-validate; defer the
  `BINARY_VERSION` bump.
- Prohibited: floating the parent gitlink to a non-released plugin commit.

**Produces**

- Plugin-commit validation evidence.

### Step I4 - Parent sync + controlled identity restore

**Purpose:** Transplant the validated snapshot onto Current while restoring
Current's own branch identity (second contract point: after the controlled
restore) and floating the gitlink to the released plugin tip.

**Preconditions**

- Steps I1-I3 passed; Current tree clean; HEAD + remote tip captured.

**Do**

```sh
git checkout Current
git merge --squash <Development-cutoff>
# controlled restore of branch-owned identity ONLY (the ceremony invariant,
# NOT a repair mechanism - boundaries' git repair taxonomy):
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite
git -C plugins/aphrodite checkout <plugin-current-tip>     # float the gitlink
git add -A
git commit -m "release: sync vX.Y.Z from Development"
git push origin Current
```

**Verify**

```sh
git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite   # EMPTY (I9)
git submodule status plugins/aphrodite                             # no '+'
```

**Expected**

- Identity diffs empty; submodule status shows no `+`; commit message names
  the version.

**Stop if**

- Protected paths differ from Current's baseline after the restore.
- Any extra/unexpected staged file appears beyond the transplant set.

**Recovery**

- Permitted: re-run the controlled restore of the named protected paths from
  Current's baseline; edit file content directly.
- Prohibited: blanket checkout/reset of the worktree to "clean up";
  restoring a path merely because the merge was inconvenient.

**Produces**

- Parent Current release-sync commit (pushed); identity verified (phase
  exit).

### Step I5 - Headroom fork leg (mandatory fork publication tracking)

**Purpose:** Carry the `vendor/headroom` fork delta into THIS release - the
fork crate is a first-class release artifact, and a stale fork version makes
CI skip the publish silently (the 1.5.0 published-version trap). Tracking
contract: `vendor/headroom/RELEASE-CYCLE.md` + `vendor/headroom/CHANGELOG.md`
(the fork's release record - update both per cycle).

**Preconditions**

- Steps I1-I4 passed; Current tree clean; HEAD + remote tip captured.

**Do**

```sh
# 1. Fork delta since the last published commit (last published = the commit
#    carrying the version live on crates.io - the 0.1.2 bump c6b61470):
git -C vendor/headroom log <last-published-commit>..HEAD --oneline
# 2. ANY delta => bump the fork crate version AND the parent pin TOGETHER
#    (vendor/headroom/crates/headroom-core/Cargo.toml `version` AND
#    crates/aphrodite/Cargo.toml line 55 - cargo check fails otherwise),
#    update vendor/headroom/RELEASE-CYCLE.md + CHANGELOG.md, commit + push
#    the fork's Current branch:
git -C vendor/headroom push Source Current
# 3. Fork tag BEFORE dispatch (fork scheme aphrodite-vX.Y.Z, e.g.
#    aphrodite-v0.10.0 - NEVER the parent Aphrodite/v* scheme); pauses for
#    human approval like Step R4:
git -C vendor/headroom tag aphrodite-v<X.Y.Z> <fork-commit>
git -C vendor/headroom push Source Current --tags
# 4. Headroom-fork release notes per the vNEXT-draft convention (retrospective
#    mode, separate from the binary notes:
#    .hermes/release-notes/headroom-fork-vNEXT-draft.md - fork tag scheme +
#    the `aphrodite-headroom-core` package name)
# 5. Float the parent gitlink to the TAGGED fork commit (CI publishes the
#    parent-recorded gitlink tree, not the local submodule HEAD) and push:
git -C vendor/headroom checkout <fork-tag-commit>
git add vendor/headroom
git commit -m "build(submodule): point vendor/headroom at <fork commit>"
git push Source Current
```

**Verify**

```sh
git submodule status vendor/headroom    # no '+'
git ls-tree HEAD vendor/headroom        # tagged fork commit recorded
curl -A <ua> https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

**Expected**

- Delta = 0, OR the fork crate + parent pin moved together, the fork tag
  exists BEFORE dispatch, and the gitlink floats to the tagged fork commit.

**Stop if**

- A fork delta exists but the fork crate version was NOT bumped - the version
  check sees the old version live on crates.io and skips silently (1.5.0
  trap: ~14 unpublished commits, nothing failed, old 0.1.2 kept shipping).
- The fork tag would be created after dispatch; the parent pin moved without
  the fork crate (or vice versa).

**Recovery**

- Permitted: bump to the next free number and re-verify (crates.io versions
  are immutable - never reuse a burned version).
- Prohibited: dispatching with a stale fork version; re-tagging the fork.

**Produces**

- Fork tag + parent gitlink float + headroom-fork notes. Dispatch order
  (Publish.yml `needs:` chain): Test → Publish-Headroom-Core →
  Publish-Aphrodite → Publish-Hermes - headroom publishes FIRST; run
  `gh workflow run Publish -f publish_crates=true` in Step R6, only after
  this leg (phase exit).

---

## Phase Release - tag, artifacts, registry (4 irreversible events)

Entry: Current line contains EXACT release content. Exit: tag, artifacts,
consumer verification. Allowed: tag, artifact build, deliberate publish.
Hard stop: existing version, missing assets, wrong trigger.

The publishing separation is MANDATORY - four irreversible events, never
combined in one opaque script invocation (definitions owned by
`aphrodite-release-workflow` §3; the checklist below is the ceremony's
execution):

1. **Create the release-sync commit** - done in Step I4.
2. **Create and push the immutable tag** - Step R4.
3. **Publish or attach binary artifacts** - Step R5 (Build.yml attaches).
4. **Publish immutable registry packages** - Step R6 (cargo publish).

Each irreversible event requires ALL of: identity confirmation, version
availability check, intended artifact/package list, trigger audit (Gate R7),
explicit human approval (`Ready for approval` pause), and post-event
consumer-perspective verification.

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
git diff <release-sync-commit> HEAD -- .gitmodules .github/workflows plugins/aphrodite
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
git show <release-sync-commit>:.github/workflows/Publish.yml | grep -nE '^on:|tags:|workflow_dispatch|publish_crates|if:'
git show <release-sync-commit>:.github/workflows/Build.yml | grep -nE '^on:|tags:|workflow_dispatch|if:'
git show <release-sync-commit>:.github/workflows/Check.yml | grep -nE '^on:|tags:|branches:'
git show <release-sync-commit>:.github/workflows/Auto.yml | grep -nE '^on:|schedule|branch:'
# build the trigger table: event -> workflows -> jobs -> publishing side effects
```

**Record**

- Which workflows trigger from this tag; which jobs publish GitHub assets;
  which jobs publish crates/packages; required secrets and manual inputs.

**Verified snapshot (commit `a81acab6`, 2026-09-20 - evidence, NOT a
substitute for the audit at tag time):**

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
assets exist - the exact 2026-09-17 failure mode was a bump ahead of asset
availability 404ing every download.

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
git submodule status plugins/aphrodite        # no '+'
git ls-tree HEAD plugins/aphrodite            # intended commit
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
curl -A <ua> https://crates.io/api/v1/crates/aphrodite | grep max_version
curl -A <ua> https://crates.io/api/v1/crates/aphrodite-hermes | grep max_version
# IF Step I5 carried a fork delta, dispatch the headroom publish now - the
# fork leg (crate + parent pin bumped, fork tag + gitlink float) MUST have
# run first, else the version check skips the stale version silently (the
# 1.5.0 published-version trap); needs chain: Test -> Publish-Headroom-Core
# -> Publish-Aphrodite -> Publish-Hermes (headroom publishes FIRST):
gh workflow run Publish -f publish_crates=true
# then verify the headroom publish landed (see the reference note):
curl -A <ua> https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

**Verify**

- The crates.io index/API serves the new versions; a consumer `cargo add` /
  `cargo install` resolves them. For headroom: the index must serve the NEW
  fork version - seeing only the old 0.1.2 means the skip fired again
  (`references/headroom-publish.md` "Post-event consumer verification").

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

---

## Phase Observe - live-engine verification (post-release)

Entry: release or runtime change complete. Exit: engine, retrieve, search,
preview behavior verified. Allowed: health and round-trip checks. Hard stop:
diagnostics disagree with the release claim.

### Step O1 - Ground truth via aphrodite_stats, not the banner

**Purpose:** The plugin's orientation banner is NOT ground truth;
`aphrodite_stats` is.

**Do**

```sh
aphrodite_stats    # version, engine_enabled, thresholds, proxy liveness
aphrodite_rebuild  # binary vs plugin version cross-check (must match)
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
aphrodite_test     # quick=1 sample; full=3: source_code/build/json
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

---

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

---

## Hotfix on Current - the fast path (user preference)

Hotfixes are worked DIRECTLY on Current - no Development round-trip, no full
ceremony. The same state machine runs with Current as the workspace: Prepare

- Validate on Current, Release on Current, then Integrate = sync-back to
  Development. The user drives every commit/tag/push decision; never commit,
  tag, or push unasked.

### Step H1 - Fix + commit on Current; check the version is free first

**Purpose:** Claim the NEXT patch number, never a burned one.

**Preconditions**

- Orientation gate on Current; HEAD + remote tip captured.

**Do**

```sh
# fix in place on Current; before committing the bump:
curl -A <ua> https://crates.io/api/v1/crates/aphrodite | grep max_version
# bump the binary patch to the next number; commit on Current
```

**Verify**

- `max_version` does not contain the proposed number.

**Stop if**

- The proposed number is already published - claim the next one.

**Recovery**

- Permitted: bump to the next free number.
- Prohibited: reusing a claimed version; retagging an older release.

### Step H2 - BINARY_VERSION bump LAST + gitlink float (same rule as R3)

**Do**

- Bump `BINARY_VERSION` only after the release assets exist (tag pushed AND
  Build completed), then float the parent gitlink to that plugin commit.
  Version bumps ride the release; the plugin submodule's `BINARY_VERSION`
  and the parent gitlink float are the LAST commits of a hotfix cycle.

**Stop if**

- Assets do not exist for the named version (`_check_version_published`
  warns).

### Step H3 - Tag on the release-sync commit only

**Do**

- Run B4 (Step I1) + Gate R7 (Step R2) + the approval pause (Step R4) before
  the tag. A post-release style/cleanup commit does NOT move the tag
  (historical example: plugin v2.1.3 sat on its `release: sync v2.1.3`
  commit, not the reformat commit that followed it).

### Step H4 - Sync-back: selective cherry-pick -x (Current → Development)

**Purpose:** Bring release-line fixes back WITHOUT merging - Development has
diverged with its own work and Current's tree is test-free with
dev-scaffolding absent.

**Preconditions**

- Submodule clean (or phantom cleared); B4 audit clean.

**Do**

```sh
# submodule FIRST (bottom-up rule), then parent:
git log --oneline Development..Source/Current     # in each repo separately
git cherry-pick -x <commit>                        # CHRONOLOGICAL order
```

- PICK: real fixes (setup.rs changes, config-template refresh, docs URL
  fixes, release-notes finalization, version bumps that ride the line).
- SKIP: gitlink-only bumps (`chore: bump plugin submodule to <sha>` - the
  gitlink is branch-owned, Development floats its own), style-only commits on
  files Development has since rewritten, release snapshots that re-add
  content Development deliberately removed (e.g. `directives/`) or delete
  test files (Development keeps tests).
- Conflict resolution: take the PICKED side's SEMANTICS but the repo's own
  formatting (nightly rustfmt: tabs, `space_after_colon = false` -
  patch-tool rustfmt warnings about unstable options are noise, not errors).
- After EVERY `--continue`, grep the staged file set for leftover markers:

```sh
git diff HEAD~1 HEAD --name-only | xargs grep -l '<<<<<<<' || true
# if one slipped in: fix, git add, git commit --amend --no-edit
```

**Stop if**

- An empty cherry-pick (`nothing to commit, working tree clean`) is treated
  as failure - it means the change is already contained; verify with
  `git diff <HEAD> <source> -- <paths>` and skip.
- A second conflict region in the same file survives the first resolution
  (it gets COMMITTED by `--continue`).

**Recovery**

- Permitted: resolve semantics + full-file marker sweep; `--amend` a slipped
  marker.
- Prohibited: merging Current wholesale; rebasing picks; disabling the hooks
  (they are already gone - see pitfalls).

### Step H5 - V1 identity re-expression guard (after any pick touching the gitlink)

**Purpose:** Re-express Development's OWN identity after hotfix picks so a
pick can never transplant Current's identity onto Development.

**Do**

```sh
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite
# (controlled restore of branch-owned identity only - the ceremony invariant,
# NOT a repair mechanism; for corrupted working content never checkout/reset)
git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite   # EMPTY (I9)
git submodule status plugins/aphrodite                             # no '+'
git cat-file -e HEAD:plugins/aphrodite/__init__.py                 # picked files exist
```

**Expected**

- Identity diffs empty; no `+`; no 160000 phantom in the submodule; every
  picked commit's files exist in HEAD.

**Stop if**

- Identity files differ from Development's baseline; a phantom gitlink
  reappears.

**Recovery**

- Permitted: re-run the controlled restore from Development's baseline;
  clear a phantom with `git -C plugins/aphrodite rm --cached <path>`.
- Prohibited: blanket worktree checkout/reset; ignoring a 160000 entry.

### Step H6 - Verify remote tip, not push output

**Do**

```sh
git log Source/Development    # confirm the remote tip
```

**Expected**

- Pushed commits visible on the remote tip - the auto-committer may already
  have pushed as commits landed ("Everything up-to-date" is not evidence).

**Produces**

- Hotfix synced back; B4 re-run after the sync (phase exit).

---

## Pitfalls (real ceremony history - do not lose)

- **Phantom self-referential gitlink (recurs):** a stray mode-160000 entry
  inside the submodule pointing at its OWN commit, swept in by the
  auto-committer alongside unrelated work. Symptom: `git submodule status`
  INSIDE the submodule fails with `fatal: no submodule mapping found in
.gitmodules for path '<submodule-name>'` (the submodule has NO
  `.gitmodules`, so any 160000 entry is self-referential); the remote shows a
  nested `plugins/aphrodite` folder that should not exist; `git clone
--recurse-submodules` dies with `fatal: No url found for submodule path
'X/X' in .gitmodules`. Detect inside the submodule:
  `git -C <submodule> ls-files -s | grep 160000` (a hit = phantom); repair:
  `git -C <submodule> rm --cached <path>` + verify the grep is empty +
  `git status` clean; commit, push; float parent gitlinks OFF any commit that
  contains it (`git ls-tree <ref> <path>` for 160000). The auto-committer
  RE-STAGES a fresh phantom pointing at the new HEAD after you clear it -
  re-run the `ls-files -s | grep 160000` check after EVERY subsequent
  submodule commit; a clean check at the end of the session is the real pass
  criterion, not one removal.
- **Auto-committer races:** it sweeps working-tree changes (including staged
  squash sets and gitlink bumps) into commits and pushes. Never fight it;
  verify final state with `git log`/`git submodule status`, not `git status`.
  A squash staged for VSCode review can be swept mid-review - the content
  survives as a commit.
- **`git cherry-pick --continue` commits leftover conflict markers:** a file
  with TWO conflict regions - resolving the first and continuing commits the
  still-marker'd second region into the branch. Grep the committed file set
  for `<<<<<<<` after every `--continue`; fix + `git commit --amend
--no-edit` when one slipped through.
- **Empty cherry-pick is "already contained", not an error.** `nothing to
commit, working tree clean` means the change is already in HEAD via an
  earlier pick or merge resolution - verify with
  `git diff <HEAD> <source> -- <paths>` and skip the commit instead of
  aborting in confusion.
- **The git hooks are GONE - never re-create them.** The entire `.githooks/`
  set (pre-commit, post-commit, post-checkout, post-merge, lib/*) was REMOVED
  2026-09-17 because the auto-bump hooks were the resurrection vector for the
  phantom self-referential gitlink: `post-commit`/`post-checkout` re-staged a
  160000 entry named after the submodule inside the submodule after every
  manual clear, and `package.json`'s `prepare` re-installed the hooks on
  every npm install. Removal = `git rm -r .githooks`, unset `core.hooksPath`
  in parent AND submodule, strip the `prepare` script, delete the
  `.gitattributes` `.githooks/*` lines, clear the phantom with
  `git -C <submodule> rm --cached <path>`. With no hooks, branch anchoring,
  gitlink auto-bump, and the commit gate are gone: submodule pins are
  verified by hand (`git submodule status` shows no '+' = I2), and a detached
  submodule HEAD is fixed manually (`git -C plugins/aphrodite checkout
Development`). Re-creating any hook or the prepare script is a ceremony
  violation.
- **Submodule-first ordering is mandatory** in every phase (release AND
  sync-back): plugin first, then parent, so the parent's gitlink references
  the released plugin in one pass.
- **Removing a subsystem = sweep the whole tree, then record the absence.**
  Skills, profiles, s2/navigation, and the installers were each removed
  across sessions: the self-heal schema (`layout_schema.json`), embedded
  templates, config example, README tree diagrams, bench scripts, release
  skills, and the classification passes ALL carry references to the removed
  thing. Delete the code AND every reference (schema entries, feature gates +
  their cfg branches, docs, tests that probe it), then state the ABSENCE as a
  ceremony rule (e.g. 'profiles never ship', 'skills live dev-side',
  'directives ship in the binary') so a future session does not re-add it or
  treat the empty cherry-pick as an error. The sweep includes the FFI symbol
  list: removing a `#[no_mangle]` export without updating the dylib's
  expected-symbol list leaves `aphrodite_rebuild` (and any dlsym-based check)
  dying with "missing an expected symbol" against the fresh build - grep for
  the symbol name in the tooling/check code, not just the crate's lib.rs, and
  remember the live session may still hold the OLD dylib until Hermes is
  restarted.
- **The setup flow needs a key, not the plugin:** proxy spawn dies with "no
  API key configured" when `APHRODITE_API_KEY` / toml `api_key` is absent -
  Hermes' provider config is NOT reused. See `references/plugin-lifecycle.md`
  for key sourcing and install layouts.
- **Package READMEs render on crates.io:** the crate-dir READMEs
  (`crates/<crate>/README.md` - what cargo auto-includes), NOT the root
  README, are what crates.io shows. Relative links there render as
  `blob/HEAD`. Write every link absolute to the file's OWN branch context -
  infer it per file, never blanket-assume: the branch that owns the file
  AFTER the merge (Development files → `tree/Development`, files staged for
  the Current distribution line → `tree/Current`).
- **Tag immutability:** create the tag only after the exact release-sync
  commit and all tag prerequisites (B4, Gate R7, asset contract). Re-tagging
  re-fires Build/Publish and rewrites history's evidence.
- **Embedded templates drift:** stale templates mean fresh `aphrodite setup`
  writes a config missing keys the engine reads - refresh before release
  (Step P3); the shim `templates/__init__.py` must stay byte-identical to
  the live plugin `__init__.py` (setup.rs asserts it).

## Related

- `aphrodite-release-workflow` - version ledger, artifact contract matrix,
  Gate R7 definitions, publishing separation, release-notes standards.
- `vendor/headroom/CHANGELOG.md` + `vendor/headroom/RELEASE-CYCLE.md` - fork
  change ledger / tracking contract (updated per cycle in Step I5).
- `aphrodite-boundaries` - git repair taxonomy, branch-owned identity
  contract, human approval boundaries, failure policy.
- `aphrodite-orientation` - mandatory preflight gate; owns the Orient phase.
- `.hermes/notes/release/RELEASE-METHODOLOGY.md` - canonical full ceremony +
  invariant checklist I1-I11 (incl. I9 protected-paths, I11 branch-identity)
  and the `### B4` full text.
- `.hermes/notes/CEREMONY.md` - condensed ceremony spec (Phase A/B, B4).
- `.hermes/governance/VERIFICATION-MATRIX.md` - global claims → probes
  (version ledger rows owned by `aphrodite-release-workflow`).
- `references/plugin-lifecycle.md` - install layouts, uninstall procedure,
  proxy key sourcing (decision-tree runbook).
- `references/engine-health-debugging.md` - live-engine probe battery,
  on-disk map, failure-chain diagnosis (decision-tree runbook).
- `references/preview-quality-debugging.md` - preview defect classes,
  empirical ctypes battery, dead config levers, fix shape (decision-tree
  runbook).
- `aphrodite-branch-release-flow` - superseded; archived; **Historical
  only - do not execute**.

## Claim-to-Test Matrix

| Claim                                                      | Evidence source                                                                                 | Test                                                                                        | Pass condition                                                                  | Failure response                                                                                                               |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Tag push side effects are exactly as audited               | Workflow files at the exact tag commit                                                          | Gate R7: build the trigger table from the actual files at the tag commit                    | Only accepted jobs reachable from the tag                                       | Change the workflow or halt the tag                                                                                            |
| Tag push publishes `aphrodite` + `aphrodite-hermes` crates | Publish.yml publish-step `if:` conditions at the tag commit                                     | `git show <tag>:.github/workflows/Publish.yml                                               | grep -n 'startsWith(github.ref'`                                                | Both publish steps carry `\|\| startsWith(github.ref, 'refs/tags/Aphrodite/')` (current state) or the audit records the change | Accept the side effect explicitly or halt the tag |
| `aphrodite-headroom-core` is never tag-published           | Publish.yml Publish-Headroom-Core publish-step `if:`                                            | Read the publish-step `if:` at the tag commit                                               | `workflow_dispatch && publish_crates && published == 'false'` only              | Treat any tag-reachable headroom publish as unexpected - stop                                                                  |
| Identity never crosses a transplant                        | `.gitmodules`, workflow triggers, plugin gitlink                                                | B4 scan + I9 diff at the 3 contract points (I1, I4, R1)                                     | Zero identity hits; identity diffs empty                                        | ABORT the ceremony; fix the offending branch                                                                                   |
| Auto.yml heartbeat does not leak (verified `a81acab6`)     | Auto.yml `branch:` on both refs                                                                 | B4 scan step 1                                                                              | Each copy pushes its OWN branch                                                 | Fix the leaking copy; re-run B4                                                                                                |
| `BINARY_VERSION` moves LAST                                | `plugins/aphrodite/BINARY_VERSION` + release asset list                                         | Compare bump-commit order against tag + Build completion; `_check_version_published` silent | Bump commit is after tag/assets; no download pointer to missing assets          | Defer the bump; never tag with a pointer to missing assets                                                                     |
| Plugin commit validated before parent gitlink update       | Plugin gitlink + fresh-process probe                                                            | Step I3: load the plugin commit in a fresh process, version pair check                      | Plugin loads; `aphrodite_rebuild` matches                                       | Fix the plugin commit; do not float the gitlink                                                                                |
| Empty cherry-pick is already-contained                     | `git diff <HEAD> <source> -- <paths>`                                                           | Step H4                                                                                     | Diff empty → skip the commit                                                    | Treat as success, not failure                                                                                                  |
| Cherry-pick never commits leftover markers                 | Staged file set after every `--continue`                                                        | `git diff HEAD~1 HEAD --name-only                                                           | xargs grep -l '<<<<<<<'`                                                        | Zero matches                                                                                                                   | Fix + `git commit --amend --no-edit`              |
| Protected paths unchanged after any transplant             | `git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite`                              | I9 at post-restore (I4) + post-sync (H5)                                                    | Empty                                                                           | Controlled restore; never blanket checkout                                                                                     |
| Runtime evidence matches the release claim                 | `aphrodite_stats` / `aphrodite_rebuild` / `aphrodite_test`                                      | Observe phase battery                                                                       | Versions agree; round trips pass; previews honest                               | Stale dylib → restart + re-probe; classify preview defects before touching code                                                |
| Every irreversible event pauses for human approval         | Steps R4-R6 + `aphrodite-boundaries`                                                            | Dry-run the 4-event separation with a simulated ceremony                                    | Workflow halts at each `Ready for approval` boundary                            | Enforce the gate; never chain events in one script                                                                             |
| `.githooks` stay removed                                   | `git config core.hooksPath`; `ls .githooks`; `package.json` prepare                             | Grep parent + submodule for hooks/prepare remnants                                          | Unset/absent everywhere; no phantom vector re-introduced                        | Re-create nothing; report the violation                                                                                        |
| Version numbers match their ledger authority               | Cargo.tomls, dep pin, `package.json`, `plugin.yaml`, `BINARY_VERSION`, gitlink                  | Step P1 ledger read                                                                         | Each row equals its authority; known `package.json` lag recorded                | Fix the manifest before claiming; report the drift                                                                             |
| Fork delta is carried into every release                   | `git -C vendor/headroom log <last-published-commit>..HEAD` + `vendor/headroom/RELEASE-CYCLE.md` | Step I5 fork-delta check                                                                    | Delta = 0, or fork crate + parent pin bumped together, fork tag before dispatch | Publish the fork with the release; never ship a stale fork version (1.5.0 trap)                                                |
