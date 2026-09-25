# Release-automation research notes (condensed)

## Tool landscape (verified live vs npm registry, 2026-08-19)

- semantic-release `25.0.9` (`@semantic-release/npm 13.1.5`, `@semantic-release/github 12.0.9`)
- release-please `17.11.1`
- @changesets/cli `3.0.1`, @changesets/get-release-plan `5.0.0`
- standard-version `9.5.0` - DEPRECATED
- auto `11.3.6`
- git-cliff `2.13.1` (bin `lib/cli/cli.js`, repo `orhun/git-cliff`)
- conventional-changelog-conventionalcommits `10.4.0`

## git-cliff architecture (the "crate + npm wrapper" answer)

- `git-cliff-core` (Rust crate): parses conventional commits, groups, renders Tera.
- npm `git-cliff`: optionalDependencies per `os`/`cpu` pull the prebuilt Rust binary;
  `bin.js` resolves + execs it. Same model to brand as a `@playform/release-notes`
  wrapper.
- To brand: fork `orhun/git-cliff` OR depend on `git-cliff-core`; publish per-platform
  binaries to GitHub Releases; ship an npm wrapper with platform optionalDeps.

## Gap-analysis pattern (do this on every repo)

1. Dependabot opens+auto-merges? 2. Conventional commits in use? 3. What fires publish?
2. The sole manual gap = version bump + CHANGELOG + GitHub Release.

## Aphrodite gap analysis (2026-09)

- PlayForm/Aphrodite already had: Dependabot auto-merge, conventional commits,
  Build.yml (tag-push GitHub Release, 4-target matrix) + Publish.yml (cargo
  publish chain; the headroom fork dispatch-gated). Only the release-creation
  link was manual.
- Tag scheme `Aphrodite/vX.Y.Z` (annotated). The scheduled Release Manager
  bumps version + CHANGELOG and creates the tag + release body; the existing
  tag-triggered workflows attach assets and publish crates.
- `package.json` carries the binary version; `BINARY_VERSION` + `plugin.yaml`
  live in the `plugins/aphrodite` submodule (separate repo, bumped in the same
  ceremony).
- Publish provenance: Publish.yml uses `environment: Release` +
  `id-token: write`; the headroom fork is crates.io trusted-publishing.
