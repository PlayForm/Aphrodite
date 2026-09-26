# Aphrodite Release Ceremony (dual-line model)

Condensed operational spec; the full action-by-action version is
`release/RELEASE-METHODOLOGY.md` (canonical - read it before ANY release
work). Two lines: **Development** (workshop, all tests + CI, append-only, no
release tags) and **Current** (distributed, test-free, tags + GitHub releases
ONLY here). Content descends by snapshot transplant (Phase A); ascends
selectively by `cherry-pick -x` (Phase B). The plugin submodule syncs FIRST
in every phase (bottom-up: the parent gitlink must reference the plugin tip).

## Phase A - RELEASE (Development -> Current)

1. **A0 preconditions** (verify or ABORT): clean trees, versions bumped
   everywhere on Development, release notes finalized, gates green, tags free,
   **B4 branch-identity audit clean on BOTH refs**.
2. **A1 plugin sync (ALWAYS FIRST)**: checkout Current, `merge --squash
Development`, selective pick (keep version bump/docs/real changes; discard
   dev-only), commit `release: sync vX.Y.Z`, push Current. Tag `vX.Y.Z` at
   the END of the whole release.
3. **A2 parent sync**: checkout Current, `merge --squash Development`,
   RESTORE branch-owned identity (`git checkout HEAD -- .gitmodules .github/
workflows plugins/aphrodite .hermes bench`), float the plugin gitlink,
   verify (identity diffs EMPTY, no `+` in `git submodule status`), commit,
   push Current, tag `Aphrodite/vX.Y.Z`, `gh release create --notes-file`
   (note staged at `.hermes/release-notes/release-notes-vX.Y.Z.md` - Dev-side,
   never crosses; the Maintain/ staging pattern ended at v1.6.2).
4. **A3 what ships vs never ships**: crates/, plugins (as gitlink), docs,
   README, CHANGELOG, Maintain/scripts (non-bench), formatter configs ship;
   `.hermes/` (incl. skills/), bench/, tests/, auto-release.sh never ship.
5. Tag AFTER the final tip is settled (re-tagging re-fires the chain).
   Build.yml fires on `refs/tags/Aphrodite/*` -> 12 artifacts (4 targets x
   bin + dylib + SUMS); Finalize asserts the matrix, commits the in-tree
   `plugins/aphrodite/SHA256SUMS.txt` INSIDE the child submodule and pushes
   child Current with the fine-grained PAT `PLAYFORM_RELEASE_PAT`
   (child-scoped; GITHUB_TOKEN cannot push the child - the 1.6.3 403), then
   TAGS the plugin `v$(plugin.yaml version)` at the child tip (annotated,
   UNSIGNED - re-sign manually if the signed-tag convention must hold;
   idempotent). Publish.yml no longer fires on the tag: it chains off Build
   via `workflow_run` (completed + success + `Aphrodite/v*` head_branch) and
   publishes `aphrodite` + `aphrodite-hermes` (the headroom-core publish step
   is unreachable - it must already be live on crates.io); Bump-Plugin-Gitlink
   then floats the parent gitlink to the child Current tip and pushes Current
    - branch-protected (GH006) degrades with a loud warning; the bump becomes
      a manual admin push.

## Phase B - SYNC-BACK (Current -> Development)

1. **B0 preconditions**: both repos on Development, plugin clean (or phantom
   cleared), **B4 audit clean**.
2. **B1 plugin sync-back**: checkout Development, clear any phantom
   self-referential gitlink (`git rm --cached plugins/aphrodite`), `merge
--squash Source/Current`, VSCode selective pick (keep formatter-aligned
   `__init__.py` + real fixes; **Development KEEPS tests** - unstage the
   `D tests/...` entries), commit, push.
3. **B2 parent sync-back**: `merge --squash Source/Current`, restore
   Development's own identity (`.gitmodules`, workflows with the TEST job,
   gitlinks), selective pick, verify identity diffs EMPTY, commit, push.
4. **B3 selection benchmark**: code fixes + formatter config + .vscode +
   README badges + release notes transfer; workflow triggers, .gitmodules
   branch fields, gitlinks, test-free deletions, Current-only CI edits stay.
5. Re-run the B4 audit (I11) after the sync.

## B4 - Branch-identity audit gate (MANDATORY pre-sync/pre-tag, I11)

Read-only ref access only - NEVER checkout the other branch. `git fetch
Source`, then scan BOTH refs:

1. Workflow triggers + push targets (`branch:` / `branches:` in
   `.github/workflows`) must match the branch they live on; a push target of
   the OTHER branch = **LEAK** (Auto.yml class). The sanctioned exception:
   Auto.yml's heartbeat pushes to Current on BOTH copies (touches only the
   CI-ignored `.github/Update.md` path).
   Publish.yml has no `branch:`/`branches:` keys (workflow_run-chained): its
   gitlink-bump push target is a runtime run-step push (`HEAD:${BRANCH}`, default
   Current). The step-4 keyword scan must treat run-step branch strings
   (`BRANCH=Current`, "Current is branch-protected") as content, not identity.
2. `.gitmodules` branch fields match their branch (plugin -> Development on
   Development / Current on Current; vendors -> Current on both, by design).
3. Gitlink targets resolve to their branch (`git -C plugins/aphrodite branch
--contains <sha>`).
4. Keyword scan for `Current|Development` in workflows + .gitmodules (docs
   describing the dual-line model are content, not identity - do not flag).

Any hit = **ABORT the ceremony**; record in `release/CEREMONY-AUDIT.md`, fix
the offending branch separately, re-run clean, then proceed.

**Open finding (2026-09-18, CEREMONY-AUDIT): Current's copy of `Auto.yml`
line 68 pushes `branch: Development` - an ABORT-class LEAK. Development's own
copy pushes `branch: Current` (sanctioned). Fixing Current is a separate
pending task - see `HANDOFF.md`.** Minor notes: `.githooks/*` still tracked on
Current (stale comment + divergence), `auto-release.sh` has a guarded
detached-HEAD fallback to `Current` (latent risk, not active).

## Tagging + invariants (immutable)

- Tags exist ONLY on Current. The PARENT tag is cut first (it fires the
  chain); the plugin tag is then created automatically by Build.yml Finalize
  at the child tip (from `plugin.yaml` version), and the parent gitlink is
  floated to that tagged child tip by Publish.yml's Bump-Plugin-Gitlink.
  Bottom-up holds for SYNC order and the gitlink float, not for tag order.
- The one rule: plugin moves first in both directions; identity files never
  cross; tests/bench/dev-scaffolding only exist on Development; tags only on
  Current; every transplant is a reviewable staged snapshot, never an
  automatic merge.
- Invariant checklist I1-I11 + D1/D2 in `release/RELEASE-METHODOLOGY.md`
  PART 7 (incl. I11 branch-identity audit, D2 chain-split opt-in OFF in
  shipped config).

## Operational notes

- **Formatter contract**: rustfmt.toml uses nightly-only unstable options;
  CI pins `nightly-2026-05-01`; stable rustfmt silently ignores them.
  `crates/aphrodite/templates/__init__.py` must stay byte-identical to
  `plugins/aphrodite/__init__.py` (drift-guard) - format the plugin first,
  then `cp` to the template.
- **Auto-committer**: the user runs one that sweeps working-tree changes into
  commits; it can stage a phantom self-referential gitlink inside submodules.
  Never fight it - clear with `git rm --cached <path>`, verify with
  `git log` not just `git status`, keep Current-only work inside the ceremony
  window.
- **BINARY_VERSION** (`MC3-01 +tag+guard`) is bumped LAST - it is a live
  download pointer; bumping it before the release assets exist 404s every
  download (the exact 2026-09-17 failure mode).
- **Release notes** are authored per `.hermes/release/RELEASE-TEMPLATE.md` and
  finalized at `.hermes/release-notes/release-notes-vX.Y.Z.md` (Dev-side,
  NEVER crosses - `gh release create --notes-file` consumes it from the
  ceremony checkout). The Maintain/ staging (`D@R4-01..05`) ended at v1.6.2.
- **Child pushes in CI** use the fine-grained PAT `PLAYFORM_RELEASE_PAT`
  (Release-environment secret, contents:write on the child repo only);
  GITHUB_TOKEN cannot push the child (the 1.6.3 403).
