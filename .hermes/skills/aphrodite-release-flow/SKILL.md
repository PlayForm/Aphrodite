---
name: aphrodite-release-flow
description: "Use when releasing or hotfixing Aphrodite (parent + plugin), or verifying/debugging the live CCR engine after a bump."
version: 2.0.0
platforms: [macos]
tags: [aphrodite, release, branch, cherry-pick, submodule, hotfix]
---

# Aphrodite Release Flow

Concrete instance of `branch-flow-protocol` for PlayForm/Aphrodite (parent P)
and its plugin submodule S (`plugins/aphrodite` → PlayForm/Aphrodite-Hermes,
remote `Source`). Replaces the deleted `aphrodite-branch-release-flow`; read
the protocol for the axioms/state machine, this skill carries the instance
declaration and the ceremony.

## Topology & identity

- **Development** = workshop (accumulates, never rewritten). **Current** =
  distributed line; tags + releases live ONLY here. The two branches are
  different products, not mergeable twins.
- **Branch-owned identity (protected, NEVER cross a transplant):**
  `.gitmodules` branch field, `.github/workflows/*` triggers
  (`[Development]` vs `[Current]`), and the `plugins/aphrodite` gitlink.
  I9 check: `git diff HEAD -- <protected>` must be empty after any sync.
- **Three version tracks, never conflated:** binary `1.4.x` (parent
  Cargo.tomls, package.json, README badge), plugin `2.1.x` (S plugin.yaml),
  and S's `BINARY_VERSION` file = which binary release the plugin pairs with
  (what `download.sh` fetches). Bump procedure (verified): the TWO parent
  crates + the `aphrodite-hermes` Cargo.toml's `aphrodite = { path = ..,
version = "X" }` dep pin must move TOGETHER in one ceremony (cargo check
  fails otherwise); package.json carries the binary version too. Bump all
  parent spots, then plugin.yaml + its install_message + README badges
  (they drift - the badge may lag two minors), then BINARY_VERSION. A LOCAL
  BINARY_VERSION bump ahead of the tag is safe when the binaries already
  exist in `~/.hermes/aphrodite/binaries` (`_ensure_binaries` no-ops) - the
  "bump LAST" ceremony rule applies at TAG time, not local prep. Two agents
  with disjoint ownership (parent vs plugin submodule) do this cleanly;
  grep for the OLD version strings after, and update any test that pins
  them (the plugin's tests rarely do).
- **Every file is classified by phase/kind/layer**: the repo's
  `.hermes/classification/` (TAXONOMY.md + four pass files + README index,
  295 files) encodes the dual-line flow as an HPC code `{K}{P}{L}-{N}`
  with ceremony annotations (`→C`/`→D` destination, `+tag`, `+bump`,
  `+float`, `+guard`, `∅` never-crosses, `@R` ritual-only, `✝` absence =
  deleted process whose absence IS the ceremony rule, `@A` archival,
  `∅ (untracked)` regenerated-per-line). Consult it when deciding what
  crosses a transplant instead of re-deriving the ship-table; new
  removals/absences get recorded there as ceremony rules.
  **Maintaining the classification:** a new pass = 4 agents in two
  phase-pairs (DEV: engine/build + knowledge/tests/bench; CUR:
  plugin/loader + release-infra/identity) with disjoint file ownership,
  each writing one `{PHASE}-{scope}.md` into `.hermes/classification/`
  with the shared table schema `Path | HPC code | Phase | Ceremony
behavior | Halted-process resume note`. Pass files mark proposals
  `(proposed, NOT yet written to TAXONOMY.md)`; folds bump TAXONOMY to a
  new minor version (v0.1 → v0.2) and land in the amendment log. When a
  pass flags a discrepancy between RELEASE-METHODOLOGY.md and the actual
  tree (stale formatter contract, Publish.yml gate), VERIFY the claim
  against the tree yourself, then fix the methodology doc in place -
  never append an 'UPDATE:' note under the stale text. All
  `.hermes/**/*.md` must stay prettier-clean (repo md style: tabs, width
  100, proseWrap preserve).

## Release ceremony (Development → Current) - submodule first

1. Prep on Development: work, bump binary `1.4.x`, gates (`cargo clippy -p
aphrodite -- -D warnings`, `ruff check plugins/aphrodite/`, tests).
   Commit on Development. Do NOT tag here.
2. S sync first (the parent's gitlink must reference the released plugin):
   S-Current → `git merge --squash Development` → commit `release: sync
vX.Y.Z` → push.
3. P sync: `git merge --squash <cutoff>` on Current → restore protected paths
   (`git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite`)
   → float the plugin gitlink to S-Current tip → verify I9 + `git submodule
status` shows no `+` → commit `release: sync vX.Y.Z from Development` →
   push.
4. Tag **on the release-sync commit**, push the tag. Build.yml fires on
   `refs/tags/Aphrodite/*` (branch-agnostic).
5. GitHub release from the tag (notes via --notes-file, never inline
   backticks). Return to Development.

## Hotfix on Current - the fast path (user preference)

Hotfixes are worked DIRECTLY on Current - no Development round-trip, no full
ceremony. The user drives every commit/tag/push decision; never commit, tag,
or push unasked.

1. Fix + commit on Current. Bump the binary patch to the NEXT number - but
   check crates.io first (below); a claimed number is never reused.
2. **`BINARY_VERSION` bump LAST, always.** It is a LIVE distribution pointer:
   a plugin whose BINARY_VERSION names a release that does not exist yet
   errors on every download. Bump it only after the release assets exist
   (tag pushed AND Build completed) - then float the parent gitlink to that
   plugin commit. The user explicitly requires this ordering.
3. Tag on the release-sync commit, never on a later style/cleanup tip - a
   post-release ruff reformat or gitlink cleanup does NOT move the tag
   (plugin v2.1.3 sits on the `release: sync v2.1.3` commit, not the
   reformat commit that followed it).
4. Version bumps ride the release; the plugin submodule's `BINARY_VERSION`
   and the parent gitlink float are the LAST commits of a hotfix cycle.
5. Pick the hotfix up to Development later with `git cherry-pick -x` - see
   the sync-back ceremony below.

## Sync-back (Current → Development) - selective cherry-pick, after a hotfix

Bringing Current's release-line work back to Development is a SELECTIVE
cherry-pick, never a merge: Development has diverged with its own work, and
Current's tree is test-free with dev-scaffolding absent.

1. Submodule FIRST (bottom-up rule), then parent. List what Current has
   that Development lacks: `git log --oneline Development..Source/Current`
   in each repo separately.
2. Decide per commit: PICK real fixes (setup.rs changes, config-template
   refresh, docs URL fixes, release-notes finalization, version bumps that
   ride the line). SKIP gitlink-only bumps (`chore: bump plugin submodule to
<sha>` - the gitlink is branch-owned, Development floats its own),
   style-only commits on files Development has since rewritten, and release
   snapshots that re-add directories Development deliberately removed
   (e.g. `directives/`) or delete test files (Development keeps tests).
3. No hooks to disable: the entire `.githooks/` set was REMOVED (2026-09-17)
    - `core.hooksPath` unset in parent AND submodule (default `.git/hooks`),
      `package.json` `prepare` stripped (it re-installed hooks on every npm
      install), `.gitattributes` hook lines dropped. Never re-create the hooks
      or the prepare script: they were the phantom-gitlink resurrection vector.
4. Cherry-pick in CHRONOLOGICAL order with `-x` (the version bumps then
   apply in sequence).
5. Conflict-resolution rule: for a release-line fix take the PICKED side's
   SEMANTICS but the repo's own formatting (nightly rustfmt: tabs,
   `space_after_colon = false` - the pick's raw style often differs;
   patch-tool rustfmt warnings about unstable options are noise, not errors).
6. After EVERY `--continue`, grep the whole staged file set for leftover
   markers: `git diff HEAD~1 HEAD --name-only | xargs grep -l '<<<<<<<'` -
   a second conflict region in the same file can survive the first
   resolution and get COMMITTED. If one slipped in: fix, `git add`,
   `git commit --amend --no-edit`.
7. Restore branch-owned identity after any pick that touches the gitlink:
   `git checkout HEAD -- plugins/aphrodite`.
8. Verify before pushing: protected paths unchanged vs pre-pick HEAD
   (`git diff <pre-pick-HEAD> HEAD -- .gitmodules .github/workflows
plugins/aphrodite` empty); every picked commit's files exist in HEAD
   (`git cat-file -e HEAD:<file>`); `git submodule status` shows no '+'; no
   160000 phantom in the submodule.
9. Push may report "Everything up-to-date" because the auto-committer
   already pushed as commits landed - confirm the remote tip with
   `git log Source/Development`, not the push output.

An EMPTY submodule sync-back is a correct outcome, not a failure: Current
commits already fixed in Development (same change), superseded (style-only
on rewritten files), or rejected by design (release snapshot re-adding
removed content / deleting tests) mean Development is ahead - skip all of
them and verify the remaining diff is empty.

## Pre-tag readiness checks (do BEFORE claiming a version or tagging)

- **crates.io may already have your version**: `curl -A <ua>
https://crates.io/api/v1/crates/<crate>` → `max_version`. If the number
  you are about to bump to is already published (a parallel release won the
  race), claim the NEXT number instead. The crates.io API rejects requests
  without a User-Agent.
- **Release-asset contract**: the assets `setup.rs`'s download path and
  `download.sh`/`download.ps1` fetch must EXIST in the release - or the
  consumer must degrade gracefully. Audit asset names against the live
  release asset list (Build.yml publishes `aphrodite-<t>` +
  `libaphrodite_hermes-<t>` only). Optional artifacts (the core
  `libaphrodite` cdylib, used only by external embedders) must be
  best-effort in the consumer - warn + continue, never abort setup - so a
  404 on an unshipped asset cannot brick a `cargo install` + setup flow.
- **Package READMEs render on crates.io**: the crate dir READMEs
  (`crates/<crate>/README.md` - what cargo auto-includes), NOT the root
  README, are what crates.io shows. Relative links there render as
  `blob/HEAD`. Before publishing, make every link absolute
  `tree/Current` URLs in ALL package READMEs and the root.
- **Embedded templates drift**: `crates/aphrodite/templates/*` are baked
  into the binary via `include_str!` (`setup.rs` CONFIG_TEMPLATE, shim). A
  stale template means fresh `aphrodite setup` writes a config missing keys
  the engine now reads (poll_worker, chain_split, navigation, preview
  tables). Before release, diff the embedded templates against the current
  live config and refresh. The shim `templates/__init__.py` must stay
  byte-identical to the live plugin `__init__.py` (setup.rs asserts it).
- **Verify what a plain tag push triggers**: Publish.yml's publish steps
  for `aphrodite`/`aphrodite-hermes` carry `|| startsWith(github.ref,
'refs/tags/Aphrodite/')` - a plain tag push DOES attempt crates.io
  publishing; only `headroom-core` is truly dispatch-gated. The
  methodology's 'manual, deliberate only' claim is stale on this point -
  check the workflow's current gates before tagging if opt-in publishing
  is intended.

## Live-engine verification (post-bump / "is it working?")

The user verifies the running engine after every version bump; the plugin's
orientation banner is NOT the ground truth - `aphrodite_stats` is.

1. `aphrodite_stats` → version, engine_enabled, thresholds, proxy liveness;
   `aphrodite_rebuild` cross-checks binary vs plugin version (must match).
2. `aphrodite_test` (quick=1 sample, full=3: source_code/build/json) proves
   the compress→retrieve→search roundtrip; all pass = healthy core.
3. Version bumps hot-reload via the dylib in `hotreload/`; each bump
   restarts the engine and RESETS session counters (turn:0, entries 0) -
   that reset is normal, not data loss.
4. Proxies down + inline-only compression is the user's accepted state; do
   not offer proxy restarts as the default fix.
5. "Debug" means TERMINAL-level probing (processes, files, logs, db), not
   just the aphrodite_* API tools - see
   `references/engine-health-debugging.md` for the probe battery.
6. **A broken/misleading preview is a bug, not cosmetic.** The preview is
   the only thing the model sees inline - if it lies about size/shape or
   collapses a JSON payload to a fragment (`[text:1L 2B | ok]`), the agent
   concludes the tool returned nothing. The user's standing principle: the
   preview must be the FINAL, MOST IN-DEPTH, honest representation of the
   stored payload. Before touching preview code, verify empirically with
   the ctypes battery and check the dead-config levers - see
   `references/preview-quality-debugging.md` for the defect-class table,
   the battery recipe, and the validated fix shape (hint-wins + drop the
   success-bool collapse).
7. Terminal output is CCR-compressed too: scan terminal results for
   `<<<CCR:` markers and retrieve them before reading on.

## Pitfalls

- **Phantom self-referential gitlink (recurs)**: a stray mode-160000 entry
  inside a submodule pointing at its OWN commit, swept in by the
  auto-committer alongside unrelated work. Symptom: `git submodule status`
  INSIDE the submodule fails with `fatal: no submodule mapping found in
.gitmodules for path '<submodule-name>'` (the submodule has NO
  `.gitmodules`, so any 160000 entry is self-referential); the remote shows
  a nested `plugins/aphrodite` folder that should not exist, and
  `git clone --recurse-submodules` dies with `fatal: No url found for
submodule path 'X/X' in .gitmodules`. Detect inside the submodule with
  `git -C <submodule> ls-files -s | grep 160000` (a hit = phantom), then
  `git -C <submodule> rm --cached <path>` + verify the grep is empty +
  `git status` clean. Commit, push; float parent gitlinks OFF any commit
  that contains it (Development gitlinks can still point at the
  phantom-containing commit after Current is fixed - check
  `git ls-tree <ref> <path>` for 160000). The auto-committer RE-STAGES a
  fresh phantom pointing at the new HEAD after you clear it, so re-run the
  `ls-files -s | grep 160000` check after every subsequent submodule commit;
  a clean check at the end of the session is the real pass criterion, not
  one removal. (The auto-bump hooks that used to re-stage it are gone -
  see the hooks-removal pitfall - so the remaining vector is the
  auto-committer alone.)
- **Auto-committer races**: it sweeps working-tree changes (including staged
  squash sets and gitlink bumps) into commits and pushes. Never fight it;
  verify final state with `git log`/`git submodule status`, not `git status`.
  A squash staged for VSCode review can be swept mid-review - the content
  survives as a commit.
- **`git cherry-pick --continue` commits leftover conflict markers.** A
  file with TWO conflict regions: resolving the first and continuing commits
  the still-marker'd second region into the branch. Grep the committed file
  set for `<<<<<<<` after every `--continue`; fix + `git commit --amend
--no-edit` when one slipped through.
- **Empty cherry-pick is "already contained", not an error.**
  `nothing to commit, working tree clean` means the change is already in
  HEAD via an earlier pick or merge resolution - verify with
  `git diff <HEAD> <source> -- <paths>` and skip the commit instead of
  aborting in confusion.
- **The git hooks are GONE - never re-create them.** The entire `.githooks/`
  set (pre-commit, post-commit, post-checkout, post-merge, lib/*) was
  REMOVED 2026-09-17 because the auto-bump hooks were the resurrection
  vector for the phantom self-referential gitlink (see next bullet):
  `post-commit`/`post-checkout` re-staged a 160000 entry named after the
  submodule inside the submodule after every manual clear, and
  `package.json`'s `prepare` re-installed the hooks on every npm install.
  Removal = `git rm -r .githooks`, unset `core.hooksPath` in parent AND
  submodule (its value was `../../.githooks`), strip the `prepare` script,
  delete the `.gitattributes` `.githooks/*` lines, clear the phantom with
  `git -C <submodule> rm --cached <path>`. With no hooks, branch anchoring,
  gitlink auto-bump, and the commit gate are gone: submodule pins are
  verified by hand (`git submodule status` shows no '+' = I2), and a
  detached submodule HEAD is fixed manually (`git -C plugins/aphrodite
checkout Development`).
- **Submodule-first ordering is mandatory**: plugin sync before parent, so
  the parent's gitlink references the released plugin in one pass.
- **Removing a subsystem = sweep the whole tree, then record the absence.**
  Skills, profiles, s2/navigation, and the installers were each removed
  across sessions: the self-heal schema (`layout_schema.json`), embedded
  templates, config example, README tree diagrams, bench scripts, release
  skills, and the classification passes ALL carry references to the
  removed thing. Delete the code AND every reference (schema entries,
  feature gates + their cfg branches, docs, tests that probe it), then
  state the ABSENCE as a ceremony rule (e.g. 'profiles never ship',
  'skills live dev-side', 'directives ship in the binary') so a future
  session does not re-add it or treat the empty cherry-pick as an error.
  The sweep includes the FFI symbol list: removing a `#[no_mangle]`
  export (e.g. `aphrodite_hermes_list_skills` when skills were dropped)
  without updating the dylib's expected-symbol list leaves
  `aphrodite_rebuild` (and any dlsym-based check) dying with
  "missing an expected symbol" against the fresh build - grep for the
  symbol name in the tooling/check code, not just the crate's lib.rs, and
  remember the live session may still hold the OLD dylib until Hermes is
  restarted (the copied-into-place new dylib only takes effect on the
  next load).
- **The setup flow needs a key, not the plugin**: proxy spawn dies with
  "no API key configured" when `APHRODITE_API_KEY` / toml `api_key` is
  absent - Hermes' provider config is NOT reused. See
  `references/plugin-lifecycle.md` for key sourcing and install layouts.

## Related

- `branch-flow-protocol` - axioms, state machine, invariant verifier.
- `submodule-fleet-management` / `git-operations` - hook mechanics, gitlink
  hygiene.
- `references/plugin-lifecycle.md` - install layouts, uninstall procedure,
  proxy key sourcing.
- `references/engine-health-debugging.md` - live-engine probe battery,
  on-disk map, failure-chain diagnosis.
- `references/preview-quality-debugging.md` - preview defect-class table
  (envelope-unwrap false positives), empirical ctypes battery, dead preview
  config levers, validated hint-wins fix shape.
