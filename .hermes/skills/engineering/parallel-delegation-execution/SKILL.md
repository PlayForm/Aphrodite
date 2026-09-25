---
name: parallel-delegation-execution
description: "Use when running large repo tasks as parallel delegate waves on the Aphrodite monorepo (PlayForm/Aphrodite, Development branch). Dispatch disjoint-ownership delegate_task batches, check completions against on-disk evidence, and integrate in the parent env."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: engineering
category_taxonomy: engineering/parallel-delegation-execution
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, delegating, wave-planning, owning, verifying, steering]
        related_skills:
            - hermes-agent
            - plan
            - simplify-code
            - skill-library-refactoring
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
owns:
    - Wave planning and dispatch of delegate_task children
    - Disjoint file ownership allocation across parallel waves
    - Post-wave verification and integration passes
depends_on:
    - hermes-agent (delegate_task semantics, runtime cap)
    - aphrodite-orientation (preflight gate before any wave)
verification:
    source_of_truth:
        - .hermes/AGENTS.md (repo facts, branch conventions)
mutation_level: orchestration
---

# Parallel delegation execution (Aphrodite)

Run large multi-crate tasks on the Aphrodite monorepo (PlayForm/Aphrodite, Development branch) - big renames, feature build-out, issue-backlog sweeps, doc rewrites - as waves of parallel `delegate_task` subagents instead of everything sequentially in the parent context. Standing workflow: break the task into small pieces, launch several at a time (up to the runtime cap), keep progress in the repo planning archive (`.hermes/notes/`), never /tmp.

## Always-on rules

- **Disjoint file ownership is the hard constraint** - every child names the exact files it may touch and forbids anything else; never parallelize shared files - do those first; split one shared integration file across owners; API consumers go in a LATER wave than the API's creators. Full: references/ownership-scope.md.
- **Children know nothing of this conversation** - embed the full contract in each task (repo absolute path, house style (tabs, license, author), required docs/instruction files, exact type/API shapes, verification command + expected output); self-contained tasks do not drift. CLAIM. Full: references/wave-dispatch.md.
- **Batch 4-at-a-time (runtime cap).** `delegate_task(tasks=[...])` runs batches concurrently; `delegation.max_concurrent_children` caps it; do not poll transcripts - the batch re-enters on completion. Steer (not stop) a drifting child - steer text is queued to its next tool result; a landed completion reports `missed_steer`. Full: references/wave-steering.md.
- **Pair-shaped waves are the standing default** - exactly two children with disjoint file ownership per phase; four is the runtime cap, not the default shape; 3-4 one-file-owner surfaces can be a legitimate three-way wave. Full: references/ownership-scope.md.
- **Self-reports are not facts - a child's "3x green" suite is not green in YOUR env**; its subprocess may not inherit the parent's exported env vars, so re-run the exact command yourself; conflicting reports resolve by reading on-disk state. Full: references/env-var-hermeticity.md.
- **Keep progress in the repo planning archive** (`.hermes/notes/`; confirm with `git check-ignore -v` first; may be gitignored by design - an empty diff is expected; never /tmp). A single long-running verification is a valid one-child delegation: dispatch it, do not await it; the child polls as a BOUNDED FOREGROUND command, never a background process (killed when the child exits). Full: references/completion-verification.md.
- **Finish the sweep directly, never by more delegation** - once the parallel implementation waves land and are verified, the final pass is executed by the orchestrator itself in-session - no more workers/agents; children are for parallel implementation; finishing touches are single-threaded by preference. Full: references/ownership-scope.md.
- **Relaunch a failed child - never hand-execute its scope mid-wave (standing orchestration preference)** - when a child dies (401/429/timeout), re-dispatch it instead of writing its files yourself; the parent organizes and verifies, children implement. Full: references/quota-death-recovery.md.
- **Read the NEWEST planning/instruction file before executing a mass change** - later directives supersede earlier ones and can reverse a rename or brand decision; a scripted mass rewrite is costly to undo. Full: references/wave-dispatch.md.

## Wave structure

1. Parent does the shared-file foundation first; 2. Wave 1: new modules, one child per file with the exact public-API contract; 3. Parent integrates (workspace test once, fix cross-module mismatches yourself); 4. Wave N: dependent layers; 5. Before done, run the full verification ladder yourself and record results in the planning archive. Full: references/wave-dispatch.md.

## Validate-then-fix issue waves (triage pairs)

For "check if these issues are valid and fixable" runs, split into two phases per task, both pair-shaped:

- **Parent establishes the repro baseline itself first** - verify each claim's DIRECTION, not just its existence; grep the test suite for tests that LOCK current behavior before any behavioral 'improvement'; when a fix names a dependency, grep the workspace Cargo.toml and lockfile first. Test cases: references/scar-catalog.md.
2. **Validation pair (read-only).** Two children, each owning a DISJOINT slice of the issue's claims; verdict per claim (valid / invalid / partially-valid) with line-numbered evidence, root cause, and a fix proposal naming the exact file+function; return structured JSON (output_schema). Full: references/completion-verification.md.
3. **Fix pair (write).** Dispatched per task in the same order, with file ownership lifted straight from the validation proposals, so the two fix children never collide.
4. **Sequence tasks one pair at a time when the user lists them "in order"** - do not batch pairs across tasks even when the concurrency cap allows; the user's ordering is a dependency, not a suggestion.
5. **"don't commit or release" is the standing gate for these runs:** state it verbatim in every child context alongside the auto-commit warning, and verify `git status` is clean of new commits after each wave.
- **Fold tightly-coupled tasks into ONE pair instead of two** - when an issue and a PR (or two listed tasks) touch the SAME files and one claims to fix the other, run one pair with ownership split across the shared surface; two separate pairs would double-edit the same file sequentially. Full: references/ownership-scope.md.
- **Reverify wave (standing default): a final READ-ONLY verification pair after all fix waves land** - agent A re-runs the test ladder + spot-checks fixes; agent B verifies git state, removals, archive verdicts; both read-only. Full: references/completion-verification.md.
- **"Research more" = additional read-only pairs, not a fix wave** (history/audit, fix-design, empirical battery, system audit); the parent VERIFIES children's factual claims against the tree; do not implement in this mode. Full: references/completion-verification.md.
- **End-of-session cleanup scope.** Delete ONLY session-created paths (scratch, temp branches, worktrees, duplicate clones) - list mtimes first; symlinked homes resolve scratch/worktree paths to the SAME physical directory. Full: references/ownership-scope.md.

Pitfall (reproduction masking): on macOS `/tmp` is a symlink to `/private/tmp` and ADDS a path level - stage at a genuinely shallow path (fewer than 4 parents, e.g. `/opt/<name>`), or compute the parents length directly. Full: references/wave-dispatch.md.

## Pitfalls

- **`delegate_task` tasks JSON can refuse to parse** (content-shape driven). Fixes: (1) write the long brief into a repo file and dispatch a SHORT context 'read <path> and follow it exactly'; (2) collapse to single-line prose and drop the `output_schema`; after TWO failed attempts, switch to the file-pointer strategy - never a third dispatch on the same payload. Probes: references/wave-dispatch.md.
- **Goal text must contain no literal `{...}` brace literals** - the validator refuses the whole batch on unexpanded template markers; the same rejection hits ANGLE-BRACKET placeholders; substitute every placeholder before calling. Full: references/wave-dispatch.md.
- **Sibling subagents race on shared files even when told otherwise** - `stale_write_blocked` when the file changed since this task's last read; re-read then retry, or use patch; net-new files bypass the guard. Full: references/ownership-scope.md.
- **The external auto-committer is a standing third writer** - warn children 'do NOT run git commit; an external auto-commit may run - ignore it'; report auto-commits rather than fighting; the planning baseline is the tree at brief-writing time, never the session-start snapshot. Test case: references/scar-catalog.md.
- **A wave's green suite in the PRIMARY crate is not green in the WORKSPACE** - `cargo test -p <primary>` skips sibling crates, `cargo check` skips `#[cfg(test)]` code; grep the changed symbol's usages across ALL crates and run every consumer crate's test target. Full: references/rust-edits.md.
- **A dependency that 'is already in Cargo.toml' can still be feature-gated** - `optional = true` behind a feature means E0433 at a sibling's compile; move to unconditional or gate the usage behind the same feature. Full: references/rust-edits.md.
- **Mid-wave steering is queued, not immediate: steer EVERY affected child and check for `missed_steer`** - briefs naming now-gone paths stay live; overlapping scope must be DROPPED; steer text lands on the child's NEXT tool result. Full: references/wave-steering.md.
- **Mid-wave SAFETY HOLD: steer every running child whose brief mutates the held surface to PLAN-ONLY mode** (no git rm, no file edits, no branch checkouts, no force ops); verify NO damage before the steers landed; switch stray branches back. Full: references/wave-steering.md.
- **User pauses the whole operation mid-wave ("pause for now, continue tomorrow")** - stop every live child, write a `## PAUSED <date>` resume checklist, fold late-arriving completions into the resume state. Full: references/wave-steering.md.
- **Two pairs needing the SAME file for different concerns = sequential waves, not a split** - queue pair B until pair A's completion report lands; re-read the on-disk state, not memorized line numbers. Full: references/ownership-scope.md.
- **Every static value in an instruction file must carry a property - a derivation path - so it is inferable at read/write time** (versions -> BINARY_VERSION, ports -> read live, counts -> count from source, thresholds -> live [compression] value, tags -> pattern + live version); ALGORITHM/MODULE-LOCATION claims: grep the source, correct IN PLACE. Worked instances: references/static-value-properties.md.
- **AGENTS.md / CLAUDE.md are guard-protected agent-instruction files: a patch triggers a per-edit approval prompt** - a timed-out prompt blocks ONLY that edit (sibling patches on the same file can still land); never retry a blocked edit via another path - the block is binding; verify which patches landed with a grep. Full: references/ownership-scope.md. Full recipe for rewriting a repo skill library with parallel waves: the skill-library-refactoring skill.
- **A doc-rewrite child can smash a table onto ONE physical line with literal `\n` escapes - verify row structure after the wave**; the corruption is usually STAGED: unstage (`git reset HEAD -- <file>`) then `git checkout -- <file>`; check `git log --oneline -- <file>` before restoring. Test cases: references/scar-catalog.md.
- **Provider per-minute inference quota caps fan-out: 429s mean the wave is too wide - re-dispatch failed units SEQUENTIALLY.** An INSTANT death (sub-second, 429/401) has produced NOTHING - re-dispatch; a LATE death has USUALLY landed the full deliverable - verify on disk, do NOT re-dispatch; after a MIXED 429+401 cluster, probe with ONE minimal agent before re-fanning 2-wide. Worked waves: references/quota-death-recovery.md.
- **Standing rules for subagent deliverables - state verbatim in every brief:** (1) only into the repo's notes taxonomy, never a scratch dir; (2) external documents ADAPTED, never copied - distill into ORIGINAL prose; (3) zero local absolute paths. Full: references/doc-rewrite-waves.md.

## References

- references/wave-dispatch.md - dispatch payloads: JSON serialization probes, brace/angle validator, remote-only briefs, new-files-only dispatch, 5-minute steer, stale /tmp harnesses.
- references/wave-steering.md - mid-wave steering: user corrections, ownership transfer, SAFETY HOLD, pause, review-turn deferral.
- references/completion-verification.md - completion evidence: background-process notifications, "Completed"-status deaths, 1800s timeouts, batch-timeout tail, partial-progress resumes, exchange files, no mid-wave workspace tests.
- references/quota-death-recovery.md - 429/401 signatures, instant vs late, mixed-cluster probe, worked waves.
- references/ownership-scope.md - ownership and scoping: helper owners, contained fixes, pivot deltas, brief paths, removals, same-file sequencing, git mv, absolute paths, merge children, worktrees, scratch clones.
- references/rust-edits.md - crate-level verification: consumer test targets, feature-gated deps, formatter, tab-loss byte-compare.
- references/doc-rewrite-waves.md - doc waves: restyle recipe, deliverable-vs-scratch, mermaid braces, verify-against-source, scope-limited sweeps, user-paced conversions.
- references/static-value-properties.md - derivation-path instances (BINARY_VERSION, `:9798`, counts, 512B, tag pattern, config.rs:250-251, proxy.rs:2668-2669, BLAKE3/SHA256/`_inline_store`/`inline_store`).
- references/env-var-hermeticity.md - the exported-env-var suite landmine and hermetic config-test recipe.
- references/scar-catalog.md - test cases: stale snapshot, scope-limited sweep, table smash, completed-status death, env landmine, batch-timeout tail, stale /tmp harness.
- references/verify-platform-gated-ctypes.md - (existing) verify platform-gated ctypes fixes on the host OS with a fake `ctypes.windll` (naive fakes silently produce all-True results); closure-based recipe.

## Stop if / Recovery

- **Stop if** two children in the same batch share a write target. **Recovery:** re-split ownership before dispatching (wave gate: Dispatch).
- **Stop if** a batch produces 429/401 deaths. **Recovery:** let survivors finish; re-dispatch dead units one at a time or 2-wide; after a mixed cluster, probe with ONE minimal task before re-fanning out. Signatures: references/quota-death-recovery.md.
- **Stop if** on-disk evidence does not match a child's report. **Recovery:** re-dispatch only the undelivered items; a late death whose deliverable is verified on disk is not re-dispatched.
- **Stop if** the user pauses the operation mid-wave. **Recovery:** `delegate_task action=stop` every live child, write a `## PAUSED <date>` resume checklist, fold late-arriving completions into it.

## Wave gates

| Gate        | Check                                                     | Pass condition                                    | Failure response                                 |
| ----------- | --------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------ |
| Dispatch    | File ownership disjoint across all children in the batch  | No shared write target                            | Re-split ownership before dispatching            |
| Completion  | On-disk evidence matches the child's report               | grep/test proves the deliverable landed           | Re-dispatch only the undelivered items           |
| Integration | Workspace build + tests in the PARENT env after each wave | `cargo test` green; cross-module mismatches fixed | Fix mismatches yourself, never via another child |
| Final       | Full ladder + scans (fmt/clippy, link, mermaid, prettier) | All scans clean; plan file marked COMPLETE        | Fix in-session; record results in the archive    |