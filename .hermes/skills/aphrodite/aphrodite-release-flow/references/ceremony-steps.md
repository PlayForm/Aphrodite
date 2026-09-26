# Ceremony steps - Prepare, Validate, Integrate

Full step walkthroughs for the first three phases of the unified lifecycle
(see SKILL.md for the phase order and machine rules). Every step carries
Preconditions, Do (exact commands), Verify, Expected, Stop if, and Recovery;
a step never silently combines mutation, validation, publishing, and recovery.

## Unified lifecycle table (state machine)

A phase may not be skipped or reordered; a phase transition requires the
previous phase's exit evidence.

| Phase                                         | Entry condition                             | Allowed changes                                | Exit evidence                                          | Hard stop                                       |
| --------------------------------------------- | ------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------- |
| **Orient** (owned by `aphrodite-orientation`) | Repository may be unknown                   | Read-only inspection                           | Correct repository, branch, submodule state identified | Dirty/unexpected repository state               |
| **Prepare**                                   | Scope and ownership known                   | Local code/docs/config edits                   | Targeted static checks pass                            | Unverified assumptions about live behavior      |
| **Validate**                                  | Changes are locally coherent                | Builds, tests, isolated runtime probes         | Evidence attached to the change                        | Test failures or stale-process uncertainty      |
| **Integrate**                                 | Validated change and clean ownership        | Controlled commits, cherry-picks, bounded sync | Protected paths unchanged or deliberately handled      | Unexpected conflict, extra staged files         |
| **Release**                                   | Current line contains exact release content | Tag, artifact build, deliberate publish        | Tag, artifacts, consumer verification                  | Existing version, missing assets, wrong trigger |
| **Observe**                                   | Release or runtime change complete          | Health and round-trip checks                   | Engine, retrieve, search, preview behavior verified    | Diagnostics disagree with release claim         |
| **Recover**                                   | A defined failure condition occurred        | Only documented repair operations              | Root cause and clean state verified                    | Destructive shortcut or inferred recovery       |

## File classification (ship-table legend)

Every file is classified by phase/kind/layer in `.hermes/classification/`
(TAXONOMY.md + pass files). HPC codes `{K}{P}{L}-{N}` carry ceremony
annotations: `→C`/`→D` destination, `+tag`, `+bump`, `+float`, `+guard`,
`∅` never-crosses, `@R` ritual-only, `✝` absence-as-ceremony-rule, `@A`
archival, `∅ (untracked)` regenerated-per-line. The classification encodes
the dual-line ship table. Consult it when deciding what crosses a transplant
instead of re-deriving the ship-table; a new removal/absence gets recorded
there as a ceremony rule. Re-derive file counts live, never assume:

```sh
find .hermes/classification -type f | wc -l
```

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
curl -A < ua > https://crates.io/api/v1/crates/aphrodite | grep max_version
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-hermes | grep max_version
# ledger authority paths (aphrodite-release-workflow §1):
grep '^version' crates/aphrodite/Cargo.toml crates/aphrodite-hermes/Cargo.toml
grep '"version"' package.json
grep -m1 '^version' plugins/aphrodite/plugin.yaml
cat plugins/aphrodite/BINARY_VERSION
git submodule status plugins/aphrodite
```

**Verify**

```sh
git status --short # only the pre-bump state; no surprise files
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
git diff --stat # scoped to the intended template/docs edits
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
cargo test -p aphrodite --lib setup::tests # targeted: config template/setup
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
# Source the environment file FIRST - its cargo() wrapper syncs the binary +
# dylib together; a fresh release build lands in target/release, and
# 'aphrodite setup' run from THAT binary installs it:
cargo build --release -p aphrodite -p aphrodite-hermes
# restart the Hermes session / plugin process so the NEW dylib is loaded;
# then, in the fresh session:
aphrodite_stats # record loaded version
aphrodite_test  # quick=1 sample, then full=3 (source_code/build/json)
```

**Verify**

```sh
aphrodite_rebuild # binary vs plugin version cross-check; must match
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

- Zero hits. At commit `a81acab6`: Development's copy of Auto.yml pushes
  `branch: Development`, Current's copy pushes `branch: Current` - a past
  leak (Current pushed Development) is FIXED; the audit stays mandatory
  because state drifts. Docs/README links and release notes describing the
  dual-line model are content, not identity - do not flag them.

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
git submodule status plugins/aphrodite # parent side: no '+'
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
aphrodite_stats
aphrodite_test # in a session pointing at this plugin tree
# artifact availability for the named BINARY_VERSION (if it names a release):
gh release view "Aphrodite/v<ver>" --repo PlayForm/Aphrodite --json assets -q '.assets[].name'
```

**Verify**

```sh
aphrodite_rebuild # binary vs plugin version pair
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
# selective discard of dev-only scaffolding (I2 precedent): the debug TOOL is
# Development-only (✝, classification DEV-engine-build.md) - drop the
# tools.rs `aphrodite_debug` insert, the schemas.rs `schema_debug` entry,
# and debug.rs `set_enabled_current`/`current_root` when staging; debug.rs
# core (record_session/last_session/debug_line/enabled_for) CROSSES normally
# (lib.rs session-tracking calls depend on it).
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
git diff HEAD -- .gitmodules .github/workflows plugins/aphrodite # EMPTY (I9)
git submodule status plugins/aphrodite                           # no '+'
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
# 3. Fork tag BEFORE the release chain (fork scheme aphrodite-vX.Y.Z, e.g.
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
git submodule status vendor/headroom # no '+'
git ls-tree HEAD vendor/headroom     # tagged fork commit recorded
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

**Expected**

- Delta = 0, OR the fork crate + parent pin moved together, the fork tag
  exists before the release chain runs, and the gitlink floats to the tagged
  fork commit.

**Stop if**

- A fork delta exists but the fork crate version was NOT bumped - the version
  check sees the old version live on crates.io and skips silently (1.5.0
  trap: commits went unpublished, nothing failed, the old 0.1.2 kept
  shipping).
- The fork tag would be created after the chain ran (the parent tag push
  freezes the gitlink CI publishes); the parent pin moved without the fork
  crate (or vice versa).

**Recovery**

- Permitted: bump to the next free number and re-verify (crates.io versions
  are immutable - never reuse a burned version).
- Prohibited: proceeding with a stale fork version; re-tagging the fork.

**Produces**

- Fork tag + parent gitlink float + headroom-fork notes. Publish.yml runs
  via `workflow_run` on Build completion (no dispatch input exists); needs:
  chain Test → Publish-Headroom-Core → Publish-Aphrodite → Publish-Hermes -
  Publish-Headroom-Core's publish step is UNREACHABLE (gated on the removed
  `workflow_dispatch publish_crates` input), so only aphrodite /
  aphrodite-hermes publish; verify in Step R6 (phase exit).
