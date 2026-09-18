# RELEASE 1.4.6 - Current Scrub + Lockdown Plan

Status: **PLAN ONLY** - trims/lockdowns on Current are DEFERRED under a safety
hold (Current is actively downloaded/consumed; no working-tree mutation allowed
until the hold lifts). Inventory below was gathered with read-only git commands
against `Current` @ `de1852f`.

Prepared: 2026-09-18 · Superproject: PlayForm/Aphrodite

---

## 1. Baseline

- `Current` tip: `de1852f` (auto-committer has NOT advanced it - re-verify at apply time; use the latest tip if it has).
- Plugin gitlink on Current: `plugins/aphrodite` → `c0bc01dbf936be367349e3fbc88b3d16253991c0`.
- Development advanced during planning: HEAD `178791a` (gitlink `0cb6eb6e`, docs refresh bump). The 1.4.6 pin target is the Aphrodite-Hermes **1.4.6 tag**, never Development's working-copy drift.
- `Current` is already test-free: `tests/`, `bench/`, `.hermes/`, `examples/`, `FORMAT-ALIGN.md`, `s2-*` are ABSENT (scrubbed at `de1852f`).
- `.github/Update.md` is ABSENT on Current ✓ and regenerated daily by `Auto.yml` (Commit job: `echo "Update: $(date)" > .github/Update.md`, pushed to `Development`). `Check.yml` `paths-ignore`s it. **Leave deleted on Current.**

## 2. Dev-only content on Current → REMOVE (when hold lifts)

Applied as `git rm` per file/dir, left STAGED/UNCOMMITTED for the user to commit (no commits by the executor).

| Path                                                                                                                                      | Why dev-only                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.editorconfig`                                                                                                                           | Editor tooling; no build/CI role                                                                                                                                                      |
| `.prettierignore`                                                                                                                         | Formatting tooling; JS format config                                                                                                                                                  |
| `.prettierrc`                                                                                                                             | Formatting tooling                                                                                                                                                                    |
| `.vscode/settings.json`                                                                                                                   | Editor config (rustfmt/ruff formatter wiring)                                                                                                                                         |
| `.githooks/` (pre-commit, post-checkout, post-commit, post-merge, `lib/bump-submodule-gitlink.sh`, `lib/sync-submodules.sh`)              | Hooks are **permanently removed** from the release line (I2 detach/float wiring is work-line-only). Present in `de1852f` contrary to the expected state - remove, and DO NOT recreate |
| `Maintain/` (SECURITY.md, hermes_tool_output_formats.json, install.bat, install.ps1, install.sh, prometheus.yml, release-notes-v1.4.3.md) | Ops/dev dir (release notes, local installer, prometheus sample, tool-schema JSON). Not needed by a product consumer                                                                   |
| `docs/APHRODITE-HEADROOM.md`                                                                                                              | Internal fork-relationship architecture doc                                                                                                                                           |
| `docs/HEADROOM-FORK-DIFF.md`                                                                                                              | Internal fork-vs-upstream divergence catalog                                                                                                                                          |
| `docs/hermes-tool-output-schemas.md`                                                                                                      | Agent-side reference ("Hermes Agent's own tool surface rather than Aphrodite's code"); companion JSON lives in `Maintain/` (removed)                                                  |

### 2a. Follow-up doc edits (required so removal leaves no dangling refs)

After the `git rm`s, patch these tracked files (staged, uncommitted):

- `docs/README.md:20` - drop "`Maintain/install.sh`," from the macOS/Linux install blurb.
- `docs/install/README.md:41,48` - remove the "Local-clone installer" row (and its `Maintain/install.sh` / `Maintain/install.ps1` references); reword line 48 to drop `Maintain/install.sh`.
- `docs/install/macos-linux.md:85` - delete the `Maintain/install.sh` row.
- `docs/tool-relay/tools.md:17` - drop the `Maintain/scripts/verify_tool_schemas.py` verification note (script dir is part of removed `Maintain/`).

### 2b. Optional candidates (documented, NOT in the primary removal set)

No workflow references them, but they are dev-only by nature. Decision left to the user when the hold lifts:

- `.gitguardian.yaml` (secret-scanning config - dev tooling)
- `.lychee.toml`, `.lycheeignore` (link-checker config - dev tooling)

## 3. KEEP on Current (verified needed)

- `ruff.toml` - **required**: plugin Python + `Check.yml` ruff-action use it.
- `deny.toml` - **required**: `Check.yml` cargo-deny job uses it.
- `Cargo.toml`, `package.json`, `pnpm-workspace.yaml`, `pyproject.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `aphrodite.toml.example`, `profiles/example/config.yaml` - build/CI/runtime.
- `.github/workflows/*` (Auto, Build, Check, Dependabot, GitHub, Publish), `.github/FUNDING.yml`, `.github/dependabot.yml` - CI/community.
- `docs/**/*.md` public docs - keep all except the three internal files above (incl. `docs/examples/llm-view.md`, `docs/hermes-integration.md` - product-facing).
- `CHANGELOG.md`, `LICENSE`, `README.md`, `assets/`, `crates/`, `directives/`, `plugins/`, `vendor/`, `deny.toml`, `.gitignore`, `.gitattributes` (post-pin), `.gitmodules` (post-pin).
- `CONTRIBUTING.md`: absent on Current - nothing to keep; note as missing in the release checklist.

## 4. Lockdowns - CONFIRMED on Current (already in place)

- **Workflow pins: ALL SHA-pinned** (verified across all 6 workflows, no floating tags):
    - `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`
    - `dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # master (2026-07)`
    - `actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6`
    - `actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7`
    - `softprops/action-gh-release@efb35369e0ad2afab669f228072c1b0d510eae64 # v3`
    - `astral-sh/ruff-action@278981a28ce3188b1e39527901f38254bf3aac89 # v4.1.0`
    - `EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2.1.1`
    - `dependabot/fetch-metadata@25dd0e34f4fe68f24cc83900b1fe3fe149efef98 # v3.1.0`
    - `pozil/auto-assign-issue@07fe6dc0e9771842b428f5739098d6140734e226 # v4.0.0`
    - `ad-m/github-push-action@881a6320fdb16eb5318c5054f31c218aec2b324c # v1.3.0` (Auto.yml only)
- **Branch refs per-branch**: `Auto.yml` pushes to `Development` (line 68) ✓ - never retarget to Current.
- **`.gitattributes` eol pins**: `* text=auto`, `*.rs/*.py/*.toml/*.yaml/*.yml/*.md/*.sh text eol=lf`, binary types, `.github/ export-ignore` ✓.

## 5. Lockdowns - TO APPLY on Current (drafted; deferred under hold)

1. **`.gitmodules` pins** (protected path - edit directly on Current, staged/uncommitted; never destructively restructured):
    - All three submodules (`plugins/aphrodite`, `vendor/headroom`, `vendor/rtk`): change `ignore = dirty` → `ignore = all` (release line ignores submodule dirt; verifier I2 checks gitlinks).
    - `plugins/aphrodite` gitlink pinned to the **1.4.6 release commit** at tag time (submodule released FIRST, bottom-up; pin = the tagged Aphrodite-Hermes commit, currently `c0bc01d` on Current - re-verify against the 1.4.6 tag). Keep `branch = Current`, `remote = Source`, urls as-is (branch-owned identity; Development may keep `ignore = dirty` for dev convenience - identity files never cross lines).
2. **B4-style pre-sync guard note** - add `B4-SYNC-GUARD.md` at the superproject root (draft below). Add the SAME file to Development so the snapshot transplant does not delete it (it is not a protected path). Must NOT be a `.githooks` entry - hooks are permanently removed from Current.
3. **`.gitattributes` cleanup after trim** - drop the two dead lines `.githooks/* text eol=lf` and `.githooks/lib/* text eol=lf` once `.githooks/` is removed.

### Draft - `B4-SYNC-GUARD.md` (content for the file)

```markdown
# B4 Pre-Sync Guard - Aphrodite release line

Mandatory before ANY sync, transplant, or tag on `Current` (1.4.6 and later):

1. **Branch identity (I1)**: `git symbolic-ref -q HEAD` must resolve to
   `refs/heads/Current` on the distributed copy. Never run the ritual from
   `Development`, a detached HEAD, or a worktree of another line.
2. **Gitlink hygiene (I2)**: `git submodule status` shows no `+`; `git status`
   shows no `M <submodule>`. Plugin gitlink equals the released
   Aphrodite-Hermes commit.
3. **Identity files never cross (I9)**: `git diff HEAD -- .gitmodules
.github/workflows plugins/aphrodite` is empty after any transplant.
4. **Workflow triggers are line-owned (I6)**: no workflow fans out to both
   lines; `Auto.yml` targets `Development` only.
5. **Clean tree (DIRTY→ABORT)**: `git status --porcelain` is empty before the
   ritual. Uncommitted dirt is never swept into a release.
6. **No tags on Development**: tags live on `Current` only (I5/I10).
7. **Bottom-up submodules**: sync `vendor/*` and `plugins/aphrodite` pins
   BEFORE the parent transplant.

Full invariant set: see the branch-flow-protocol (I1-I10). Any failure is a
release blocker - ABORT, never proceed.
```

## 6. Ceremony steps to execute at launch (1.4.6)

1. Run `B4-SYNC-GUARD.md` checks on both lines; verify `Current` tip.
2. Release Aphrodite-Hermes (plugin) first; note the tag SHA.
3. Snapshot transplant `Development` → `Current` (submodule gitlinks synced first, bottom-up). Protected paths (`.gitmodules`, `.github/workflows/*`, `plugins/aphrodite` gitlink) are restored to Current's line-owned values after the copy - never transplanted verbatim.
4. Re-apply the dev-only scrub on Current if Development re-introduced any item in §2 during the cycle (Maintain/, .githooks, .editorconfig, .vscode/, .prettier*, internal docs) + the §2a doc-reference fixes.
5. Apply §5 lockdowns (gitmodules `ignore = all`, plugin gitlink = 1.4.6 tag SHA, `B4-SYNC-GUARD.md` present, `.gitattributes` .githooks lines removed).
6. Verify invariants I1-I10 (branch identity, gitlinks, protected paths, line-owned workflows, clean tree).
7. Tag `1.4.6` on Current, publish, verify `Current` tree == tagged content (I3).

## 7. Deferred action list (safety hold - NOT executed)

- [ ] `git switch Current` + `git rm` the §2 items (staged, uncommitted)
- [ ] §2a doc-reference patches (staged, uncommitted)
- [ ] `.gitmodules` `ignore = all` ×3 + plugin gitlink pin (staged, uncommitted)
- [ ] Add `B4-SYNC-GUARD.md` to Current AND Development (staged, uncommitted)
- [ ] `.gitattributes` .githooks line removal (staged, uncommitted)
- [ ] Optional §2b decision (.gitguardian.yaml, .lychee.toml, .lycheeignore)
- [ ] No commits, no pushes - leave staged changes for user/auto-committer

## 8. Notes / hazards

- Development's local WIP was committed by the auto-committer during planning (`a1669bd` skill/notes consolidation, `178791a` gitlink bump `47cb321→0cb6eb6e`). No gitlink drift remains uncommitted on Development; the Current pin must still be the 1.4.6 tag, not Development's pointer.
- `vendor/headroom` + `vendor/rtk` remain pinned submodules on Current (release-line deps) - do not scrub `vendor/`.
- Re-verify `Current` tip at apply time (auto-committer may advance it).
