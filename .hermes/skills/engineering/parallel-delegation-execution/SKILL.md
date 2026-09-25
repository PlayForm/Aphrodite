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

- **Disjoint file ownership is the hard constraint.** Every child names the exact files it may create/modify and forbids anything else; never parallelize shared files (manifests, .gitmodules, directory renames) - do those first; split one shared integration file across owners; consumers of a new API go in a LATER wave than the API's creators. Full: references/ownership-scope.md.
- **Children know nothing of this conversation.** Embed the full contract in each task: repo absolute path, house style (tabs, license, author), the required docs/instruction files to read, the exact type/API shapes to build against, and the verification command with its expected output. Self-contained tasks do not drift. CLAIM.
- **Batch 4-at-a-time (runtime cap).** `delegate_task(tasks=[...])` runs the batch concurrently; `delegation.max_concurrent_children` caps it. Do not poll transcripts to wait - the batch re-enters the conversation on completion. Steer (not stop) a child that drifts - steer text is queued to its next tool result; a landed completion reports `missed_steer`; fold a late scope-extension request into the integration pass. Full: references/wave-steering.md.
- **Pair-shaped waves are the standing default.** Dispatch exactly two children with disjoint file ownership per phase; four is the runtime cap, not the default shape (3-4 one-file-owner surfaces can be a legitimate three-way wave). Full: references/ownership-scope.md.
- **Self-reports are not facts - a child's "3x green" suite is not green in YOUR env.** A child's subprocess may not inherit the parent's exported env vars, so re-run the exact command in your own environment before believing a clean child suite; conflicts between children's reports resolve by reading on-disk state. Full: references/env-var-hermeticity.md.
- **Keep progress in the repo planning archive** (`.hermes/notes/` - the Aphrodite family convention; confirm with `git check-ignore -v` before choosing a directory, since creating the wrong dir leaves an unignored directory the auto-committer can sweep) as a plan file: checkboxes per phase, updated after each wave, so the next session resumes from the topmost unchecked task. Never /tmp - the plan file is what the next session resumes from. The archive may be gitignored by design (`git check-ignore -v <path>` confirms): its edits never appear in `git status`/`git log` - an empty diff is the expected state, not evidence of a lost write. A single long-running verification is a valid one-child delegation: dispatch it, do not await it; the child polls as a BOUNDED FOREGROUND command, never a background process (a subagent-owned background process is KILLED when the child exits; the task returns "awaiting notification" incomplete). Full: references/completion-verification.md.
- **Finish the sweep directly, never by more delegation.** Once the parallel implementation waves land and are verified, the "proceed until everything is done" pass is executed by the orchestrator itself in-session (fixes, wiring, docs, CI, probes, archive updates) - no more workers/agents. Children are for parallel implementation; finishing touches are single-threaded by preference.
- **Relaunch a failed child - never hand-execute its scope mid-wave (standing orchestration preference).** When a child dies (401/429/timeout), re-dispatch it instead of writing its files yourself: the parent organizes and verifies, children implement. This governs scope a child already OWNS mid-wave; it does not repeal "finish the sweep directly", which governs the final integration pass after the waves land.
- **Read the NEWEST planning/instruction file before executing a mass change.** Later directives supersede earlier ones and can reverse a rename or brand decision made earlier in the same session - a scripted mass rewrite is costly to undo.

## Wave structure

1. Parent does the shared-file foundation first (renames, manifests, .gitmodules, symlinks) and verifies it (cargo test / smoke probe green). Input: the pre-wave tree. Output: a shared foundation no child may write.
2. Wave 1: new modules, one child per file, each with the exact public-API contract it must expose (other children will import those names). Input: the verified foundation plus per-child ownership lists. Output: new module files with the promised API.
3. Parent integrates: run the workspace test once after the wave; fix cross-module mismatches yourself (children cannot see each other's code). Input: the wave's files. Output: a green workspace test run.
4. Wave N: dependent layers (CLI binary, tests, installers) - same rules. Input: the integrated tree. Output: the dependent-layer files.
5. Before declaring done, run the full verification ladder yourself and record results in the planning archive. Input: the final tree. Output: archive records plus the done state.

## Validate-then-fix issue waves (triage pairs)

For "check if these issues are valid and fixable" runs, split into two phases per task, both pair-shaped:

- **Parent establishes the repro baseline itself first** (run the failing test, stage the shallow path, trace the claimed code path) and verifies each claim's DIRECTION, not just its existence; grep the test suite for tests that LOCK current behavior before dispatching any behavioral 'improvement'; when a fix names a dependency, grep the workspace Cargo.toml and lockfile first. Test cases: references/scar-catalog.md.
2. **Validation pair (read-only).** Two children, each owning a DISJOINT slice of the issue's claims (e.g. one takes the Python/runtime side, the other the scripts/docs/Rust side). Contract in every task: read-only - no repo edits, no git operations; verdict per claim as valid / invalid / partially-valid with line-numbered evidence, root cause, and a concrete fix proposal naming the exact file+function it would touch; return structured JSON (an output_schema makes the reports comparable across the pair).
3. **Fix pair (write).** Dispatched per task in the same order, with file ownership lifted straight from the validation proposals, so the two fix children never collide.
4. **Sequence tasks one pair at a time when the user lists them "in order"** - do not batch pairs across tasks even when the concurrency cap allows; the user's ordering is a dependency, not a suggestion.
5. **"don't commit or release" is the standing gate for these runs:** state it verbatim in every child context alongside the auto-commit warning, and verify `git status` is clean of new commits after each wave.
- **Fold tightly-coupled tasks into ONE pair instead of two.** When an issue and a PR (or two listed tasks) touch the SAME files and one claims to fix the other, run them as a single pair with ownership split across the shared surface; state the fold in the planning archive. Full: references/ownership-scope.md.
- **Reverify wave (standing default): a final READ-ONLY verification pair after all fix waves land** - agent A re-runs the test ladder and spot-checks fixes; agent B verifies git state, removals, archive verdicts, credit trail. A reverify child that dies on 429 with results in the transcript still counts. Full: references/completion-verification.md.
- **"Research more" = additional read-only pairs, not a fix wave** (history/audit, fix-design, empirical battery, system audit), each with a shared report-naming scheme in the notes archive; the parent VERIFIES children's factual claims against the tree before reporting them as findings; do not implement in this mode. Full: references/completion-verification.md.
- **End-of-session cleanup scope.** Delete ONLY session-created paths (scratch dirs and files, temp branches, worktrees, duplicate clones) - list mtimes first; verify pre-existing content stays untouched; symlinked homes resolve scratch/worktree paths to the SAME physical directory. Full: references/ownership-scope.md.

Pitfall (reproduction masking): on macOS `/tmp` is a symlink to `/private/tmp` and ADDS a path level - stage at a genuinely shallow path (fewer than 4 parents, e.g. `/opt/<name>`), or compute the parents length directly. Full: references/wave-dispatch.md.

## Pitfalls

- **`delegate_task` tasks JSON can refuse to parse** (content-shape driven, not content-dependent). Fixes in order: (1) write the long brief into a repo file and dispatch a SHORT context 'read <path> and follow it exactly'; (2) collapse to single-line prose and drop the `output_schema`; after TWO failed attempts, switch to the file-pointer strategy - never a third dispatch on the same payload. Probes: references/wave-dispatch.md.
- **Goal text must contain no literal `{...}` brace literals** - the validator refuses the whole batch on unexpanded template markers; the same rejection hits ANGLE-BRACKET placeholders (`gh run watch <RUN_ID>`). Phrase URLs, JSON bodies, and paths in words; substitute every placeholder. Full: references/wave-dispatch.md.
- **Sibling subagents race on shared files even when told otherwise** - write_file refuses with `stale_write_blocked` when the file changed since this task's last read; re-read then retry, or use patch for small edits; net-new files bypass the guard entirely. Full: references/ownership-scope.md.
- **The external auto-committer is a standing third writer** - warn children "do NOT run git commit; an external auto-commit may run - ignore it"; if auto-commits appear, report them to the user rather than fighting; the planning baseline is the tree at brief-writing time, never the session-start snapshot. Test case: references/scar-catalog.md.
- **A wave's green suite in the PRIMARY crate is not green in the WORKSPACE** - `cargo test -p <primary>` does not compile sibling crates and `cargo check` does not compile `#[cfg(test)]` code; a child's 'all consumers compatible' claim is a hypothesis until you grep the changed symbol's usages across ALL crates and run every consumer crate's test target. Full: references/rust-edits.md.
- **A dependency that 'is already in Cargo.toml' can still be feature-gated** - `optional = true` behind a feature means the crate compiles without it (E0433 at a sibling's compile); check the manifest line; move it to an unconditional dependency or gate the usage behind the same feature. Full: references/rust-edits.md.
- **Mid-wave steering is queued, not immediate: steer EVERY affected child and check for `missed_steer`.** Briefs naming now-gone paths stay live in children that already read them; when a second task overlaps a running child's scope, steer the child to DROP those files; steer text lands on the child's NEXT tool result; a completion landing before the delivery boundary reports `missed_steer`. Full: references/wave-steering.md.
- **Mid-wave SAFETY HOLD: steer every running child whose brief mutates the held surface to PLAN-ONLY mode** (no git rm, no file edits, no branch checkouts, no force ops); verify NO damage occurred before the steers landed and switch stray branches back; re-check `git status`/`branch --show-current` per held repo. Full: references/wave-steering.md.
- **User pauses the whole operation mid-wave ("pause for now, continue tomorrow")** - stop every live child (`delegate_task action=stop`), write a `## PAUSED <date>` resume checklist, and fold late-arriving completions into the resume state. Full: references/wave-steering.md.
- **Two pairs needing the SAME file for different concerns = sequential waves, not a split** - queue pair B until pair A's completion report lands; tell B the file is free and to re-read the on-disk state, not assumed line numbers. Full: references/ownership-scope.md.
- **Every static value in an instruction file (brief, skill, plan file) must carry a property - a derivation path - so it is inferable at read/write time** (versions -> BINARY_VERSION at handshake time; ports -> read live; counts -> count from source; thresholds -> live [compression] value; tag examples -> pattern + live version); ALGORITHM and MODULE-LOCATION claims: grep the source, correct IN PLACE, never append 'UPDATE: actually...'. Worked instances: references/static-value-properties.md.
- **AGENTS.md / CLAUDE.md are guard-protected agent-instruction files: a patch triggers a per-edit approval prompt; a timed-out prompt blocks ONLY that edit ("Silence is not consent"), while sibling patches on the same file in the same batch can still land.** Send parent-side map edits (skill lists, version lines, doctrine fixes in AGENTS.md) as separate small patches and expect some to be blocked when no user is present; never retry a blocked edit via another path (terminal, write_file, a reworded patch) - the block is binding. Verify which patches landed with a grep and leave the blocked line for the user. Full recipe for rewriting a repo skill library with parallel waves: the skill-library-refactoring skill.
- **A doc-rewrite child can smash an entire table onto ONE physical line with literal `\n` escapes - verify row structure after the wave and restore from HEAD, not the index** (the corruption is usually STAGED: `git reset HEAD -- <file>` then `git checkout -- <file>`; check `git log --oneline -- <file>` before restoring; the child's completion summary truncation is evidence of neither file health nor corruption). Test cases: references/scar-catalog.md.
- **Provider per-minute inference quota caps parallel fan-out: 429s mean the wave is too wide - re-dispatch failed units SEQUENTIALLY.** An INSTANT death (sub-second 429/401) has produced NOTHING - re-dispatch; a LATE death has USUALLY landed the full deliverable - verify on disk, do NOT re-dispatch; after a MIXED 429+401 cluster, probe with ONE minimal agent before re-fanning 2-wide. Worked waves: references/quota-death-recovery.md.
- **Standing rules for subagent deliverables - state them verbatim in every brief:** (1) deliverables go ONLY into the repo's notes taxonomy (`.hermes/notes/<category>/`), never a scratch dir; (2) external documents are ADAPTED, never copied - distill into ORIGINAL prose in the repo's voice; (3) zero local absolute paths in any deliverable. Full: references/doc-rewrite-waves.md.

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