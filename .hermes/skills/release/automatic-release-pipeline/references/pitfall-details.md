# Pitfall detail (worked instances moved out of SKILL.md)

Each section names the thing, the respect, and the test. These are the
worked instances of the one-line rules in SKILL.md's Pitfalls section.

## Fork-crate publish pair

**A release that path-depends on a vendored fork crate silently skips the
fork publish when the fork's own version is unchanged.** CI's version-check
reads the fork crate's Cargo.toml version, finds it already in the crates.io
index, and skips the publish - while the parent crate (path+version dep,
path stripped on publish) publishes against the OLD published content.
Nothing fails, so the gap is invisible: the fork's latest tree exists only
as a local path dependency. Bump the fork's crate version AND the parent's
pin in the same cycle; at release time, check that the fork's Cargo.toml
version is not still the previously published number.

**Publish ordering when a fork crate's version CHANGES: dispatch the fork
publish BEFORE the tag push.** The fork's publish step is gated
`workflow_dispatch` (tag pushes cannot reach it), and the parent's
tag-triggered publish resolves the dependency from the registry at publish
time - the new fork version must already be live or the parent publishes
against the old content (or fails). After the dispatch succeeds, the tag
push re-runs the fork step and fails red on re-publish - known cosmetic
behavior, not a regression; verify with timestamps (dispatch publish time <
tag push time) and the parent crate's dependency list.

## npm registry propagation probes

**npm registry propagation lags the publish run** - `npm view` can keep
showing the OLD version for minutes after the workflow logged
`+ @scope/pkg@newver`. The run log (with its sigstore provenance logIndex
line) and the DIRECT registry endpoint are the ground truth:
`curl -s https://registry.npmjs.org/@scope/pkg/<version>` → HTTP 200, and
the package root endpoint's `dist-tags.latest`. A stale `npm view`/`npm
install` check minutes later is a cache/propagation artifact, not a failed
publish - re-check the direct endpoint before concluding, and never "fix" a
publish that already succeeded.

## Version-realign hiding places

**When re-aligning a version "everywhere", derive the surface from the
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

## Script-removal procedure

**Remove a local release script only after proving no workflow or skill
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

## Dependabot red-PR merge

**Dependabot auto-merge can merge fully-red PRs when checks aren't required
- and a version-only bump can be invalid across a major version** (ureq 2.x
`tls` feature does not exist in 3.x, which exposes only `_rustls`/`_test`).
A red `cargo test` on the PR branch proves the manifest is broken; revert
the requirement bump (keep the old major) and gate the auto-merge on
required checks before re-approving bumps.

## Tag shape and tag-push detail

Core rule: never run `git tag <name>` bare in automation - always
`git tag -a <name> -m "<msg>"`, because when the repo sets
`tag.gpgsign` / `tag.forcesignannotated` (or a `git tag` alias), a plain tag
becomes annotated+signed and opens `$EDITOR` for a message, killing the
script mid-flow or leaving a half-created tag. Then verify
`git rev-parse <name>^{}` points at the intended commit.

Remote evidence is the raw output of `git ls-remote --tags <remote>` - the
remote tag set, not local tags, not manifest agreement.

Tag-shape matching rules: UNKNOWN - `references/tag-and-test-details.md`
was linked from SKILL.md but never existed in this skill directory, and no
repo text defines the matching rules.

Interrupted-session rules: UNKNOWN - same missing source; no repo text
defines recovery from an interrupted tag/session flow.

Mis-tag handling: UNKNOWN - same missing source; no repo text defines how
to correct a tag created at the wrong commit.

## Stale-binary reruns

**Real-binary tests fail after the bump = the locally installed binary is
the old version.** Rebuild BOTH binaries from the workspace root
(`cargo install --path crates/<crate> --force`), then re-run - the failures
are the expected transient, not a regression. (Source:
`references/dual-repo-release-ceremony.md`.)

## Doctest gate scoping

**A `cargo test` gate on a codegen'd crate fails on DOCTESTS, not unit
tests** - generated `///` examples use `crate::` paths (invalid in doctests,
which are external crates) and bare identifiers (`Fn`, `Option`,
`DashMap`) that collide with std items. When the package has no test targets
(`autotests=false`, no `[[test]]`), scope the gate to skip doctests:
`cargo test --release --tests --bins` - it still compiles the shipped bins
in test mode and runs 0 tests, exit 0. (Source:
`references/release-dry-run.md`.)

## HERMES_AGENT_SRC CI env

UNKNOWN - `references/tag-and-test-details.md` (the file that described
HERMES_AGENT_SRC CI env usage) never existed in this skill directory, and
no repo text defines the variable or its use.