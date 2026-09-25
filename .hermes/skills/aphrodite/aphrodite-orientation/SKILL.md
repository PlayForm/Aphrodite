---
name: aphrodite-orientation
description: "Use when starting any Aphrodite workflow or debugging session. Mandatory preflight gate: verify repository, branch, submodule, and runtime state first."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-orientation
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, orientation, preflight, git, submodule, safety]
        related_skills:
            [
                aphrodite-boundaries,
                aphrodite-release-flow,
                aphrodite-release-workflow,
                aphrodite-testing-discipline,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
    runtime_modes:
        - source
        - installed
owns:
    - The orientation gate (preflight) and its 5 read-only commands
    - Interpretation rules for orientation output (root, branch, status, submodule, remote)
    - The Orient phase of the unified lifecycle
    - Submodule state diagnosis (detached / missing / modified / phantom gitlink)
    - Auto-committer awareness (capture HEAD and remote tip before beginning)
depends_on:
    - aphrodite-boundaries (terminology, stop/recovery semantics, git repair taxonomy)
supersedes: []
verification:
    source_of_truth:
        - .hermes/governance/VERIFICATION-MATRIX.md (git orientation state row)
        - .hermes/AGENTS.md (repo facts)
mutation_level: read-only
---

# Aphrodite Orientation

Mandatory preflight gate. **No Aphrodite workflow begins with a mutation
command.** Before any live procedure-release, hotfix, upgrade, debugging,
benchmarking, or testing-run the orientation gate and interpret the output
against the skill's declared scope. This skill owns the Orient phase of the
unified lifecycle; it performs read-only inspection only
(`mutation_level: read-only`).

## Orientation gate (5 read-only commands)

Run all five, in order, from the directory you intend to work in:

```sh
git rev-parse --show-toplevel
git branch --show-current
git status --short
git submodule status --recursive
git remote -v
```

Capture the output verbatim. Do not proceed until every line has been
interpreted (below).

## Interpretation rules

| Observation                                                                               | Interpretation                            | Action                                                    |
| ----------------------------------------------------------------------------------------- | ----------------------------------------- | --------------------------------------------------------- |
| Repository root is not the expected repo (parent or plugin submodule)                     | Wrong worktree                            | **Stop**; move to the intended repository                 |
| Branch is not permitted by the skill's declared scope                                     | Wrong branch                              | **Stop**; do not switch branches to "fix" this            |
| `git status --short` shows unrelated changes                                              | Dirty/unexpected state                    | **Stop**; do not clean, stash, or reset them              |
| Submodule detached, missing, modified unexpectedly, or a nested mode-160000 entry appears | Broken submodule state                    | **Stop**; run the submodule diagnosis below               |
| Auto-committer may be active                                                              | `git status` alone is not stable evidence | Capture current `HEAD` and remote tip first, then proceed |

## Submodule diagnosis (dedicated procedure)

When the gate shows a submodule problem, do not guess. Classify the symptom:

1. **Detached HEAD** (`-` prefix, no `+`): the gitlink points at a commit that
   is not a branch tip. Record the detached SHA, then check whether the
   submodule's branch field (`.gitmodules`) names the expected branch.
2. **Modified** (`+` prefix): working tree differs from the checked-out
   commit. Inspect `git -C <submodule> status --short`; distinguish intended
   work from accidental drift.
3. **Missing** (no entry or `-` with absent directory): the submodule
   directory is absent. `git submodule update --init` is the permitted repair
   only after the parent state is verified clean.
4. **Phantom gitlink** (mode-160000 entry pointing at itself or an ancestor):
   remove the indexed mode-160000 entry, then validate recursively after
   later commits. Never ignore it because the parent status looks clean.

For any submodule issue, collect the evidence from the gate, then choose a
permitted recovery from `aphrodite-boundaries` (Git repair taxonomy) or stop
and escalate.

## Auto-committer awareness

`git status` is not stable evidence when an external auto-committer sweeps
working-tree changes. Before any workflow that depends on a clean or specific
state:

```sh
git rev-parse HEAD
git ls-remote origin <branch>
```

Record both. Re-check `HEAD` at every boundary where state matters (B4 audit,
pre-tag, post-sync) and compare against the captured baseline.

## Runtime & config preflight (installed mode)

When the workflow touches the installed runtime (`~/.hermes/aphrodite/`,
`~/.hermes/plugins/aphrodite/`), extend the gate with read-only checks:

- **Remotes and submodules:** the repository's remote is `Source`
  (`ssh git@github.com/PlayForm/Aphrodite.git`); three submodules:
  `plugins/aphrodite` (remote `Source`, branch `Development`),
  `vendor/headroom` (remote `Source`, branch `Current`), and
  `vendor/rtk` (remote `Source`, branch `Current`). `git submodule status
--recursive` must show none detached or dirty beyond declared scope.
- **Upstream config is env-driven:** the `aphrodite.toml` template
  substitutes ONLY the ports (cache `9797`, token `9798`); `api_url` and
  `model` come from `APHRODITE_API_URL` / `APHRODITE_MODEL`, and the proxy's
  API key from `APHRODITE_API_KEY` (or `[defaults] api_key` in the toml).
  `aphrodite setup --api-key/--api-url/--model` are parsed but the toml
  template has no placeholders for them - never claim those flags write into
  the toml.
- **Verify the key is actually exported:** the proxy fails loudly with
  `no API key configured - set APHRODITE_API_KEY env var` when the var is
  absent OR commented out in the private environment file. Check
  `env | grep APHRODITE_API_KEY` (and that the line is not commented out)
  before any proxy-dependent workflow; never assume presence from a file's
  contents.

These are read-only checks; fixing a missing key or config is a mutation
that belongs to the owning workflow, not to Orient.

## The unified lifecycle (Orient phase owned here)

All Aphrodite work fits one lifecycle. Each phase has an entry condition,
allowed changes, exit evidence, and a hard stop. This skill owns the **Orient**
row; operational skills declare which phases they drive.

| Phase         | Entry condition                             | Allowed changes                                | Exit evidence                                          | Hard stop                                       |
| ------------- | ------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------- |
| **Orient**    | Repository may be unknown                   | Read-only inspection                           | Correct repository, branch, submodule state identified | Dirty/unexpected repository state               |
| **Prepare**   | Scope and ownership known                   | Local code/docs/config edits                   | Targeted static checks pass                            | Unverified assumptions about live behavior      |
| **Validate**  | Changes are locally coherent                | Builds, tests, isolated runtime probes         | Evidence attached to the change                        | Test failures or stale-process uncertainty      |
| **Integrate** | Validated change and clean ownership        | Controlled commits, cherry-picks, bounded sync | Protected paths unchanged or deliberately handled      | Unexpected conflict, extra staged files         |
| **Release**   | Current line contains exact release content | Tag, artifact build, deliberate publish        | Tag, artifacts, consumer verification                  | Existing version, missing assets, wrong trigger |
| **Observe**   | Release or runtime change complete          | Health and round-trip checks                   | Engine, retrieve, search, preview behavior verified    | Diagnostics disagree with release claim         |
| **Recover**   | A defined failure condition occurred        | Only documented repair operations              | Root cause and clean state verified                    | Destructive shortcut or inferred recovery       |

**Hard stop for Orient:** dirty/unexpected repository state. When the gate
fails, the workflow does not advance to Prepare; it goes to Recover or stops.

## Orientation-before-mutation rule

Every skill that mutates state (git, config, runtime, release) must call this
gate before its first mutation step and record the five outputs. A step that
begins with a mutation command without a prior orientation gate is a violation
of this skill and of `aphrodite-boundaries`.

## Local test matrix

| Claim                                | Evidence source        | Test                                                | Pass condition                                   | Failure response                           |
| ------------------------------------ | ---------------------- | --------------------------------------------------- | ------------------------------------------------ | ------------------------------------------ |
| Gate commands are read-only          | This skill             | Run the 5 commands, then `git status --short` twice | Identical output; no state change                | Remove the mutating command                |
| Wrong-root detection stops work      | This skill             | Run gate in a non-Aphrodite directory               | Output root differs; workflow halts              | Fix the interpretation rule                |
| Submodule classification is accurate | `git submodule status` | Check out a known-detached submodule, run gate      | `-` prefix detected and diagnosis path taken     | Fix the classification logic               |
| Auto-committer baseline is captured  | `git rev-parse HEAD`   | Run gate under active auto-committer                | HEAD + remote tip recorded before first mutation | Enforce baseline capture in every workflow |
