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

Cut Aphrodite releases on a weekly/monthly cadence: version bump + CHANGELOG + GitHub Release, driven by a scheduled "Release Manager" that complements, never replaces, the existing publisher (Build.yml + Publish.yml).

## When to Use

- Aphrodite releases must be cut on a recurring schedule with no manual step per dependency update; the only manual gap is bump version + write CHANGELOG + create the GitHub Release (the "crate + npm wrapper generating release notes" role is filled by git-cliff)

## Prerequisites

- Read `.github/` and the version manifests first to map the EXISTING chain: Dependabot open/auto-merge, conventional commits (`fix:`, `feat:`), publish trigger (`Aphrodite/v*` tag push → Build.yml; Publish.yml `workflow_dispatch publish_crates=true`; `on: release: created`).
- A workflow file that runs on tag push is not a release; it proves only that a pipeline is defined. Run the Gate R7 trigger audit from `aphrodite-release-workflow` at the commit to be tagged before relying on any trigger claim.
- git-cliff (Rust crate `git-cliff-core` + npm wrapper) is the standard conventional-commit changelog tool.

## How to Run

- Research first: preserve findings OUTSIDE the repo (e.g. `~/.hermes/tmp/<topic>/`); do not edit, commit, or rebuild until the user says "apply"/"implement".
- Scaffold only: `// TODO: scaffold` placeholders, no implementation logic.
- Verify "repo untouched" before claiming research-only: `git status --porcelain` and `git diff --stat` both empty.
- Privacy gate: before any commit/release, the leak-scan must be empty - `git ls-files -z | xargs -0 grep -Il -E 'ssh://|<username>|<workspace>'` → no matches.

## Stop-if / Recovery

- **Stop-if**: the user has not said "apply"/"implement" for the findings. **Recovery**: keep the findings in `~/.hermes/tmp/<topic>/`; do not edit, commit, or rebuild the target repo.
- **Stop-if**: a tag push is gated on explicit user OK and the OK has not come. The pipeline's ability to publish is not the gate being lifted. **Recovery**: report the exact command and do not push.
- **Stop-if**: the leak-scan returns matches. **Recovery**: scrub the matched tokens, re-run the scan, then commit or release.
- **Stop-if**: a delivery claim has no probe output behind it. **Recovery**: run the ladder (Procedure 2) and attach the output to the claim.

## Tool Comparison

Tool verdicts (semantic-release, release-please, changesets, standard-version, git-cliff) are finished observations; re-probe before relying (CLAIM). Recommendation: scheduled workflow + git-cliff for notes + existing publisher via `workflow_call` or plain tag push. Table: `references/research.md`.

## Procedure

### 1. Map the manual gap

1. Does Dependabot open AND auto-merge dep PRs? If yes, dep churn is free.
2. Are commits conventional? Any standard tool classifies them automatically.
3. For Aphrodite: `Aphrodite/v*` tag push reaches Build.yml (4-target matrix) and Publish.yml (`cargo publish` for `aphrodite` + `aphrodite-hermes`); `aphrodite-headroom-core` is dispatch-gated. Re-verify at the tag commit (Gate R7).
4. The manual gap is usually: bump version + write CHANGELOG + create the GitHub Release. Close it with a scheduled "Release Manager" - do NOT replace the existing publisher.

### 2. Verify the chain actually delivers (ladder, cheapest → decisive)

Probe commands are in the Claim-to-Test Matrix and in `references/release-readiness-verification.md`. A crate being published is not GitHub assets being uploaded - the two paths fail independently.

### 3. Design the conflict-safe Release Manager

Scheduled release + Dependabot auto-merge + manual dispatch are multiple concurrent writers to the version manifest. Essentials (taxonomy: `references/conflicts.md`):

- **Mutual exclusion**: Release.yml AND Dependabot's Merge job share the SAME `concurrency.group` with `cancel-in-progress: false`.
- **Tag LAST**: `git pull --rebase` right before bumping; tag after the rebase, immediately before push, with a bounded 3-retry + `git tag -f`.
- **Idempotent publish**: guard before publish - `curl -sL https://crates.io/api/v1/crates/<crate>/<version>` → skip if JSON contains `"num"`.
- **GITHUB_TOKEN gotcha**: a release created with the default `GITHUB_TOKEN` does NOT trigger another workflow's `on: release: created` - CALL via `workflow_call` or inline it.
- **Empty-release guard**: skip if zero commits since the last tag.
- **Dependabot conventional fix**: `commit_message: { prefix: "chore", include: "scope" }` in `dependabot.yml`, or dep bumps bucket as "unknown".
- **OIDC provenance**: keep `environment: Release` + `id-token: write`; trusted-publishing crates use `rust-lang/crates-io-auth-action@v1` (pattern: `references/release-dry-run.md`).

### 4. Run tag-triggered workflows correctly

- **Dispatch on the TAG, not the branch**: a branch dispatch sets `github.ref` to that branch, so `if: startsWith(github.ref, 'refs/tags/...')` jobs are skipped.
- **`gh workflow run W --ref <tag>` executes the workflow file AT the tag commit** - re-point the tag after any workflow change (delete remote ref, delete local, re-create at new HEAD, push).
- **Prefer the separated release pattern**: the Release job creates the release once with no files; each matrix leg attaches ONLY its own artifacts (`fail_on_unmatched_files: true`) with its own `SHA256SUMS-<target>.txt`; a Finalize job asserts every target's assets. Layout: `references/release-readiness-verification.md`.
- **Actions disabled**: if a tag exists and `gh run list` shows 0 runs ever, check `gh api repos/O/R/actions/permissions` → `{"enabled": false}`; add to the org selected list, then `gh api -X PUT repos/O/R/actions/permissions -F enabled=true` (`-F` = typed boolean; `-f` sends "true" → 422).
- **Re-fire without re-pushing**: `gh workflow run Build.yml -R O/R --ref <tag>` (needs `workflow_dispatch`).

### 5. Rehearse destructive release steps before the real run

Steps are irreversible: dependency bumps, `git push --delete`, tag delete/recreate, `gh release delete`, `cargo publish`. Rehearse with a dry-run harness: clone into a throwaway sandbox (`git clone --local --no-hardlinks`), stub PATH with wrappers that log every mutating command as `[DRY]`. Stub taxonomy + Publish.yml pattern: `references/release-dry-run.md`.

## Pitfalls

- **Never run `git tag <name>` bare in automation** - always `git tag -a <name> -m "<msg>"`, because `tag.gpgsign` / `tag.forcesignannotated` (or a `git tag` alias) turns a plain tag annotated+signed and opens `$EDITOR`, killing the script. Verify `git rev-parse <name>^{}`. Tag-shape/mis-tag/interrupted-session/remote-evidence rules: `references/pitfall-details.md`.
- **Never run Intel-macOS legs in a hosted matrix** - `macos-13` runners queue indefinitely on the free tier. Build `x86_64-apple-darwin` on a macos-14/macos-latest ARM64 runner via the dtolnay/rust-toolchain `targets:` input; clang cross-compiles from arm64.
- **Check the output, never the exit code** - empty pipelines exit 0; parse registry JSON with `jq`, never `python3 -c` (blocked).
- **`gh` `--json` fields are camelCase** (`tagName`, `databaseId`); REST-doc snake_case names silently return nothing from `jq` queries.
- **Confirm git state with live git, never the session summary** - run `git log --oneline -3 && git ls-files | wc -l && git status --porcelain` before destructive ops.
- **Tag push may be gated on explicit user OK** - the pipeline's ability to publish is not the gate being lifted. If no answer comes, report the exact command and do not push.
- **Version consistency across manifests says nothing about whether the tag exists.** Three manifests agreeing is not a pushed tag; the remote tag set is `git ls-remote --tags <remote>` raw output.
- **Freeze the release branch after the tag push until the tag-triggered workflows finish** - a gitlink-bump job commits on detached HEAD at the tag and pushes `HEAD:<branch>`; any commit after the tag push makes it non-fast-forward. Recovery + `git submodule status` `+` check: `references/dual-repo-release-ceremony.md`.
- **A Dependabot config must live at `.github/dependabot.yml`, never under `.github/workflows/`** - every file there is parsed as a workflow, so a misplaced config yields 'Invalid workflow file' plus a failing run per push. Fix: `git mv .github/workflows/Dependabot.yml .github/dependabot.yml`.
- **`set -euo pipefail` requires `|| true` on the idempotent-delete steps** (`git tag -d`, `git push --delete Source`, `gh release delete`) - they fail on first-time versions.
- **crates.io publish auth is per-crate: token OR trusted publishing, never both** - trusted-publishing crates reject tokens with `403 ... can only be published using Trusted Publishing`; check the setting before wiring `CARGO_REGISTRY_TOKEN`. Full pattern: `references/release-dry-run.md`.
- **A vendored fork crate silently skips its own publish when its version is unchanged** - bump fork + parent pins in the same cycle. `references/pitfall-details.md`.
- **Fork version CHANGES: dispatch the fork publish BEFORE the tag push** - the parent resolves deps from the registry at publish time. `references/pitfall-details.md`.
- **Verify a publish came from the right commit: headSha + the crate's dependency list, not the release page.** `gh run view <id> --json headSha` must equal the expected commit; `https://crates.io/api/v1/crates/<name>/<version>/dependencies` is the nested-dep view.
- **npm registry propagation lags the publish run** - `npm view` shows the old version minutes after `+ @scope/pkg@newver`; the direct endpoint is ground truth. `references/pitfall-details.md`.
- **Tag-push release bodies are amendable after creation; tags and crates are not.** When created by an action (e.g. softprops/action-gh-release) with an empty/draft body, attach notes with `gh release edit <tag> --notes-file <file>`; never recreate it.
- **Re-align a version everywhere: derive the surface from the PREVIOUS release commit, then sweep grep to zero hits.** Hiding places + sweep: `references/pitfall-details.md`.
- **Dependabot auto-merge can merge fully-red PRs when checks aren't required - a version-only bump can be invalid across a major version** (ureq 2.x `tls` ≠ 3.x). `references/pitfall-details.md`.
- **Remove a local release script only after proving no workflow or skill invokes it.** Procedure: `references/pitfall-details.md`.
- **Release pipeline principle: Actions-driven only.** Tag push → build workflow (builds + release assets + in-tree checksums inside the submodule) + publish workflow (registry + parent→child gitlink bump); nothing local runs the release.
- **A Publish job needs the repo's `Release` environment (and token secret when token-gated) before the first run** - `gh api repos/O/R/environments` shows `total_count: 0` on fresh repos.
- **When automating a public repo, keep only the org author + public URLs in git identity** - do not re-introduce ssh/personal remotes; scaffold sets public `https://github.com/PlayForm/Aphrodite.git`; keep it.

## References

- `references/release-dry-run.md` - dry-run harness + Publish.yml pattern + auth + doctest gate
- `references/dual-repo-release-ceremony.md` - parent+submodule ceremony
- `references/research.md` - tool comparison, git-cliff architecture
- `references/conflicts.md` - concurrent-writer taxonomy + mitigations
- `references/release-readiness-verification.md` - empirical ladder + traps + separated pattern
- `references/tooling-landscape.md` - ecosystem map
- `references/pitfall-details.md` - fork-crate pair, npm propagation, version-realign hiding places, script removal, red-PR merge, tag-shape rules, stale-binary reruns, doctest gate, HERMES_AGENT_SRC

## Templates

- `templates/Release.yml` - weekly scheduled, conflict-safe Release Manager
- `templates/cliff.toml` - git-cliff config (Fix/Add/Change/Dependencies)
- `templates/Release.tpl` - GitHub Release body Tera template
- `templates/dependabot.yml` - conventional-commit `commit_message.prefix` fix

## Claim-to-Test Matrix

| Claim                                                                                                                                   | Test that would falsify it                                                                                                                                                                  | Status                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| The tag-triggered chain fires (`Aphrodite/v*` → Build.yml; Publish.yml `workflow_dispatch publish_crates=true`; `on: release: created`) | Run the Gate R7 trigger audit from `aphrodite-release-workflow` at the commit to be tagged                                                                                                  | CLAIM - re-run Gate R7 at the tag commit               |
| The crate is published                                                                                                                  | `curl -s https://crates.io/api/v1/crates/<name> -H "User-Agent: <ua>"                                                                                                                       | jq -r '.crate                                          | .max_version + " " + .created_at'` - per crate (`aphrodite`, `aphrodite-hermes`) | Probe in row               |
| The release and the tag exist                                                                                                           | `gh api repos/PlayForm/Aphrodite/releases --paginate -q '.[].tag_name'` AND `git ls-remote --tags <remote>`; empty output = no release, no tag                                              | Probe in row                                           |
| The installer can consume the release assets                                                                                            | `plugins/aphrodite/download.sh <version> <target>` against a temp `BINARY_DIR`; a 404 on the asset URL = assets never uploaded even when the installer resolved its pinned `BINARY_VERSION` | Probe in row                                           |
| crates.io and GitHub assets are independent delivery paths                                                                              | probe both paths: a crate can be published while every `download.sh`/badge URL is still 404                                                                                                 | Probe in row                                           |
| Every target's assets are attached (4 targets × binary + dylib + `SHA256SUMS-<target>.txt` = 12 assets)                                 | assert the release asset list contains every target's set                                                                                                                                   | Probe in row                                           |
| The workflow fired on the tag                                                                                                           | `gh run list` shows the run                                                                                                                                                                 | Probe in row                                           |
| The publish came from the expected commit                                                                                               | `gh run view <id> --json headSha` equals the expected commit; `https://crates.io/api/v1/crates/<name>/<version>/dependencies` is the authoritative nested-dep view                          | Probe in row                                           |
| A release created with the default `GITHUB_TOKEN` triggers another workflow's `on: release: created`                                    | `gh run list` after the release shows the downstream workflow                                                                                                                               | CLAIM - fix is `workflow_call` or inlining the publish |
| The research-only pass left the repo untouched                                                                                          | `git status --porcelain` and `git diff --stat` both empty                                                                                                                                   | Probe in row                                           |
| The leak-scan is empty before commit/release                                                                                            | `git ls-files -z                                                                                                                                                                            | xargs -0 grep -Il -E 'ssh://                           | <username>                                                                       | <workspace>'` → no matches | Probe in row |
| Tool verdicts match the current ecosystem (semantic-release, release-please, changesets, standard-version, git-cliff)                   | re-probe the tooling landscape before relying on a verdict                                                                                                                                  | CLAIM - finished observation; re-verify                |
