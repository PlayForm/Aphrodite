---
name: playform-cargo-maintenance
description: "Use when bumping or publishing Aphrodite Cargo versions. Version cycle + crates.io publish gates for the Aphrodite Rust workspace (aphrodite, aphrodite-hermes, headroom fork)."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: release
category_taxonomy: release/playform-cargo-maintenance
date: 2026-09-25
metadata:
    hermes:
        tags: [release, bumping, rebuilding, retesting, publishing, verifying]
        related_skills:
            - automatic-release-pipeline
            - github-readme-generation
            - aphrodite-release-workflow
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
owns:
    - The bump → rebuild → retest cycle for the workspace crates (crates/aphrodite, crates/aphrodite-hermes, vendor/headroom)
    - The crates.io publish mechanics for the workspace crates
    - The publish-gate enforcement: no cargo publish, tag push, push, or release until the user explicitly lifts the gate
    - It must NOT change the release gates themselves (version ledger, Gate R7, artifact contract - owned by aphrodite-release-workflow)
depends_on:
    - aphrodite-release-workflow (Gate R7 trigger audit at the tag commit)
    - github-readme-generation (README restyle checklist)
supersedes: []
verification:
    source_of_truth:
        - Live release state, probed: crates.io API max_version (UA'd curl), git ls-remote --tags, built binary --version output - not this prose
        - .github/workflows/Build.yml and .github/workflows/Publish.yml (publisher triggers, re-verified at the tag commit)
mutation_level: mutate
---

# PlayForm Cargo Maintenance (Aphrodite)

Version cycle and release mechanics for the Aphrodite Rust workspace: `crates/aphrodite` (core), `crates/aphrodite-hermes` (plugin binary), and the vendored fork `vendor/headroom` (`aphrodite-headroom-core`). Repo: PlayForm/Aphrodite monorepo, branch `Development` (public line) / `Current`, remote `Source`.

`aphrodite-release-workflow` owns the release gates (version ledger, Gate R7, artifact contract). This skill owns the bump → rebuild → retest cycle and the crates.io publish mechanics. A working publish path is not the gate being lifted: the path proves only that the pipeline CAN publish; the gate stays with the user until the user explicitly lifts it (see Publish gate).

## When to Use

- "bump the version", "minor/major/patch bump", "rebuild and retest"
- Wiring or checking crates.io publishing for the Aphrodite crates

## Procedure (bump → rebuild → retest)

1. **Bump in the lockstep set** (repo convention - all must move together):
    - `crates/aphrodite/Cargo.toml` → `version = "X.Y.Z"` (`build.rs` stamps it into the binary at build time)
    - `crates/aphrodite-hermes/Cargo.toml` → `version = "X.Y.Z"` AND its `aphrodite = { path = "../aphrodite", version = "X", ... }` dep pin. The pin must equal the crates.io version, or `cargo publish` fails.
    - `package.json` → `version` (carries the binary version)
    - `CHANGELOG.md` → new `## X.Y.Z` entry at the top with `### Change` / `- Bumped package version from A.B.C to X.Y.Z.`
    - `README.md` → the release badge `message=vX.Y.Z` (badge labels must stay truthful; the help-text block is version-free and needs no edit)
    - Generate the CHANGELOG entry from git history: `git log <last-tag>..HEAD` plus `git -C vendor/headroom log` when the fork moved. Verify the change set against the diff before writing.
    - Multi-crate extras (plugin `plugin.yaml` + `BINARY_VERSION`, `fixtures/<crate>_version.json`, both-repo README badges) and the version-hit classification matrix: `references/worked-detail.md`
2. **Rebuild**: `cargo build --release`. The binary only reports the new version after a rebuild; `build.rs` re-stamps at build time, not on edit.
3. **Refresh installed binaries**: `cargo install --path crates/<name> --force` (workspace root Cargo.toml is a virtual manifest - point at the crate dir). Plugin tree byte-consistency recipe + verify-all-four: `references/worked-detail.md`.
4. **Retest** with machine-checkable exits: `./target/release/aphrodite --version` AND the installed `aphrodite --version` must both print the new version. A live run must exit 0 and its newest window header must now read `from <previous-tag> to last commit`. The tool's own output is the proof.
5. **Re-check git state**: an external sync often COMMITS the bump locally (working tree goes clean while `git log` gains a commit). Verify `git show HEAD:<file>` carries the new version before claiming anything was lost, kept, or published.

## Release execution (only after the publish gate is explicitly lifted)

1. Commit + push the PLUGIN submodule FIRST (its post-commit hook bumps the parent gitlink), then stage everything in the parent and push. Verify `git submodule status` shows no `+` before tagging.
2. Annotated tag `Aphrodite/v<version>` on the parent + push. The tag format is ALWAYS `Aphrodite/v<semver>` (annotated; never deviate). The workflows key off `startsWith(github.ref, 'refs/tags/Aphrodite/')`, and the tag must exist in that exact form or the release jobs never run. This triggers Build.yml (GitHub release with per-target assets) AND Publish.yml's tag-reachable `cargo publish` for `aphrodite` + `aphrodite-hermes`. Re-verify the trigger table at the tag commit (Gate R7 in `aphrodite-release-workflow`); the headroom fork publish is `workflow_dispatch`-gated only.
3. When the headroom fork version moved, dispatch its publish explicitly BEFORE the parent tag push: `gh workflow run Publish.yml -f publish_crates=true --ref Current`, then watch both runs with `gh run watch <id> --exit-status` (background + notify_on_complete; never poll in a loop).
4. Verify EXTERNALLY, never from the workflow's green check alone:
    - crates.io: `curl -s -H "User-Agent: <contact>" https://crates.io/api/v1/crates/<name>` → `max_version` equals the new version (UA-less requests get dropped).
    - Release assets: `gh release view <tag> --json assets --jq '.assets[].name'` → expected per-target binary + dylib + checksum set (12 assets).
5. Re-dispatches are idempotent: the workflow's index-version skip (`already_published`) makes a rerun after a partial failure safe.

## Publish gate (USER RULE)

- A version bump is NEVER authorization to publish. "don't re-publish" is a standing gate: no `cargo publish`, no tag push, no push, no release - until explicitly instructed. Local commits by the sync are fine; remote-facing actions are not.
- Publishing from GitHub is NOT uniform across the fleet. PlayForm/Aphrodite's Publish.yml publishes `aphrodite` + `aphrodite-hermes` on `Aphrodite/v*` tag push with `CARGO_REGISTRY_TOKEN`. Crates gated to crates.io TRUSTED PUBLISHING (OIDC) reject API tokens with a 403. Check the target repo's `.github/workflows/` AND the crate's crates.io Trusted Publishing settings before assuming either way. Pattern details: `references/publish-from-github.md`.

## Stop-if / Recovery

- **Stop-if**: the publish gate is not explicitly lifted. **Recovery**: report the exact command (`cargo publish`, `git push Source <tag>`) and do not run it.
- **Stop-if**: `git submodule status` shows a `+` before tagging. **Recovery**: commit + push the plugin submodule first, then re-run `git submodule status` until it shows no `+`.
- **Stop-if**: a release run failed, the workflow file was fixed afterwards, and the tag still points at the old commit. **Recovery**: `git tag -f` + `git push --force Source <tag>` so the tag-push workflow runs at the fixed commit. When the release already exists, do NOT move the tag; complete the missing artifact manually (see submodule-checkout pitfall, anatomy in `references/worked-detail.md`).
- **Stop-if**: a gitlink bump is needed and only a branch-ref dispatch is available. **Recovery**: bump the gitlink manually; the tag-push-gated job is skipped on branch refs (see Pitfalls).

## Pitfalls

One-line rules; full recipes, worked instances, and probes in `references/worked-detail.md`.

- **A workspace crate whose manifest carries `version = "X.Y.Z"` TWICE - crate version AND a path-dep pin (`crate = { path = "../crate", version = "X.Y.Z", ... }`) - a bare-string patch on `version = "X.Y.Z"` hits the FIRST occurrence (the pin) and silently leaves the crate version stale; `cargo build` still succeeds, drift invisible until `--version` or `cargo publish`.** Match each occurrence with surrounding context (`name = "<crate>"\nversion = ...` for the crate version; the `path = "../..."`
 line for the pin), then `grep -c 'version = "X.Y.Z"' <file>` and confirm the expected count (2 with a pin, 1 otherwise) before building.
- **A CI "already published" check is a silent-skip trap, not a safety gate: it goes GREEN and publishes NOTHING while the content advanced.** Compare the fork's content delta since the LAST PUBLISHED COMMIT (`git log <last-published-commit>..HEAD` inside the fork/submodule), not just the version string. If content moved, bump the crate version FIRST; a live version in the index means the next dispatch skips.
- **A parent crate whose `path + version` dep pins a fork crate needs the fork version LIVE on the registry before ANY publish path runs** (`cargo publish` strips the `path` key; the registry resolves the pinned version at publish time). Sequence: fork bump → fork tag → fork publish (dispatch) → parent tag. Publishing the parent first fails on the missing dependency version; re-publishing after a success errors red ("already published") - plan around both.
- **A number already on crates.io can never be claimed again.** Before committing fixes on top of a released line, check `max_version` with the UA'd curl (`curl -s -H "User-Agent: <contact>" https://crates.io/api/v1/crates/<name>`; UA-less requests are dropped) and bump to the next patch FIRST if the working-tree version already equals it.
- **CI-only deltas after a tag do NOT warrant a bump.** Workflow YAML fixes and gitlink-pointer updates riding `Current` leave the published artifacts byte-identical; re-releasing burns a version number on nothing. Audit with `git diff --stat <tag>..HEAD` plus `git diff --name-only <tag>..HEAD`; when every changed path is under `.github/` (plus the submodule gitlink), the release stands and the fixes ride along on the NEXT tag. Bump only when crate/plugin SOURCE content moved.
- **crates.io renders the PACKAGE-dir README** (cargo auto-includes `crates/<name>/README.md`, not the root README). Relative links (`](docs/...)`) render as `blob/HEAD` on the crates.io page - make every link an absolute `tree/Development` URL in ALL package READMEs of a workspace.
- **Version POINTERS on the distributed line (`BINARY_VERSION`, download pins) advance LAST**, only after the release they point at is confirmed by `git ls-remote --tags` and the crates.io `max_version` curl. A pointer is a live distribution link; bumping it early makes consumers fetch a not-yet-published artifact and error. The parent release tags first; the pointer bump + gitlink float is the final step.
- **The sync that commits your bump may also fast-forward/force-reset the branch mid-task.** Re-run `git log` + `git show HEAD:<file>` immediately before any git-level claim; `git status` alone is not enough.
- **Version claims must be verified in the BUILT binary's output (`--version`), not just the source files** - a stale binary reports the old version after a source-only bump.
- **When the bump touches README content**, apply the `github-readme-generation` restyle checklist (badges, em-quads, GFM alerts, byte-exact examples).
- **A failed release run fixed in the workflow file needs the TAG moved to the fixed commit** (`git tag -f` + `git push --force Source <tag>`). The workflow that runs is the one at the tag's commit; pushing the branch alone does not re-trigger the tag-push workflow.
- **A job that WRITES INTO a submodule needs its own `actions/checkout` with `submodules: recursive`** - a network-only job has no checkout, so a step writing `plugins/aphrodite/SHA256SUMS.txt` fails "No such file or directory" at the first redirect (v0.0.3 Build.yml Finalize). When the release already exists, do NOT move the tag to re-run; complete the missing artifact manually. Anatomy: `references/worked-detail.md`.
- **Cross-repo child pushes from a workflow need a PAT, AND the persisted GITHUB_TOKEN extraheader must be removed GLOBALLY** (`git config --global --unset-all http.https://github.com/.extraheader`) or the push silently 403s as `github-actions[bot]` even with a valid PAT in the URL. Working recipe: `references/worked-detail.md`.
- **A gitlink-bump job on the tag push RACES the release workflow's Finalize child push - it must WAIT for the child tip** whose in-tree `SHA256SUMS.txt` pins the release version, not fetch once (loop: `git show origin/Current:SHA256SUMS.txt | grep -q "^BINARY_VERSION: ${V}$"`, bounded e.g. 40 x 15s; a 5x10s retry is NOT enough). Full loop: `references/worked-detail.md`.
- **A workflow job that PUSHES into a submodule's own repo cannot use `GITHUB_TOKEN`** - it is scoped to the parent repo and 403s on any other repo. Needs a fine-grained PAT scoped to the CHILD repo only (Contents: write), stored as an ENVIRONMENT secret. Recipe + guards: `references/worked-detail.md`.
- **Fine-grained PATs cannot be minted from the CLI with an OAuth-authenticated `gh` (`gho_` token): `POST /user/pat` returns 404 unless the token carries `pat:write` - which only a CLASSIC PAT can hold, and classic PATs have no creation API at all.** One-time web-UI step (Settings → Developer settings → Fine-grained tokens; repo access = ONLY the child repo, Contents: write); the CLI handles storage/wiring after. Probe the API once, then hand the web step to the user; do not loop on it.
- **`Bump-Plugin-Gitlink`-style jobs gated on `if: startsWith(github.ref, 'refs/tags/Aphrodite/')` run ONLY on tag-push refs** - a manual `gh workflow run ... --ref Development` dispatch silently SKIPS them (conclusion: skipped), leaving the parent gitlink stale after a manual child push. Bump the gitlink manually when dispatching from a branch ref.
- **Submodule remotes in PlayForm repos are named `Source`, not `origin`.** `git push origin` from inside a submodule fails with an SSH access-rights error (a missing-remote-name problem, not auth). Push `git push Source Current` (the .gitmodules `remote = Source` binding).
- **Scripts running the repo's own freshly-built binary must resolve the explicit override (`APHRODITE_BIN`) BEFORE the PATH lookup** - a stale cargo-installed copy on PATH shadows the fresh build and silently regenerates with the OLD behavior. Resolution: override first, PATH fallback second, loud error + exit 1 when neither exists.
- **A generator that CREATES a new fixture file must extend the CI fixture validator's known-name branches (name prefix → required keys) in the SAME change** - the validator fails unknown names ("unexpected fixture name"), so the first CI run goes red (generated `plugin_defaults.json` failed exactly this way). Recipe: `references/worked-detail.md`.
- **After a release push, "all good?" includes the branch's CI runs - check `gh run list` and triage failures into yours vs pre-existing.** New generated fixtures and workflow edits are yours to fix now; environment gaps may be pre-existing - verify with the workflow's run history before blaming or fixing.
- **`cargo test` on this fleet's codegen'd doc examples fails as doctests** (`crate::` paths, bare `Fn`/`Option`) - scope the publish Test gate to `cargo test --release --tests --bins`; the unit harnesses still run.
- **The fmt gate is the PINNED NIGHTLY rustfmt, never the toolchain's stable one** - stable rustfmt SILENTLY IGNORES a rustfmt.toml with `unstable_features = true` plus nightly-only options, so a green stable check proves nothing. Gate-equal: `cargo +nightly-<pin> fmt --check` scoped to the FULL workspace, NEVER plain `cargo fmt --check`. Details + VSCode override: `references/worked-detail.md`.
- **Mirror the CI job's exact scope when verifying format/lint gates** - a root-wide check with a NEWER tool version flags out-of-scope files and looks like a regression (latest ruff formats `.md` fenced blocks, so `ruff format --check .` lights up docs + vendored wiki files the CI gate never touches). Use `git ls-files '*.py'` (never sees submodule content) and the CI's pinned tool version; a scoped green + a root-wide red is a scope mismatch. Details: `references/worked-detail.md`.

## Related skills

- `aphrodite-release-workflow` - the release gates (version ledger, Gate R7, artifact contract) this skill's ceremony executes
- `automatic-release-pipeline` - scheduled release automation
- `github-readme-generation` - README content verification when docs change

## Claim-to-test table

| Claim | Test | Pass | Fail |
|---|---|---|---|
| The lockstep set moved together | `grep -c 'version = "X.Y.Z"' <file>` per manifest; `git show HEAD:<file>` | expected count (2 with a path-dep pin, 1 otherwise) and new version in HEAD | wrong count or old version in HEAD |
| The dep pin equals the crates.io version | `curl -s -H "User-Agent: <contact>" https://crates.io/api/v1/crates/<name>`; the pin vs `max_version` | pin == `max_version` | pin mismatch (`cargo publish` fails) |
| The built binary reports the new version | `./target/release/aphrodite --version` AND `aphrodite --version` | both print the new version | either prints the old number |
| The installed plugin tree is byte-consistent | `aphrodite --version`, dylib version report, `plugin.yaml` `version`, `BINARY_VERSION` | all four agree | handshake reports binary X vs installed pin Y |
| The release run works end-to-end | live run; newest window header | exit 0 and header reads `from <previous-tag> to last commit` | non-zero exit or old header |
| The tag exists in the exact required form | `git ls-remote --tags` | `Aphrodite/v<semver>` (annotated) present | tag missing or wrong form (release jobs never run) |
| The crates.io publish happened, not skipped | `curl -s -H "User-Agent: <contact>" https://crates.io/api/v1/crates/<name>` | `max_version` equals the new version | stale `max_version` (silent-skip) |
| The release assets are complete | `gh release view <tag> --json assets --jq '.assets[].name'` | per-target binary + dylib + checksum set (12 assets) | fewer assets |
| No pending child commit before the tag | `git submodule status` | no `+` | `+` present |
| The git-state claim is backed by HEAD | `git show HEAD:<file>` | carries the new version | old version (claim false) |
| Fork content did not move since the last publish | `git log <last-published-commit>..HEAD` inside the fork/submodule | no content delta | content moved (next publish skips or ships stale) |
| The publish gate still applies | no `cargo publish`, no `git push Source <tag>`, no push, no release absent explicit instruction | only local commits by the sync | any remote-facing action |
| The workflow at the tag commit is the one that runs | `git tag -f` + `git push --force Source <tag>` | tag-push workflow re-runs at the fixed commit | branch-only push (workflow does not re-run) |
| The publish-path assumption matches the target repo | inspect `.github/workflows/` AND the crate's crates.io Trusted Publishing settings | token or OIDC matches the repo's actual gate | 403 (`can only be published using Trusted Publishing`) |