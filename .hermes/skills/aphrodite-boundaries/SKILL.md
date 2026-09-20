---
name: aphrodite-boundaries
description: "Use when any Aphrodite work touches state, git, context, secrets, or external side effects. Canonical owner of the universal boundary rules."
version: 1.0.0
platforms: [macos]
tags: [aphrodite, boundaries, safety, stop-conditions, recovery]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Universal boundary groups (state, git, context, release)
    - Core terminology: live, archived, owner, verify, stop, recovery, branch-owned identity
    - Branch-owned identity contract table
    - Git repair taxonomy
    - Secrets model and secret-handling rules
    - Human approval boundaries (authority to cause external side effects)
depends_on:
    - aphrodite-orientation (preflight gate before any mutation-bound workflow)
supersedes: []
verification:
    source_of_truth:
        - .hermes/governance/BOUNDARIES.md (same rules, governance copy)
        - .hermes/governance/VERIFICATION-MATRIX.md (probes for every claim)
mutation_level: read-only
---

# Aphrodite Boundaries

Canonical owner of every boundary rule in the Aphrodite skill system. Other
skills **reference** these rules, never copy them. This skill is the
read-only core-protocol layer: it declares what may never be done, who owns
what, and how recovery is permitted. It contains no repository-specific
commands (those live in `aphrodite-orientation` and the operational skills).

## Terminology (canonical definitions)

| Term                      | Definition                                                                                                                                                                                                                   |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **live**                  | A document or skill that may steer new work. Has `status: active`, a declared owner, scope, mutation level, and a test matrix.                                                                                               |
| **archived**              | A preserved, dated snapshot that may not steer new work. `status: archived`, `do_not_execute: true`, named successor, `historical_cutoff`. Updating an archive is prohibited except to correct archival metadata.            |
| **owner**                 | The single skill or area that makes an exclusive decision or canonically states a rule. Other skills link to the owner; they do not restate the rule.                                                                        |
| **verify**                | Run the named read-only probe (or inspect the named source) and compare against the expected output. Never accept a claim from prose or memory.                                                                              |
| **stop**                  | Halt the current workflow immediately, without "fixing forward". Gather evidence, then choose a permitted recovery or escalate.                                                                                              |
| **recovery**              | A repair operation that is explicitly permitted for a named situation. Anything not listed as permitted is prohibited.                                                                                                       |
| **branch-owned identity** | A path or property whose content is the identity of one release line and must never cross a transplant. Restorable only as part of an explicitly named promotion/sync ritual, and only from the branch's committed baseline. |

## Branch-owned identity contract

Machine-checkable contract for every path or property that carries release-line
identity. Invoked at three points in any promotion: **before staging**,
**after the controlled restore**, and **before tag creation**. A path is never
restored merely because it is inconvenient to merge; only a contract-declared
identity path qualifies.

| Path or property            | Development owner    | Current owner            | May cross promotion?       | Required post-sync check             |
| --------------------------- | -------------------- | ------------------------ | -------------------------- | ------------------------------------ |
| `.gitmodules` branch field  | `Development`        | `Current`                | No                         | Matches checked-out branch           |
| Workflow branch triggers    | Development triggers | Current triggers         | No                         | No foreign branch references         |
| `plugins/aphrodite` gitlink | Development lineage  | Released Current lineage | No, except explicit float  | Points to intended plugin commit     |
| Product source              | Accumulating changes | Release snapshot         | Yes, via controlled squash | Tests and diff review                |
| Release tags                | Never created        | Release-sync commit only | N/A                        | Tag resolves to exact release commit |

## Git repair taxonomy

Every recovery operation is classified. The **allowed response** is the only
permitted repair; the **forbidden shortcut** is never used, even when it looks
faster.

| Situation                                         | Allowed response                                                               | Forbidden shortcut                                   |
| ------------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Intended text/content is wrong                    | Edit the file directly, rerun verification                                     | `git reset` or checkout to erase uncertainty         |
| Branch-owned identity crossed during a transplant | Restore only named protected paths from that branch baseline                   | Blanket checkout/reset of the worktree               |
| Cherry-pick conflict                              | Resolve semantics, scan all changed files for markers, then continue           | Continuing after resolving only the visible conflict |
| Empty cherry-pick                                 | Verify equivalence; skip as already contained                                  | Treating it as a failed transfer                     |
| Phantom gitlink in submodule                      | Remove the indexed mode-160000 entry, validate recursively after later commits | Ignoring it because parent status looks clean        |
| Wrong published version                           | Release a new version                                                          | Retagging or reusing a registry version              |

## Universal boundaries (state)

- Never mutate a repository, plugin, release, remote, tag, or configuration
  until the relevant precondition has been verified.
- Never infer a live behavior from a skill's prose; inspect the named source
  or run the named probe when the behavior affects data safety, releases, or
  runtime correctness.
- Never continue after a failed verification merely because a later step
  might "fix it."
- Treat an empty result as a valid state only where the workflow says it is
  valid, such as an already-contained cherry-pick or an intentionally empty
  sync-back.

## Universal boundaries (git)

- A branch-owned path may be restored **only** as part of an explicitly named
  promotion or sync ritual, and only from the branch's committed baseline.
- Never use `git checkout`, `git restore`, `reset`, rebase, or force-push to
  repair accidental content changes; repair intended file content directly and
  validate it.
- A tag is immutable historical evidence: create it only after the exact
  release-sync commit and all tag prerequisites have passed.
- Work bottom-up across submodules: validate and publish the plugin commit
  before updating the parent gitlink.

## Universal boundaries (context)

- Never compress a retrieval response or an Aphrodite diagnostic response;
  doing so can turn the retrieval path into a marker-resolution loop.
- Never split a tool call from its matching tool result when selecting a
  context-compression boundary.
- Treat hook input structures as read-only unless the framework documents
  mutability; the `prellmcall` history is a copy, so in-place edits are
  discarded.
- A hook that replaces output must return the original output for pass-through;
  an empty string is a destructive replacement, not "no change."

## Universal boundaries (release)

- Never bump `BINARYVERSION` before the referenced release assets exist and
  are accessible.
- Never reuse a claimed version; check the registry before publishing a
  version that matters.
- Never claim a release is complete based on a plugin banner or Git state
  alone; use runtime probes plus artifact verification.

## Secrets model

- Verify only **presence**, source name, and redacted fingerprint-never print
  an API key.
- Precedence is defined and stable: environment overrides TOML; a credential
  store (if configured) sits below environment. State whether an empty value
  overrides a valid lower-precedence source.
- Child-process passthrough is tested without echoing secrets.
- On a missing key, emit a clear degraded-state message rather than an opaque
  proxy failure.
- Never include secrets in CCR payloads, previews, logs, benchmark fixtures,
  release notes, or issue reports.
- Rotate/revoke externally exposed secrets rather than relying on Git history
  cleanup.

## Human approval boundaries

Technical readiness ("safe to release") never authorizes an external side
effect. Each of the following pauses the workflow with an explicit summary
(`Ready for approval:` block listing commit, proposed tag, plugin commit,
expected triggered workflows, expected external publications, verification
gates passed, known degraded conditions):

- Commit creation
- Push
- Tag creation
- GitHub release publication
- Artifact attachment
- Workflow dispatch
- Registry publication
- Remote configuration change
- Deletion or history rewrite

## Failure behavior policy

Every hook and transform picks one policy; never leave it implicit:

| Policy              | Use when                                                  | Behavior                                             |
| ------------------- | --------------------------------------------------------- | ---------------------------------------------------- |
| **Fail open**       | Observability, optional optimization, preview enhancement | Log structured error; return original content        |
| **Fail closed**     | Security redaction, incompatible artifact validation      | Stop operation with readable diagnostic              |
| **Degrade**         | Upstream proxy unavailable                                | Retain local/raw behavior and expose degraded status |
| **Retry boundedly** | Transient local socket/process start                      | Limited retries with backoff; then degrade           |
| **Escalate**        | Data loss, invalid marker grammar, release side effect    | Halt workflow; require human decision                |

Compression and preview transformations ordinarily **fail open**; artifact
compatibility and registry publishing **fail closed**; a missing optional
release asset **degrades** with a warning.

## Local test matrix

| Claim                                          | Evidence source                        | Test                                                                 | Pass condition                                | Failure response                                       |
| ---------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------ |
| Terminology definitions are unique             | This skill                             | Grep other skills for conflicting definitions of live/archived/owner | No conflicting definition found               | Update the offending skill to link here                |
| Branch-owned identity contract rows match repo | `.gitmodules`, workflow files, gitlink | `aphrodite-orientation` orientation gate + I9/B4 checks              | No foreign identity statement on any branch   | Stop; run B4 audit                                     |
| Context rules prevent retrieval loops          | CCR transform pipeline                 | Compress a retrieval response and check for re-marking               | Retrieval payload stays raw; no nested marker | Update the skip classifier (owner: compression-safety) |
| Secrets never appear in outputs                | Session logs, fixtures                 | Grep fixtures/notes for key-shaped strings                           | Zero matches                                  | Redact and rotate                                      |
| Approval boundary pauses before side effects   | Workflow logs                          | Simulate a release gate without approval                             | Workflow halts at approval point              | Fix the gate                                           |
