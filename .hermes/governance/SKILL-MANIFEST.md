# Aphrodite Skill Manifest

The canonical contract for the Aphrodite skill system target end-state. Every
agent rewriting skills during the 2026-09-18 refactor honors this table:
canonical name, status, owner, scope, source of truth, dependencies, and
successor/archive location. A skill is not "live" until its row here says
`active` and its SKILL.md carries the matching frontmatter
(`status: active`, declared `scope`, `mutation_level`).

**Legend:** P = PlayForm/Aphrodite (parent), S = PlayForm/Aphrodite-Hermes
(plugin submodule at `plugins/aphrodite`, remote `Source`). Runtime modes:
`source` (repo workspace) / `installed` (user plugin). Mutation levels:
`read-only` | `local` | `git` | `remote` | `publish`.

## Live skills (target end-state)

| Canonical name                      | Status | Owner (area)               | Scope                       | Source of truth                                            | Dependencies                                                                                    | Successor/archive location                                                | Mutation level |
| ----------------------------------- | ------ | -------------------------- | --------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------- |
| `aphrodite-boundaries`              | active | Core protocol              | P + S · Development         | `.hermes/governance/BOUNDARIES.md`, VERIFICATION-MATRIX.md | aphrodite-orientation                                                                           | -                                                                         | read-only      |
| `aphrodite-orientation`             | active | Preflight gate             | P + S · Development         | VERIFICATION-MATRIX.md (git orientation row)               | aphrodite-boundaries                                                                            | -                                                                         | read-only      |
| `aphrodite-hook-reference`          | active | Hook contracts             | S · Development             | Hermes source `invoke_hook` sites                          | hook-contracts, context-engine-contract, ccr-protocol, compression-safety, engine-observability | short dispatcher; details owned by the five contract skills               | read-only      |
| `aphrodite-hook-contracts`          | active | Hook contracts             | S · Development             | Hermes valid-hook registry + invocation source             | aphrodite-boundaries                                                                            | -                                                                         | read-only      |
| `aphrodite-context-engine-contract` | active | Context engine             | S · Development             | Hermes `ContextEngine` base + registration code            | aphrodite-boundaries                                                                            | -                                                                         | read-only      |
| `aphrodite-ccr-protocol`            | active | CCR protocol               | S · Development             | Rust marker producer/parser, resolver                      | compression-safety                                                                              | C-003 canonical owner                                                     | read-only      |
| `aphrodite-compression-safety`      | active | CCR safety                 | S · Development             | Transform pipeline skip list, recursion guards             | ccr-protocol                                                                                    | -                                                                         | read-only      |
| `aphrodite-engine-observability`    | active | Engine health              | S · Development             | Proxy health endpoints, log locations                      | compression-safety                                                                              | -                                                                         | read-only      |
| `aphrodite-development`             | active | Dev setup                  | P + S · Development         | Workspace manifests, runtime home                          | aphrodite-orientation, testing-discipline                                                       | supersedes `aphrodite-development-lessons`                                | local          |
| `aphrodite-operations`              | active | Compressed-session ops     | P + S · Development         | Runtime home layout, dylib version probes                  | boundaries, orientation, engine-observability                                                   | -                                                                         | local          |
| `aphrodite-testing-discipline`      | active | Testing rules              | P + S · Development         | Maintain/tests, gate commands                              | boundaries, orientation                                                                         | -                                                                         | local          |
| `aphrodite-tool-testing`            | active | CCR tools                  | S · Development             | 13-tool API, retrieve-first doctrine                       | aphrodite-boundaries, aphrodite-orientation                                                     | -                                                                         | read-only      |
| `aphrodite-benchmarking`            | active | Benchmarks                 | P + S · Development         | Benchmark fixture suite, experiment record                 | aphrodite-boundaries, aphrodite-orientation                                                     | -                                                                         | read-only      |
| `aphrodite-auto-expand-testing`     | active | Auto-expand (inert config) | S · Development             | Rust config parse + consumer scan                          | aphrodite-boundaries, aphrodite-orientation                                                     | C-001 canonical owner                                                     | read-only      |
| `aphrodite-cargo-upgrade`           | active | Dep upgrades               | P + S · Development         | Cargo.toml/lockfile, breakpoint records                    | boundaries, orientation                                                                         | absorbs `aphrodite-upgrade-breakpoints`                                   | git            |
| `aphrodite-release-flow`            | active | Release ceremony           | P + S · Development/Current | `RELEASE-METHODOLOGY.md`, workflow files at tag commit     | orientation, boundaries, release-workflow                                                       | successor of `aphrodite-branch-release-flow`; C-002/C-004 canonical owner | publish        |
| `aphrodite-release-workflow`        | active | Release gates              | P + S · Development/Current | Workflow definitions, version ledger                       | release-flow, boundaries                                                                        | -                                                                         | publish        |

## Archived skills (do not execute)

| Canonical name                  | Status                   | Owner (area)              | Scope                        | Source of truth                                                       | Dependencies | Successor/archive location              | Mutation level |
| ------------------------------- | ------------------------ | ------------------------- | ---------------------------- | --------------------------------------------------------------------- | ------------ | --------------------------------------- | -------------- |
| `aphrodite-branch-release-flow` | archived, do_not_execute | Release (historical)      | P + S · Current (historical) | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-branch-release-flow/` | none         | successor `aphrodite-release-flow`      | read-only      |
| `aphrodite-v0.8.6-patterns`     | archived, do_not_execute | Historical                | P (historical)               | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-v0.8.6-patterns/`     | none         | historical only; no successor           | read-only      |
| `aphrodite-upgrade-breakpoints` | archived, do_not_execute | Dep upgrades (historical) | P + S (historical)           | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-upgrade-breakpoints/` | none         | absorbed into `aphrodite-cargo-upgrade` | read-only      |

## Refactor completion note

The 2026-09-20 rewrite wave is complete: every row above is `active` and its
SKILL.md carries matching frontmatter (`status: active`, declared `scope`,
`mutation_level`). The `aphrodite-hook-reference` row is the short dispatcher;
the five contract skills own the details. Any future status change must update
this manifest in the same change.

## Manifest maintenance rules

- This manifest is the contract; update it in the same change that changes a
  skill's status, name, or owner. Never describe a skill as live in prose
  without updating its row.
- An archived skill never reappears as live. Re-activation requires a new
  row with `active` status, a fresh version, and a note in
  CONTRADICTION-REGISTER.md.
- No active skill lists an archived skill under "Related" unless the link
  label says **Historical only-do not execute**.
- Every row's `Source of truth` must name a file or executable probe, never
  another skill's prose.
- Verify the manifest against the skills directory with:
  `ls .hermes/skills/` and a frontmatter scan (every SKILL.md's `name` must
  match a canonical name here).
