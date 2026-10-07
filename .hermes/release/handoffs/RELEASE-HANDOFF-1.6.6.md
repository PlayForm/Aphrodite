# RELEASE-HANDOFF-1.6.6.md - temporary handoff for the next agent (merge + release)

> **Status:** Development is ready to cut the NEXT release (v1.6.6 binary / v2.2.6 plugin).
> All PR-118488 review feedback (review 5441199534) is implemented, verified, and
> committed on Development. This handoff tells the next agent exactly what to do:
> bump, re-install locally, prepare release notes/changelog/badges, merge to
> Current, release. Write by Hermes agent 2026-10-07.

## 1. Where things stand (all verified live, not assumed)

| Item                                   | Value                                                                                              |
| -------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Parent branch                          | `Development` @ `946d22ab` (clean; auto-committer swept)                                           |
| Plugin submodule                       | `Development` @ `f2cadd5` (clean; gitlink synced, no `+`)                                          |
| Parent gitlink →                       | `f2cadd5dccd440d75cbbae150582598c5d3ecd20`                                                         |
| Release worktree                       | `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite-Release` on `Current` @ `1a29d60` |
| Binary version (crates + package.json) | `1.6.5` (Cargo.toml x2, package.json)                                                              |
| Plugin version (`plugin.yaml`)         | `2.2.5`                                                                                            |
| `BINARY_VERSION` (plugin)              | `1.6.4` ← **still lags by design (bump-LAST rule)**                                                |
| crates.io max                          | `aphrodite` 1.6.5, `aphrodite-hermes` 1.6.5 (the PREVIOUS release, live)                           |
| Last tag                               | `Aphrodite/v1.6.5` + plugin `v2.2.5` (both pushed, release exists)                                 |

**What's already on Development (all committed, gates green):**

- **Review ask 1 (prefetch guard):** `crates/aphrodite/src/prefetch.rs` - `ReadOutcome::Refused` + `refuse_reason`, refuses `.env`/`auth.json`/`~/.ssh`/Hermes home/non-file; 10/10 tests. Commit `193165bb`.
- **Review ask 2a (setup never touches plugin dir):** `crates/aphrodite/src/setup/run.rs` - catalog-install guard (Hermes-home plugin dir = report-only, no symlink removal/create/writes/deletes; notice points at `bash download.sh`/`pwsh download.ps1`). Commit `193165bb`.
- **Review ask 2b (dylib refuses missing sums):** `crates/aphrodite/src/setup/dylib.rs` - missing sums / no entry = hard `Err`. Commit `193165bb`.
- **Review ask 2c (setup hints → download scripts):** `plugins/aphrodite/__init__.py` + `crates/aphrodite/templates/__init__.py` (byte-identical, drift-guard passes). Commit `193165bb` (template) + submodule `5a7a9d1`.
- **Review ask 3 (opt-in context engine + requires_hermes):** description "opt-in context engine"; "default-on" claim fixed (registration is `APHRODITE_CONTEXT_ENGINE=1`, TOML `compression.context_engine` defaults true); `requires_hermes: ">=0.20.2"` (first Hermes release with `pre_tool_call` `modify`, evidenced from `~/.hermes/hermes-agent`: commit `d083b8559` → tag `v2026.8.16` → pyproject 0.20.2). Commits `f30cb79b` (parent write.rs) + `f2cadd5` (submodule plugin.yaml).
- **Review ask 4 (Disclosure):** both `plugin.yaml` + `write.rs` description carry "Disclosure - pre_tool_call auto-backgrounds long terminal/process commands (modify: background=true, notify_on_complete=true) and rewrites chained terminal commands when enabled."
- **Config 1:1 audit (sa-4):** both TOMLs now mutually consistent + 1:1 with parsers; dead `[templates.*]` subtree + `compression.prefetch` removed; `chain_split_min/max_segments` + `defaults.api_url/model` added; unwired keys annotated. In `193165bb`.
- **Docs audit (sa-5):** 21 doc files fixed (thresholds, tool counts 13, classifier 30, no hot-reload claims, `bash download.sh` explicit install, endpoints table, version stamps 1.6.5). In `193165bb`.
- **Skills:** lessons recorded (`51a7dda6`) incl. the submodule detached-HEAD recovery pitfall (this wave's scar) and the release child-push fallback.

**Gates (recorded):** `cargo check` 0 errors · `cargo test -p aphrodite --lib` 410 passed / 0 failed / 1 ignored (network-gated) · config 16/16 · setup 6/6 · prefetch 10/10 · fmt `--check` clean · drift-guard byte-identical · ruff clean · prettier green on owned docs.

## 2. Next agent's task (in order)

### Step 1 - Bump (binary track, ONE ceremony, on Development main worktree)

Per `aphrodite-release-workflow` ledger: claim the NEXT numbers - binary **1.6.6**, plugin **2.2.6** (1.6.5/2.2.5 are live, never reuse). Bump together:

1. `crates/aphrodite/Cargo.toml` version → 1.6.6
2. `crates/aphrodite-hermes/Cargo.toml` version → 1.6.6 AND its dep pin `aphrodite = { path = "../aphrodite", version = "1.6.6", ... }`
3. `package.json` version → 1.6.6
4. Parent `README.md` badges → release `v1.6.6` (line ~16), plugin `v2.2.6` (line ~18)
5. Submodule `plugins/aphrodite/plugin.yaml` version → 2.2.6 + install_message `aphrodite v2.2.6` (and plugin README badge → v2.2.6 - DO NOT repeat the badge miss)
6. **`BINARY_VERSION` stays `1.6.4` until tag time** (bump-LAST rule; the 1.6.5 pin is what's installed - a 1.6.6 pointer before assets exist 404s downloads)
   Verify: `cargo check -p aphrodite -p aphrodite-hermes` (dep-pin ceremony), grep for stale `1.6.5`/`2.2.5` in live manifests (CHANGELOG/release-notes history is fine).

### Step 2 - Re-install locally (full)

Per `aphrodite-development`: `cargo build --release -p aphrodite -p aphrodite-hermes` (exit 0, expect v1.6.6), `cp target/release/aphrodite` + `libaphrodite_hermes.dylib` → `~/.hermes/aphrodite/binaries/`, then run `./target/release/aphrodite setup` from the FRESH binary (writes loader + BINARY_VERSION 1.6.6 + manifest). Expect `setup` exit 1 ONLY on the final proxy-start port conflict (9797 held by the live session) - install itself succeeds. Verify `aphrodite --version` → v1.6.6, `~/.hermes/plugins/aphrodite/plugin.yaml` version 1.6.6, shim carries the new "bash download.sh / pwsh download.ps1" hints.

### Step 3 - Release notes + changelog

- `CHANGELOG.md` (parent root): add `## v1.6.6 - ...` entry at top (date 2026-10-07). Cover: prefetch path guard (review), setup catalog-install guard, mandatory sums refusal, setup hints, opt-in context engine + requires_hermes 0.20.2, Disclosure, config/docs 1:1 cleanup.
- Release notes draft: `.hermes/release-notes/vNEXT-draft.md` (replace the 1.6.5 draft) - follow `RELEASE-TEMPLATE.md` (Summary, Changes, Infrastructure/Verification, What Ships = full 4-target matrix, Links). No `{PENDING}`/`DO NOT PUBLISH`.
- No `BINARY_VERSION` bump in these files.

### Step 4 - Merge to Current + release (the ceremony)

Per `aphrodite-release-flow` (read `.hermes/skills/aphrodite/aphrodite-release-flow/SKILL.md` + `references/release-steps.md`):

1. **I1 B4 audit** before ANY sync (workflow triggers, .gitmodules, gitlinks).
2. **I2 plugin sync FIRST** (submodule-first): in `plugins/aphrodite`, `git checkout Current`, `git merge --squash Development`, commit `release: sync v2.2.6 from Development`, push. (Prefer working in the release worktree's submodule - `Aphrodite-Release/plugins/aphrodite` - and note: if the submodule is detached, recovery = verify parent-is-tip, `git branch -f <branch> <sha>`, `git checkout <branch>` - NEVER rewrite; see skill lesson.)
3. **I3** validate plugin commit; **I4** parent sync in the release worktree (`Aphrodite-Release`, already on Current): squash Development, restore identity paths (`.gitmodules .github/workflows plugins/aphrodite`), float gitlink to plugin tip, `release: sync v1.6.6 from Development`, push. Watch for the auto-committer's transient gitlink-bump commits (they land mid-ceremony; final state is what matters).
4. **R3** `BINARY_VERSION` bump LAST: only AFTER the release assets exist, set `plugins/aphrodite/BINARY_VERSION` → 1.6.6, commit+push plugin Current, float parent gitlink.
5. **R4** tag `Aphrodite/v1.6.6` (annotated, `-m "Aphrodite/v1.6.6"`) at the release-sync commit + push → Build.yml runs (release + 12 assets) → Publish.yml via workflow_run publishes crates. **PAUSE at the "Ready for approval" boundary** (irreversible events) - present the approval summary, get explicit user approval before pushing the tag.
6. Post-release: plugin repo manual release `v2.2.6` (`gh release create v2.2.6 --repo PlayForm/Aphrodite-Hermes ...` - makes it "Latest", which the README badge reflects) + re-pin the catalog PR (`plugin-catalog/aphrodite.yaml` sha → new plugin commit, version 2.2.6) in `~/Developer/Application/NikolaRHristov/hermes-agent` + update PR body + comment.

## 3. Pitfalls / scars to honor

- **Submodule must ALWAYS be on its branch** (`Development` in the main worktree, `Current` in the release worktree). A commit on a detached submodule HEAD lands on no branch and push fails - verify `git -C <submodule> symbolic-ref -q HEAD` after any wave. Recovery: append-only (`git branch -f` + checkout), never rewrite.
- **`PLAYFORM_RELEASE_PAT`** (Release env secret, fine-grained, contents:write on the CHILD only) failed auth once - if Finalize's child push fails again, the manual fallback is: generate in-tree SHA256SUMS.txt from the release's per-target assets, commit+push child Current via SSH, tag the plugin, float the gitlink; then re-run Build so Finalize no-ops. A fixed PAT exists now.
- **GitHub runner queue**: if jobs sit queued (started_at null) it's runner availability, not the workflow - wait or retry; don't cancel.
- **fmt**: `cargo +nightly-2026-05-01 fmt --all` is the gate toolchain; CI checks it.
- **Drift-guard**: `plugins/aphrodite/__init__.py` ≡ `crates/aphrodite/templates/__init__.py` byte-identical, always (`cp` after edit).
- **Auto-committer is a standing third writer**: never `git commit` yourself in the repo (subagents included); verify with `git log`/`git submodule status`, not `git status`.
- **Never commit/tag/push unasked; hold GitHub actions until the user says go.**

## 4. Files most likely to be touched by the next agent

- `crates/aphrodite/Cargo.toml`, `crates/aphrodite-hermes/Cargo.toml` (version + pin), `package.json`
- `README.md` (badges), `plugins/aphrodite/plugin.yaml` (version, install_message, README badge), `CHANGELOG.md`
- `.hermes/release-notes/vNEXT-draft.md`
- Ceremony: git refs only (Current sync commits, tag `Aphrodite/v1.6.6`, plugin tag `v2.2.6`), `BINARY_VERSION` LAST
- Catalog PR: `~/Developer/Application/NikolaRHristov/hermes-agent/plugin-catalog/aphrodite.yaml`

## 5. Session addendum (2026-10-07, pre-merge verification - committed on Development)

Everything below was live-verified this session and is COMMITTED on Development
(the auto-committer swept the bump); the merge ceremony was NOT started.

- **Bump committed:** parent `2675ada6` (`1031f238` "Stage v1.6.6 bump, changelog,
  and release handoff" + `2675ada6` gitlink bump) - crates 1.6.6, hermes pin 1.6.6,
  package.json 1.6.6, README badges v1.6.6/v2.2.6, CHANGELOG v1.6.6 entry,
  vNEXT-draft.md rewritten. Submodule `90b460f` on Development - plugin.yaml 2.2.6 +
  install_message + plugin README badge v2.2.6. `BINARY_VERSION` still 1.6.4
  (bump-LAST honored). Gitlink synced, no `+`.
- **Gates run:** `cargo check -p aphrodite -p aphrodite-hermes` green (7.24s);
  `cargo build --release` green (17.56s) -> `aphrodite v1.6.6`; stale-version sweep
  clean; drift-guard byte-identical; prettier clean on CHANGELOG + vNEXT-draft.
- **Local re-install done:** fresh binary + 2 dylibs in `~/.hermes/aphrodite/binaries/`
  (atomic-rename trick needed - in-place `cp` over the RUNNING proxy's mapped file
  triggers taskgated SIGKILL "Code Signature Invalid"; always `cp` to `.new` + `mv`).
  `aphrodite setup` from fresh binary: catalog-install guard live (report-only, review
  ask 2a), installed BINARY_VERSION pin 1.6.6, plugin registered, exit 1 only on the
  expected 9797 port conflict. Installed loader plugin.yaml is OLD-format until the
  catalog re-pin (step 6) - report-only by design, not a miss.
- **B4 audit (I1): PASS** - zero identity hits; Current gitlink `387c6eb` = plugin
  Current tip; Development gitlink `90b460f` = plugin Development tip.
- **Gate R7 (R2) at Source/Current:** tag push -> Build.yml (release + 12 assets +
  Finalize child push) -> Publish.yml `workflow_run` (cargo publish aphrodite +
  aphrodite-hermes); headroom-core publish step unreachable; Auto.yml pushes Current.
- **Fork (I5) recommendation:** `aphrodite-headroom-core` 0.1.3 live; fork delta = 2
  dep-only commits (fastembed->5, crate refresh) past `aphrodite-v0.10.0`. NO fork bump
  this cycle (matches 1.6.4 precedent; a pin to unpublished 0.1.4 would fail
  Publish-Aphrodite). RELEASE-CYCLE.md §5 is stale (says 0.1.3 unpublished) - update at
  ceremony time as doc-only reconciliation.
- **Next agent starts at I2** (plugin sync `release: sync v2.2.6`) in the release
  worktree on Current; pause at tag (R4) per the approval boundary. All 6 review asks
  and 4 commitment points from the original handoff remain unchanged.
