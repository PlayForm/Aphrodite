---
name: automatic-release-pipeline
description: "Use when automating Aphrodite releases on a schedule. Scheduled Release Manager: version bump + CHANGELOG + GitHub Release, complementing Build.yml/Publish.yml."
version: 1.5.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: release
category_taxonomy: release/automatic-release-pipeline
date: 2026-09-25
metadata:
    hermes:
        tags: [release, automating, scheduling, bumping, changeloging, publishing]
        related_skills: [github-actions-maintenance, aphrodite-release-workflow]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
owns:
    - The scheduled Release Manager workflow design (templates/Release.yml, templates/cliff.toml, templates/Release.tpl, templates/dependabot.yml)
    - The release-readiness verification ladder (registry, tag, installer probes)
    - The rules for what may move into the existing publisher; it must NOT modify that publisher (Build.yml + Publish.yml)
depends_on:
    - aphrodite-orientation (preflight gate before any release mutation)
    - aphrodite-release-workflow (Gate R7 trigger audit)
    - github-actions-maintenance (action version pinning)
supersedes: []
verification:
    source_of_truth:
        - Live release state, probed: crates.io API, gh api repos/PlayForm/Aphrodite/releases, git ls-remote --tags, plugins/aphrodite/download.sh - not this prose
        - .github/workflows/Build.yml and .github/workflows/Publish.yml (publisher triggers)
mutation_level: mutate
---

# Automatic Release Pipeline

Cut Aphrodite releases on a weekly/monthly cadence: version bump + CHANGELOG
+ GitHub Release, driven by a scheduled "Release Manager" workflow. The
manager complements, never replaces, the existing publisher: the Build.yml
artifact matrix + the Publish.yml crates chain.

## When to Use

- Aphrodite releases must be cut on a recurring schedule with no manual step per dependency update
- CHANGELOG.md and the GitHub Release notes must stay in sync with zero manual work
- The repo is ~80% automated and the only manual gap is: bump version + write CHANGELOG + create the GitHub Release
- The user asks for a "crate + npm wrapper that generates release notes" (this already exists as git-cliff)

## Prerequisites

- Read `.github/` and the version manifests first to map the EXISTING chain:
  Dependabot open/auto-merge, conventional commits (`fix:`, `feat:`), and the
  current publish trigger (`Aphrodite/v*` tag push → Build.yml; Publish.yml
  `workflow_dispatch publish_crates=true`; `on: release: created`).
- Confirm the delivery chain actually works BEFORE automating it. A workflow
  file that runs on tag push is not a release; the file proves only that a
  pipeline is defined. Workflow files, manifests, and badges are claims; live
  probes are facts. Run the Gate R7 trigger audit from
  `aphrodite-release-workflow` at the commit to be tagged before relying on
  any trigger claim.
- git-cliff (Rust crate `git-cliff-core` + npm wrapper) is the standard tool
  for conventional-commit changelog generation; do not invent the pattern
  from scratch.

## How to Run

- Research first: preserve findings OUTSIDE the repo (e.g.
  `~/.hermes/tmp/<topic>/`); do not edit, commit, or rebuild the target repo
  until the user says "apply"/"implement".
- Scaffold only: leave `// TODO: scaffold` placeholders, no implementation logic.
- Verify "repo untouched" before claiming research-only: `git status
  --porcelain` and `git diff --stat` must both be empty.
- Privacy gate: before any commit/release, the leak-scan must be empty -
  `git ls-files -z | xargs -0 grep -Il -E 'ssh://|<username>|<workspace>'` →
  expect NO matches (extend the pattern with any personal tokens you know).

## Stop-if / Recovery

- **Stop-if**: the user has not said "apply"/"implement" for the findings.
  **Recovery**: keep the findings in `~/.hermes/tmp/<topic>/`; do not edit,
  commit, or rebuild the target repo.
- **Stop-if**: a tag push is gated on explicit user OK and the OK has not
  come. The pipeline's ability to publish is not the gate being lifted.
  **Recovery**: report the exact command and do not push.
- **Stop-if**: the leak-scan returns matches. **Recovery**: scrub the matched
  tokens, re-run the scan, then commit or release.
- **Stop-if**: a delivery claim has no probe output behind it. **Recovery**:
  run the ladder (Procedure 2) and attach the output to the claim.

## Tool Comparison

| Tool             | Verdict |
| ---------------- | ------- |
| semantic-release | Autonomous but OWNS publishing; fights an existing `release: created` design |
| release-please   | PR/manifest mode; works WITH `release: created` (keeps a human brake) |
| changesets       | Needs per-PR changeset authoring; bad for hands-off dep-only releases |
| standard-version | DEPRECATED - never use |
| git-cliff        | Best for notes-only generation; pairs with any publisher |

Recommendation: scheduled workflow + git-cliff for notes + the existing
publisher (Build.yml + Publish.yml) via `workflow_call` or a plain tag push.
The verdicts are finished observations from `references/research.md`; re-probe
the tooling landscape before relying on them (CLAIM).

## Procedure

### 1. Map the manual gap

1. Does Dependabot open AND auto-merge dep PRs? If yes, dep churn is free.
2. Are commits conventional? Any standard tool classifies them automatically.
3. What triggers publish today? For Aphrodite: an `Aphrodite/v*` tag push
   reaches Build.yml (4-target artifact matrix) and Publish.yml (`cargo
   publish` for `aphrodite` + `aphrodite-hermes`); `aphrodite-headroom-core`
   is dispatch-gated. Re-verify at the tag commit (Gate R7).
4. The only manual gap is usually: bump version + write CHANGELOG + create
   the GitHub Release. Close it with a scheduled "Release Manager" - do NOT
   replace the existing publisher.

### 2. Verify the chain actually delivers (ladder, cheapest → decisive)

1. Registry path: `curl -s https://crates.io/api/v1/crates/<name> -H "User-Agent: <ua>" | jq -r '.crate | .max_version + " " + .created_at'` - proves `cargo install <name>` resolves. Per crate (`aphrodite`, `aphrodite-hermes`).
2. GitHub path: `gh api repos/PlayForm/Aphrodite/releases --paginate -q '.[].tag_name'` AND `git ls-remote --tags <remote>`. Empty output is not a quiet success: it means no release and no tag. Empty pipelines exit 0 - check the OUTPUT, not the exit code.
3. Decisive: run the user-facing installer script itself (`plugins/aphrodite/download.sh <version> <target>`). A 404 on the asset URL proves release assets were never uploaded even when the installer resolved its pinned `BINARY_VERSION`.
4. Follow every README badge/link to its target - badges pointing at an empty releases page are the report card.

A crate being published is not GitHub assets being uploaded. crates.io and
GitHub release assets are INDEPENDENT delivery paths that fail independently:
a crate can be published while every `download.sh`/badge is still 404.

### 3. Design the conflict-safe Release Manager

Scheduled release + Dependabot auto-merge + manual dispatch are multiple
concurrent writers to the version manifest. Essentials:

- **Mutual exclusion**: give Release.yml AND Dependabot's Merge job the SAME
  `concurrency.group` with `cancel-in-progress: false` - they queue, never
  overlap.
- **Tag LAST**: `git pull --rebase` right before bumping; create the tag after
  the rebase, immediately before push, with a bounded 3-retry that re-absorbs
  + `git tag -f`.
- **Idempotent publish**: a registry version-exists guard before any publish
  step - `curl -sL https://crates.io/api/v1/crates/<crate>/<version>` → skip
  if the JSON contains `"num"` (registry versions are immutable).
- **GITHUB_TOKEN gotcha**: a release created with the default `GITHUB_TOKEN`
  does NOT trigger another workflow's `on: release: created`. CALL the
  publisher via `workflow_call` or inline the publish.
- **Empty-release guard**: skip if zero commits since the last tag.
- **Dependabot conventional fix**: `commit_message: { prefix: "chore",
  include: "scope" }` in `dependabot.yml`, or dep bumps bucket as "unknown"
  in the changelog.
- **OIDC provenance**: keep `environment: Release` + `id-token: write`; for
  crates.io trusted-publishing crates use `rust-lang/crates-io-auth-action@v1`
  (see references/publish-from-github.md).

### 4. Run tag-triggered workflows correctly

- **Dispatch on the TAG, not the branch**: a `workflow_dispatch` on a branch
  sets `github.ref` to that branch, so `if: startsWith(github.ref,
  'refs/tags/...')` jobs are skipped.
- **`gh workflow run W --ref <tag>` executes the workflow file AT the tag
  commit** - a workflow fix is not live until the tag is re-pointed (delete
  remote ref, delete local, re-create at new HEAD, push).
- **Prefer the separated release pattern**: the Release job creates the
  release once with no files; each matrix leg attaches ONLY its own artifacts
  (`fail_on_unmatched_files: true`) with its own per-target
  `SHA256SUMS-<target>.txt`; a Finalize job asserts every target's assets and
  fails loudly (the Aphrodite Build.yml layout: 4 targets × binary + dylib +
  sums = 12 assets). This avoids release-creation races and the
  combined-checksum merge job (download-artifact collides on a shared
  checksum filename across legs).
- **Actions disabled**: if a tag exists and `gh run list` shows 0 runs ever,
  check `gh api repos/O/R/actions/permissions` → `{"enabled": false}`. Add the
  repo to the org selected list, then enable with `gh api -X PUT
  repos/O/R/actions/permissions -F enabled=true` (`-F` sends a typed boolean;
  `-f` sends the string "true" and the API rejects it with 422).
- **Re-fire without re-pushing**: `gh workflow run Build.yml -R O/R --ref
  <tag>` (requires `workflow_dispatch` on the workflow).

### 5. Rehearse destructive release steps before the real run

Release scripts and workflow commands perform irreversible actions:
dependency bumps that rewrite manifests, `git push --delete`, tag
delete/recreate, `gh release delete`, and `cargo publish`. Rehearse with a
dry-run harness before any real run: clone the repo into a throwaway sandbox
(`git clone --local --no-hardlinks`), stub PATH with wrappers that log every
mutating command as `[DRY]` instead of executing it, and run the script
against the sandbox. The real repo and network are never touched. Stub
taxonomy and the crates.io Publish.yml pattern:
`references/release-dry-run.md`.

## Pitfalls

- **Never run `git tag <name>` bare in automation** - always
  `git tag -a <name> -m "<msg>"`, because when the repo sets
  `tag.gpgsign` / `tag.forcesignannotated` (or a `git tag` alias), a plain tag
  becomes annotated+signed and opens `$EDITOR` for a message, killing the
  script mid-flow or leaving a half-created tag. Then verify
  `git rev-parse <name>^{}` points at the intended commit. Tag-shape
  matching, interrupted-session, mis-tag, and remote-evidence rules:
  `references/tag-and-test-details.md`.
- **Never run Intel-macOS legs in a hosted matrix** - `macos-13` runners
  queue indefinitely on the free tier. Build `x86_64-apple-darwin` on a
  macos-14/macos-latest ARM64 runner with the target installed via the
  dtolnay/rust-toolchain `targets:` input; clang cross-compiles macOS x86_64
  from arm64 natively.
- **Check the output, never the exit code** - empty pipelines exit 0; parse
  registry JSON with `jq` in `terminal` commands, never `python3 -c` (blocked
  by the runtime hook).
- **`gh` `--json` fields are camelCase** (`tagName`, `databaseId`); REST-doc
  snake_case names silently return nothing from `jq` queries.
- **Confirm git state with live git, never the session summary** - a compacted
  summary can report "0 commits / unborn repo" while commits exist. Run
  `git log --oneline -3 && git ls-files | wc -l && git status --porcelain`
  before any destructive op.
- **Tag push may be gated on explicit user OK** - the pipeline's ability to
  publish is not the gate being lifted. If the answer does not come, report
  the exact command and do not push.
- **Version consistency across manifests says nothing about whether the tag
  exists.** Three manifests agreeing is not a pushed tag; the remote tag set
  is `git ls-remote --tags <remote>` raw output.
- **Freeze the release branch after the tag push until the tag-triggered
  workflows finish** - a gitlink-bump job that commits on detached HEAD at the
  tag and pushes `HEAD:<branch>` dies with non-fast-forward the moment ANY
  commit lands on that branch after the tag push; an unrelated fix is enough.
  The bumped gitlink is the release's last mile: if the job dies, land the
  same bump manually (add the submodule gitlink, commit, push); the
  workflow's detached commit is dangling and harmless. Check `git submodule
  status` for a `+` after any release whose workflow had a late fix pushed
  during the run.
- **A Dependabot config must live at `.github/dependabot.yml`, never under
  `.github/workflows/`** - every file in `.github/workflows/` is parsed as a
  GitHub Actions workflow, so a misplaced config yields 'Invalid workflow
  file' plus a failing action run on every push to the branch. Fix:
  `git mv .github/workflows/Dependabot.yml .github/dependabot.yml`, then
  verify the next push stops producing the phantom run.
- **`set -euo pipefail` on a release script requires `|| true` on the
  idempotent-delete steps** (`git tag -d`, `git push --delete Source`,
  `gh release delete`) - they fail on a first-time version and would abort
  the flow before the real work.
- **crates.io publish auth is per-crate: token OR trusted publishing, never
  both** - check the crate's Trusted Publishing setting before wiring
  `CARGO_REGISTRY_TOKEN`. Token-gated crates reject OIDC; trusted-publishing
  crates reject tokens with `403 ... can only be published using Trusted
  Publishing`. For trusted publishing use `rust-lang/crates-io-auth-action@v1`
  (id: auth) + `id-token: write` +
  `CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}` - cargo's native
  OIDC can say `error: no token found` and never exchange, so always use the
  explicit action. The registration (repo + workflow filename + environment)
  must match the job exactly.
- **A release that path-depends on a vendored fork crate silently skips the
  fork publish when the fork's own version is unchanged.** CI's version-check
  reads the fork crate's Cargo.toml version, finds it already in the crates.io
  index, and skips the publish - while the parent crate (path+version dep,
  path stripped on publish) publishes against the OLD published content.
  Nothing fails, so the gap is invisible: the fork's latest tree exists only
  as a local path dependency. Bump the fork's crate version AND the parent's
  pin in the same cycle; at release time, check that the fork's Cargo.toml
  version is not still the previously published number.
- **Publish ordering when a fork crate's version CHANGES: dispatch the fork
  publish BEFORE the tag push.** The fork's publish step is gated
  `workflow_dispatch` (tag pushes cannot reach it), and the parent's
  tag-triggered publish resolves the dependency from the registry at publish
  time - the new fork version must already be live or the parent publishes
  against the old content (or fails). After the dispatch succeeds, the tag
  push re-runs the fork step and fails red on re-publish - known cosmetic
  behavior, not a regression; verify with timestamps (dispatch publish time <
  tag push time) and the parent crate's dependency list.
- **Verify a publish came from the right commit: the run's headSha and the
  published crate's dependency list, not the release page.**
  `gh run view <id> --json headSha` must equal the expected commit;
  `https://crates.io/api/v1/crates/<name>/<version>/dependencies` is the
  authoritative nested-dep view (the plain version endpoint does not nest
  deps).
- **npm registry propagation lags the publish run** - `npm view` can keep
  showing the OLD version for minutes after the workflow logged
  `+ @scope/pkg@newver`. The run log (with its sigstore provenance logIndex
  line) and the DIRECT registry endpoint are the ground truth:
  `curl -s https://registry.npmjs.org/@scope/pkg/<version>` → HTTP 200, and
  the package root endpoint's `dist-tags.latest`. A stale `npm view`/`npm
  install` check minutes later is a cache/propagation artifact, not a failed
  publish - re-check the direct endpoint before concluding, and never "fix" a
  publish that already succeeded.
- **Tag-push release bodies are amendable after creation; tags and crates are
  not.** If the release is created by an action (e.g.
  softprops/action-gh-release) with an empty/draft body, attach finalized
  notes with `gh release edit <tag> --notes-file <file>` and re-run the edit
  after note corrections (version-claim fixes) - never recreate the release
  to fix a body.
- **When re-aligning a version "everywhere", derive the surface from the
  PREVIOUS release commit, then sweep with grep and re-grep until zero
  hits.** `git show <prev-release-commit> --stat` lists the exact file set
  the ceremony bumps (manifests, badges, workflow comments, changelog,
  child-plugin files, gitlink) - use it as the checklist. Claims hide in:
  root README badges (one repo can carry badges for MULTIPLE tracks - binary
  AND plugin - on the same line), docs/README claims that must move in
  lockstep on every line, classification/governance docs, release notes (file
  AND the GitHub release body), and the submodule's own
  manifests/README/install_message. Bump the obvious manifest first, then
  `grep -rn '<old-version>'` across the repo AND its submodules AND every
  worktree line - a sweep that ends with a non-zero grep is not done.
- **Dependabot auto-merge can merge fully-red PRs when checks aren't required
  - and a version-only bump can be invalid across a major version** (ureq 2.x
  `tls` feature does not exist in 3.x, which exposes only `_rustls`/`_test`).
  A red `cargo test` on the PR branch proves the manifest is broken; revert
  the requirement bump (keep the old major) and gate the auto-merge on
  required checks before re-approving bumps.
- **Remove a local release script only after proving no workflow or skill
  invokes it.** A convenience script (e.g. `auto-release.sh`) that duplicates
  what Actions already do is dead weight; delete it with `git rm` ONLY when
  `grep -rn '<name>' .github/workflows/` is empty (check the exit code, not
  just output) AND no active ceremony/skill reference depends on it (grep
  `.hermes/skills/`). Update the classification/governance ledger row that
  described it (mark `✝ deleted <date>`, note that the bump sequence is now a
  manual ceremony checklist item) - historical docs/notes referencing the
  script are RECORDS, not dependencies, and stay untouched. Verify after
  removal: `grep -rln '<name>'` across the repo returns only
  docs/notes/history.
- **Release pipeline principle: Actions-driven only.** Tag push → build
  workflow (builds + release assets + in-tree checksums committed inside the
  submodule) + publish workflow (registry + parent→child gitlink bump);
  nothing local runs the release. A script that no workflow calls is not part
  of the pipeline - remove it.
- **A crates.io Publish job needs the repo's `Release` environment (and token
  secret when the crate is token-gated) before the first run** -
  `gh api repos/O/R/environments` shows `total_count: 0` on a fresh repo and
  the job dies on secret resolution; check and create before wiring
  Publish.yml.
- **When automating a public repo, keep only the org author + public URLs in
  git identity** - do not re-introduce ssh/personal remotes; scaffold sets
  public `https://github.com/PlayForm/Aphrodite.git`; keep it.

## Claim-to-Test Matrix

| Claim | Test that would falsify it | Status |
| --- | --- | --- |
| The tag-triggered chain fires (`Aphrodite/v*` → Build.yml; Publish.yml `workflow_dispatch publish_crates=true`; `on: release: created`) | Run the Gate R7 trigger audit from `aphrodite-release-workflow` at the commit to be tagged | CLAIM - re-run Gate R7 at the tag commit |
| The crate is published | `curl -s https://crates.io/api/v1/crates/<name> -H "User-Agent: <ua>" | jq -r '.crate | .max_version + " " + .created_at'` - per crate (`aphrodite`, `aphrodite-hermes`) | Probe in row |
| The release and the tag exist | `gh api repos/PlayForm/Aphrodite/releases --paginate -q '.[].tag_name'` AND `git ls-remote --tags <remote>`; empty output = no release, no tag | Probe in row |
| The installer can consume the release assets | `plugins/aphrodite/download.sh <version> <target>` against a temp `BINARY_DIR`; a 404 on the asset URL = assets never uploaded even when the installer resolved its pinned `BINARY_VERSION` | Probe in row |
| crates.io and GitHub assets are independent delivery paths | probe both paths: a crate can be published while every `download.sh`/badge URL is still 404 | Probe in row |
| Every target's assets are attached (4 targets × binary + dylib + `SHA256SUMS-<target>.txt` = 12 assets) | assert the release asset list contains every target's set | Probe in row |
| The workflow fired on the tag | `gh run list` shows the run | Probe in row |
| The publish came from the expected commit | `gh run view <id> --json headSha` equals the expected commit; `https://crates.io/api/v1/crates/<name>/<version>/dependencies` is the authoritative nested-dep view | Probe in row |
| A release created with the default `GITHUB_TOKEN` triggers another workflow's `on: release: created` | `gh run list` after the release shows the downstream workflow | CLAIM - fix is `workflow_call` or inlining the publish |
| The research-only pass left the repo untouched | `git status --porcelain` and `git diff --stat` both empty | Probe in row |
| The leak-scan is empty before commit/release | `git ls-files -z | xargs -0 grep -Il -E 'ssh://|<username>|<workspace>'` → no matches | Probe in row |
| Tool verdicts match the current ecosystem (semantic-release, release-please, changesets, standard-version, git-cliff) | re-probe the tooling landscape before relying on a verdict | CLAIM - finished observation; re-verify |

## References

- `references/release-dry-run.md` - dry-run harness for release scripts + crates.io Publish.yml pattern
- `references/dual-repo-release-ceremony.md` - manual release of a parent repo + plugin submodule: surface derivation from the prior release commit, commit patterns, tag creation, crates dispatch, verified failure modes
- `references/research.md` - tool comparison, git-cliff architecture, gap analysis
- `references/conflicts.md` - concurrent-writer taxonomy + exact mitigations
- `references/release-readiness-verification.md` - empirical ladder for proving a release is consumable
- `references/tooling-landscape.md` - ecosystem map (cargo-release, changesets, semantic-release, release-please)
- `references/tag-and-test-details.md` - tag-shape matching, mis-tag handling, stale-binary rebuilds, doctest gate scoping, HERMES_AGENT_SRC CI env

## Templates

- `templates/Release.yml` - weekly scheduled, conflict-safe Release Manager
- `templates/cliff.toml` - git-cliff config (Fix/Add/Change/Dependencies grouping)
- `templates/Release.tpl` - GitHub Release body Tera template
- `templates/dependabot.yml` - conventional-commit `commit_message.prefix` fix