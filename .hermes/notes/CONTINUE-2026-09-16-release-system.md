# Session Continuation - 2026-09-16 (release system, fully tested)

Read this FIRST to resume. This session built + tested the COMPLETE dual-track
release system end-to-end (v1.4.3 released through Current, test-free policy,
formatter alignment). Everything below is verified with real tool output.

## State at session end

| Repo | Branch | HEAD | Notes |
| ---- | ------ | ---- | ----- |
| Aphrodite (parent) | `Development` | `d70d9f8` | clean tree; vendor/headroom gitlink dirty (`M vendor/headroom` - auto-committer bump pending) |
| plugins/aphrodite (submodule) | `Development` | `c0bc01d` | clean; phantom gitlink cleared before switch |
| Source/Current (parent) | `Current` | `2f3b461` | fully pushed; tag `Aphrodite/v1.4.3` at `36dec3c` (NOT at final tip - deliberate) |
| Source/Current (plugin) | `Current` | `f8f4cdf` | pushed (ruff reformat of __init__.py) |

## v1.4.3 release - DONE (but NOT fully released)

- **Current branch carries the release**: `2f3b461` = final release-line state (test-free,
  latest deps, common-markdown directives, formatter config, template byte-sync).
- **Tag `Aphrodite/v1.4.3`** exists at `36dec3c` (shim-template fix commit) - DELIBERATELY
  NOT moved to the final tip; the user said "the tag can stay". GitHub release body HAS been
  edited with the finalized notes (verified live).
- **Build/Publish runs** on the tag: Build succeeded on `be11670`; subsequent forced re-tags
  re-triggered Build (in_progress) + Publish (in_progress) - watch them next session.
- **Publish workflow fires on `Aphrodite/v*` tags but does NOT publish crates** unless
  `workflow_dispatch` with `publish_crates: true` (manual, deliberate). Dependency order:
  aphrodite-headroom-core → aphrodite → aphrodite-hermes. NEVER `cargo publish` locally.

## The test-free release policy (user's standing rule, this session)

Current ships **NO tests, NO bench, NO dev scaffolding**:
- Test files deleted from Current (`966cb41`); `crates/aphrodite/tests/`, root `tests/`, bench
  examples gone. All tests + CI run on Development only.
- `Check.yml` on Current: Test job REMOVED (`3589b33` fixed a dangling `Test:` YAML key that
  broke the workflow - "Unexpected value ''" at L108); `[Current]` triggers kept. Development
  keeps full CI.
- `.hermes/`, `bench/`, `skills/` NEVER cross to Current (protected paths + pre-commit guard
  lesson below).
- Benchmark numbers (`compression_aware_task`) measured but belong to **1.5.0** release notes,
  NOT 1.4.3 (cutoff decision).

## The 1.4.3 vs 1.5.0 boundary (dual-track)

- **1.4.3 = pure 1.4.2→1.4.3 prep**: release pipeline, directives, hooks, plugin hardening
  (PR #7/#8 Windows), benchmark TOOLING + docs. Cutoff commit `73b3272` (last pure-prep;
  verified chain-split-free: `git grep chain_split 73b3272` empty).
- **1.5.0 = everything after**: chain-split core `475917d` + invisibility `d26dd89` + directive
  prose `8521b27` + Tier-1 teaching loop `85771ee` + Tier-3 error hints `50ff098` + retrieve
  fix `da57c79` + classifier fix + `.gitignore` work. Strategy doc:
  `.hermes/notes/RELEASE-STRATEGY-1.4.3-1.5.0.md` (332 lines, authoritative).
- **Directives DID ship in 1.4.3** (user's final call: "pull the directives into 1.4.3, leave
  chains for 1.5.0"). Three locations all good markdown: `crates/aphrodite/src/builtin_directives/`
  (6 files), root `directives/` (5 files, was old-style on BOTH branches - sync from plugin),
  `plugins/aphrodite/directives/` (5 files, already good at `175c104`).

## Formatter alignment (this session's big lesson)

**VSCode Alt+F vs CI drift root cause**: repo `rustfmt.toml` uses nightly-only unstable options
(`space_after_colon = false` etc.). Stable rustfmt / rust-analyzer internal formatter IGNORE
those → space-after-colon. CI pins `nightly-2026-05-01` → honors them.

**Fixes committed to Current (`c5cbed8` + `589807a`)**:
1. `.vscode/settings.json` (now TRACKED, was gitignored-dead): `rust-analyzer.rustfmt.overrideCommand`
   → `["rustup","run","nightly-2026-05-01","rustfmt","--edition","2024"]` - VSCode now uses the
   SAME nightly rustfmt as CI → byte-identical.
2. `ruff.toml` made explicit: line-length 100, double quotes, space indent, LF,
   docstring-code-format, `extend-exclude = ["crates/aphrodite/templates/**"]`.
   NOTE: `indent-width` and `skip-magic-trailing-comma` are NOT valid ruff options (TOML parse
   error) - removed.
3. `.gitignore` IDE section fixed: `!.vscode/` + `.vscode/**` + `!.vscode/settings.json` (the
   bare `!.vscode/settings.json` under excluded dir is a no-op).
4. `rustfmt.toml` ignore += `crates/aphrodite/templates/` (shim must stay byte-identical).

**Python shim template lesson (CI failure `setup::tests::test_hermes_plugin_shim_template_matches_live`)**:
`crates/aphrodite/templates/__init__.py` MUST be byte-identical to `plugins/aphrodite/__init__.py`
(setup.rs asserts). CLI `ruff format` = VSCode Alt+F exactly (verified: 100-style, `1 file
reformatted` → `already formatted`). **Order matters: format the PLUGIN first, then copy to
template** (`cp plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`). Template
is a generated mirror, never edited directly.

## Release ceremony - the working sequence (both directions)

**Development → Current (release, snapshot transplant, no cherry-picks down):**
1. Prep on Development (bump, notes, gates). S sync first, then P.
2. S: `merge --squash Development` on S-Current → commit `release: sync vX.Y.Z` → push.
3. P: `merge --squash <cutoff>` on Current → restore protected paths
   (`.gitmodules .github/workflows plugins/aphrodite .hermes bench skills`) → float gitlink →
   verify (I9: `git diff HEAD -- protected` empty; `git submodule status` no `+`) → commit
   `release: sync vX.Y.Z from Development` → push → tag on Current only → GitHub release
   (`--notes-file`, never inline backticks) → back to Development.
4. Direct promotion (no promote/vX.Y.Z branch) - PR variant documented in strategy doc §8.

**Current → Development (sync-back, for hotfixes AND release-line fixes):**
- Plugin FIRST, then parent (submodule-first: parent's gitlink must reference the plugin's tip).
- `git merge --squash Source/Current` STAGES only (no commit) → VSCode selective pick per file
  (unstage = `git restore --staged`, discard = `git restore`).
- Restore Development's OWN identity files (`.gitmodules`, `.github/workflows` with
  `[Development]` triggers) - they must NOT transfer from Current.
- Benchmark for what transfers: shared content (code/config/docs/formatter) YES; branch-owned
  identity (workflow triggers, .gitmodules branch fields) NO.

## Auto-committer races (user runs it; DON'T fight, verify state)

- Phantom self-referential gitlink recurs in SUBMODULES: `git rm --cached <path>` inside the
  submodule, re-check `git ls-files -s` has no 160000 entry. Hit in plugins/aphrodite twice and
  vendor/headroom once this session.
- It sweeps uncommitted work into commits (including our ruff reformat → plugin `f8f4cdf`).
- Verify with `git log`/`git submodule status`, not just `git status`.

## NEXT SESSION - immediate steps (in order)

1. **Finish the v1.4.3 release**: watch the tag Build/Publish runs (in_progress at session end).
   Decide whether to move tag `Aphrodite/v1.4.3` from `36dec3c` → `2f3b461` (final tip) and
   force-push. Plugin tag `v2.1.3` on S-Current was NEVER created (user: "tagging is done at the
   end" - do it as part of finishing).
2. **Sync Current work back to Development** (user approved, both repos on Development now):
   a. Plugin: `git -C plugins/aphrodite merge --squash Source/Current` → VSCode review → commit.
   b. Parent: `git merge --squash Source/Current` → restore Development identity
      (`.gitmodules .github/workflows plugins/aphrodite vendor/headroom`) → VSCode review → commit.
   c. Bring: formatter config (ruff.toml, .vscode/settings.json, rustfmt.toml ignore), template
      byte-sync, root `directives/` good markdown, README plugin badge v2.1.3, finalized release
      notes. Leave: test-free deletions? NO - Development KEEPS tests (that's its job).
   d. Note: parent `vendor/headroom` gitlink is dirty (`M vendor/headroom`) - bump to the
      pushed `84c8d117` (ml-cluster pin) as part of this.
3. **Save the release methodology as a skill** (user asked): full phases, both directions,
   formatter alignment, test-free policy, badge checklist (release + plugin + crates.io - plugin
   badge drifts silently, was stale v2.1.2 → fixed v2.1.3), gitlink/phantom traps.
4. **1.5.0 prep** (later): bump 1.4.3→1.5.0 on Development, Tier-1 + Tier-3 + retrieve/classifier
   fixes ride along, benchmark numbers in notes.

## Open items / known issues

- Tag `Aphrodite/v1.4.3` at `36dec3c` ≠ final tip `2f3b461` - deliberate, decide next session.
- Plugin tag `v2.1.3` not yet created on S-Current.
- `vendor/headroom` gitlink dirty on Development (bump pending).
- auto-release.sh removed from Current (release works without it - Build.yml fires on tags,
  no script calls); references scrubbed from CHANGELOG/docs/release-notes.
- Publish workflow on Current tag: fires but no crates published without manual dispatch -
  verify its in_progress run's conclusion.