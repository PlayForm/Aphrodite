# Skill Manifest

The canonical inventory of the repo skill library: every skill under
`.hermes/skills/<category>/<skill>/SKILL.md`, with its version, status,
mutation level, category, and (for archived skills) successor. The library is
organized by category (aphrodite, git, github, hermes, release, docs,
engineering, curation, archived) and ships on the public Development branch.
This file is anonymized: no personal identifiers or machine-local paths -
neutral stand-ins (`~/.hermes/tmp/`, `<workspace>`) are used throughout.

Fields are read from each SKILL.md frontmatter (`name`, `version`, `category`,
`status`, `mutation_level`, `successor`). `mutation_level` defaults to `local`
when a skill does not declare it; observed values: `read-only`, `local`,
`publish`, `mutate`, `mutates`, `orchestration`. `successor` is set only for
archived skills. 58 skills total: 57 active, 1 archived.

## aphrodite (18)

| Skill                             | Version | Status | Mutation level | Category  | Description                                                                                                                               |
| --------------------------------- | ------- | ------ | -------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| agent-session-benchmarking        | 1.1.0   | active | local          | aphrodite | Use when benchmarking agents/plugins with real LLM sessions.                                                                              |
| aphrodite-auto-expand-testing     | 3.1.0   | active | read-only      | aphrodite | Use when debugging why CCR markers appear raw in the LLM view.                                                                            |
| aphrodite-benchmarking            | 2.1.0   | active | read-only      | aphrodite | Use when benchmarking the aphrodite compression proxy.                                                                                    |
| aphrodite-boundaries              | 1.1.0   | active | read-only      | aphrodite | Use when any Aphrodite work touches state, git, context, secrets, or external side effects.                                               |
| aphrodite-cargo-upgrade           | 1.3.0   | active | local          | aphrodite | Use when upgrading cargo deps in PlayForm/Aphrodite.                                                                                      |
| aphrodite-ccr-protocol            | 1.1.0   | active | read-only      | aphrodite | Use when producing, parsing, resolving, or versioning CCR markers.                                                                        |
| aphrodite-compression-safety      | 1.1.0   | active | read-only      | aphrodite | Use when deciding what may be compressed, how messages are sliced, or how recursion and re-compression are prevented in the CCR pipeline. |
| aphrodite-context-engine-contract | 1.1.0   | active | read-only      | aphrodite | Use when configuring, debugging, or verifying the Aphrodite context engine.                                                               |
| aphrodite-development             | 2.1.0   | active | local          | aphrodite | Use when developing the aphrodite plugin.                                                                                                 |
| aphrodite-engine-observability    | 1.1.0   | active | read-only      | aphrodite | Use when probing, diagnosing, or verifying Aphrodite engine health.                                                                       |
| aphrodite-hook-contracts          | 1.1.0   | active | read-only      | aphrodite | Use when implementing or debugging Aphrodite Hermes hooks.                                                                                |
| aphrodite-hook-reference          | 2.1.0   | active | read-only      | aphrodite | Use when deciding which aphrodite hook/context contract to consult.                                                                       |
| aphrodite-operations              | 2.1.0   | active | local          | aphrodite | Use when operating inside an aphrodite-compressed session or the Aphrodite repo.                                                          |
| aphrodite-orientation             | 1.1.0   | active | read-only      | aphrodite | Use when starting any Aphrodite workflow or debugging session.                                                                            |
| aphrodite-release-flow            | 2.2.0   | active | publish        | aphrodite | Use when releasing or hotfixing Aphrodite (parent + plugin submodule), or verifying the live CCR engine after a bump.                     |
| aphrodite-release-workflow        | 2.2.0   | active | publish        | aphrodite | Use when releasing Aphrodite.                                                                                                             |
| aphrodite-testing-discipline      | 2.1.0   | active | local          | aphrodite | Use when probing, testing, or verifying the Aphrodite plugin or dylib.                                                                    |
| aphrodite-tool-testing            | 2.1.0   | active | read-only      | aphrodite | Use when teaching or verifying aphrodite CCR tool usage.                                                                                  |

## git (7)

| Skill                      | Version | Status | Mutation level | Category | Description                                                                                                                                                                            |
| -------------------------- | ------- | ------ | -------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| branch-flow-protocol       | 1.1.0   | active | local          | git      | Use when designing branch release flows in the Aphrodite monorepo.                                                                                                                     |
| dual-state-merge-review    | 1.1.0   | active | local          | git      | Use when reviewing branch merges: staged vs pending review.                                                                                                                            |
| fork-pr-branch-sync        | 1.1.0   | active | local          | git      | Use when syncing fork PR branches to base for clean merges.                                                                                                                            |
| git-feat-dev-workflow      | 1.2.0   | active | local          | git      | Use when running the feat-dev reverse-PR branch workflow to contribute upstream (e.g. hermes-agent) from the Aphrodite dev environment.                                                |
| git-operations             | 1.1.0   | active | local          | git      | Use when maintaining git repositories in the PlayForm/Aphrodite monorepo.                                                                                                              |
| save-oneshot-tooling       | 0.1.0   | active | local          | git      | Use when debugging or modifying the Save gcommit oneshot commit-message tooling.                                                                                                       |
| submodule-fleet-management | 1.1.0   | active | local          | git      | Use when resetting, syncing, or keeping Aphrodite's submodule fleet (plugins/aphrodite with remote 'Source', vendor/headroom, vendor/rtk) on-branch with auto-synced gitlinks (hooks). |

## github (8)

| Skill                      | Version | Status | Mutation level | Category | Description                                                                                                          |
| -------------------------- | ------- | ------ | -------------- | -------- | -------------------------------------------------------------------------------------------------------------------- |
| github-actions-maintenance | 1.1.0   | active | local          | github   | Use when pinning GitHub Actions versions to commit SHAs or syncing generated workflow files (Build.yml/Publish.yml). |
| github-auth                | 1.3.0   | active | local          | github   | Use when GitHub authentication is missing or broken.                                                                 |
| github-batch-pr-merge      | 1.2.0   | active | local          | github   | Use when bulk-merging or closing open PRs across repos/orgs.                                                         |
| github-code-review         | 1.3.0   | active | local          | github   | Use when reviewing code or pull requests.                                                                            |
| github-issue-responses     | 1.2.0   | active | local          | github   | Use when drafting public responses to GitHub issue reports.                                                          |
| github-issues              | 1.3.0   | active | local          | github   | Use when creating, triaging, or managing GitHub issues.                                                              |
| github-pr-workflow         | 1.3.0   | active | local          | github   | Use when driving the GitHub PR lifecycle.                                                                            |
| github-repo-management     | 1.3.0   | active | local          | github   | Use when cloning, creating, or forking GitHub repos.                                                                 |

## hermes (7)

| Skill                         | Version | Status | Mutation level | Category | Description                                                                                                                              |
| ----------------------------- | ------- | ------ | -------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| debugging-hermes-tui-commands | 1.2.0   | active | local          | hermes   | Use when debugging or adding Hermes TUI slash commands, including commands that surface Aphrodite plugin state.                          |
| hermes-agent                  | 3.3.0   | active | local          | hermes   | Use when operating, configuring, theming, extending, or orchestrating Hermes Agent inside the Aphrodite monorepo.                        |
| hermes-config-maintenance     | 1.2.0   | active | local          | hermes   | Use when cleaning dead toolset/plugin refs from Hermes config, bypassing the config write guard, or wiring Aphrodite env/config keys.    |
| hermes-cron-scheduling        | 1.1.0   | active | local          | hermes   | Use when scheduling durable Hermes cron jobs and watchdogs for the Aphrodite runtime.                                                    |
| hermes-diagnostics            | 1.1.0   | active | local          | hermes   | Use when diagnosing Hermes TUI, memory, gateway, or session-compression misbehavior in an Aphrodite development session.                 |
| hermes-session-recovery       | 1.1.0   | active | local          | hermes   | Use when a Hermes session running the Aphrodite plugin was truncated, died mid-work, or you need to reconstruct what a past session did. |
| hermes-shell-hooks            | 2.1.0   | active | local          | hermes   | Use when adding, testing, or debugging Aphrodite shell hooks.                                                                            |

## release (2)

| Skill                      | Version | Status | Mutation level | Category | Description                                              |
| -------------------------- | ------- | ------ | -------------- | -------- | -------------------------------------------------------- |
| automatic-release-pipeline | 1.4.0   | active | local          | release  | Use when automating Aphrodite releases on a schedule.    |
| playform-cargo-maintenance | 1.2.0   | active | local          | release  | Use when bumping or publishing Aphrodite Cargo versions. |

## docs (2)

| Skill                     | Version | Status | Mutation level | Category | Description                                                            |
| ------------------------- | ------- | ------ | -------------- | -------- | ---------------------------------------------------------------------- |
| github-readme-generation  | 1.2.0   | active | local          | docs     | Use when generating, editing, or restyling PlayForm/Aphrodite READMEs. |
| markdown-readme-audit-fix | 1.2.0   | active | local          | docs     | Use when fixing README markdown formatting and links.                  |

## engineering (11)

| Skill                         | Version | Status | Mutation level | Category    | Description                                                                                                                      |
| ----------------------------- | ------- | ------ | -------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------- |
| code-quality-improvement      | 1.2.0   | active | local          | engineering | Use when improving code quality and docs across the Aphrodite monorepo (Rust crates, plugin Python, .hermes docs).               |
| macos-binary-deployment       | 1.1.0   | active | mutates        | engineering | Use when deploying the freshly built Aphrodite binary and libaphrodite_hermes.dylib into ~/.hermes/aphrodite/binaries on macOS.  |
| parallel-delegation-execution | 1.1.0   | active | orchestration  | engineering | Use when running large repo tasks as parallel delegate waves on the Aphrodite monorepo (PlayForm/Aphrodite, Development branch). |
| plan                          | 1.1.0   | active | local          | engineering | Use when asked to plan, not execute (absorbed writing-plans).                                                                    |
| repo-formatting-gates         | 1.1.0   | active | local          | engineering | Use when running a repo's formatting gates with CI parity.                                                                       |
| rust-comment-quality          | 1.1.0   | active | local          | engineering | Use when replacing stale //! TODO markers with structured module documentation in the Aphrodite Rust crates.                     |
| rust-module-atomization       | 1.1.0   | active | local          | engineering | Use when atomizing a Rust file into a per-item module tree in the Aphrodite crates, keeping the suite green at every phase.      |
| shell-script-conventions      | 1.1.0   | active | local          | engineering | Use when editing shell scripts in the Aphrodite workspace or the user's private rc files.                                        |
| simplify-code                 | 1.2.0   | active | local          | engineering | Use when simplifying recent Aphrodite code.                                                                                      |
| surgical-file-editing         | 1.1.0   | active | local          | engineering | Use when patch mangles whitespace on multi-line edits in the Aphrodite monorepo (crates, plugin Python, .hermes docs).           |
| test-driven-development       | 1.3.0   | active | local          | engineering | Use when coding test-first in the Aphrodite monorepo.                                                                            |

## curation (2)

| Skill                        | Version | Status | Mutation level | Category | Description                                                                        |
| ---------------------------- | ------- | ------ | -------------- | -------- | ---------------------------------------------------------------------------------- |
| hermes-agent-skill-authoring | 1.1.0   | active | mutate         | curation | Use when authoring or editing repo skills in the Aphrodite `.hermes/skills/` tree. |
| skill-library-refactoring    | 1.1.0   | active | local          | curation | Use when refactoring/deduplicating the Aphrodite repo's skill library.             |

## archived (1)

| Skill                 | Version | Status   | Mutation level | Category | Successor              | Description                                                                                                |
| --------------------- | ------- | -------- | -------------- | -------- | ---------------------- | ---------------------------------------------------------------------------------------------------------- |
| benchmark-run-summary | 1.1.0   | archived | local          | archived | aphrodite-benchmarking | Use when summarizing an adversarial RED/BLUE/PURPLE/WHITE/BLACK benchmark run of the Aphrodite CCR engine. |

## Maintenance rules

- Update this manifest in the same change that changes a skill's version,
  status, mutation level, category, or successor.
- An archived skill never reappears as live; re-activation requires a new row
  with `active` status and a fresh version (record the change in
  CONTRADICTION-REGISTER.md).
- Verify the manifest against the library with `find .hermes/skills -name
SKILL.md | sort` plus a frontmatter scan - every SKILL.md `name`, `version`,
  `status`, and `mutation_level` must match a row here.
