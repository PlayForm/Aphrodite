---
name: turn-budgeted-problem-solving
description: "Use when a task needs deep multi-step reasoning or debugging + implementation: decompose into parts, one agent per part, one concept per turn, bounded reasoning blocks."
version: 2.0.0
author: Hermes Agent
license: MIT
platforms: [macos, linux]
category: engineering
category_taxonomy: engineering/turn-budgeted-problem-solving
date: 2026-10-18
metadata:
    hermes:
        tags:
            [
                reasoning,
                delegation,
                debugging,
                implementation,
                turns,
                decomposition,
                orchestration,
                monoid,
                ledger,
                composition,
            ]
        related_skills:
            [parallel-subagent-orchestration, aphrodite-testing-discipline, aphrodite-boundaries]
status: active
mutation_level: local
---

# Turn-Budgeted Problem Solving

For tasks that need a LOT of reasoning before, during, and after implementation -
debugging a subtle failure, designing a fix, implementing it, verifying it -
where the full problem does not fit in one reasoning block. The failure mode it
prevents: a single agent (or the parent session) tries to hold the WHOLE problem
in one turn, reasons shallowly across many concepts at once, and either misses
the root cause or iterates on one request for many turns (the hook-wedge
session: 40+ turns of inline diagnosis before fanning out).

The deeper purpose: **disperse conclusions and assumptions as far and wide as
possible** - every finding feeds the previous research (as reference) and
expands into subsequent ones (as dependency). This skill formalizes that
dispersion as a **functional, monoidal complex over a shared findings ledger**
(§The monoidal complex) - not merely a workflow.

## When to use

- The task is a debugging + fix + verify cycle with an unknown root cause.
- The problem spans multiple files/modules and multiple disciplines
  (e.g. Rust dylib + Python loader + Hermes gateway runtime).
- A single reasoning block would have to cover 5+ distinct concepts.
- You catch yourself iterating on one request repeatedly (same failure,
  new turn) instead of decomposing.
- The conclusions should be reusable by later research or parallel
  workstreams (the dispersion requirement).

## The three-level pyramid

```
PROBLEM  ── one task, one goal, one acceptance criterion
│
├─ PART 1   one agent, owns a DISJOINT slice of the problem
│    Turn 1  concept: the registry design        → reason → edit → prove → LEDGER
│    Turn 2  concept: the config template        → reason → edit → prove → LEDGER
│    ...     one concept per turn, bounded
├─ PART 2   another agent, DISJOINT filesystem/paths (runs in parallel)
│    Turn 1  concept: reset defaults             → ... → LEDGER
│    Turn 2  concept: reinstall                  → ... → LEDGER
└─ PART 3   parent-held: approval-gated ceremony steps
```

- **Problem** = the task as given.
- **Part** = a bigger slice of the problem, owned by exactly ONE agent.
- **Turn** = one concept inside that part; the agent reasons ONLY about that
  concept, makes the smallest edit, proves it, reports, and appends its
  conclusion to the shared ledger.
- Parts parallelize ONLY on disjoint ownership (different files, crates,
  filesystems). Never two agents on the same file/crate/folder.
- Sequential parts stay gated: install depends on the fix build; benchmark
  depends on install; tag depends on all.

## The monoidal complex (formal core)

Reasoning is modeled as a **functional monoid over a shared findings ledger**
(associative merge `⊕` with identity `ε`); the toroidal extension adds two
independent cycles (below). This is what makes dispersed conclusions compose:

- **Every turn is a function.** Input: current context + ledger. Output:
  enriched context + one ledger entry `{key, conclusion, assumption,
evidence}`. A turn that produces no ledger entry produced no reasoning.
- **Composition is associative.** `T3 ∘ (T2 ∘ T1) = (T3 ∘ T2) ∘ T1` - the
  merged findings are identical regardless of grouping, so parallel parts can
  merge into the ledger in ANY order. That associativity licenses dispersion:
  agents never need a fixed order to contribute.
- **The ledger is the memory.** Every conclusion AND every assumption is
  written to the ledger - not just the winner. Assumptions are as valuable as
  conclusions: a later turn may verify or refute them. This is exactly how
  one turn's output feeds prior research (reference) or expands subsequent
  ones (dependency).
- **The orchestrator is the fold.** The parent reduces the ledger:
  `fold(ε, [T1..Tn])` = the final Context. It aggregates solutions across
  turns, including options agents rejected - those entries carry the
  trade-off reasoning the final pick needs.
- **Dispersion rule**: a conclusion that could inform another part MUST be
  appended to the ledger before the turn ends; a conclusion kept only in the
  agent's head is lost. Practical ledger: the part's transcript + a notes
  file (e.g. `.hermes/tmp/<task>/findings.md`), or a skill reference when the
  finding is durable.

### The toroidal extension (two independent cycles)

The monoid generalizes to a **toroid**: two independent composition cycles on
the ledger surface - the sequence cycle (turn n → n+1, last wraps back to the
anchor) and the parts cycle (part A's findings wrap to inform part B; the
final fold wraps into the next research pass's anchor). A finding sits at
(sequence × parts) and routes to ANY combination of later turns and parts -
more configurations than a linear monoid. The ledger is never finished:
durable findings wrap into the owning skill's references. Formal definition:
`references/monoidal-reasoning-complex.md`.

## Procedure

1. **Write the anchor yourself before dispatching.** The root-cause evidence,
   the binding contract (FFI surface, semantics that must not change), the
   file-ownership table, the exact verification commands, and the
   one-concept-per-turn sequence. Children know nothing of the conversation -
   repeat the shared background in EVERY task context.
2. **Create the ledger up front.** A shared findings file (gitignored scratch
   or a notes path) with the anchor's assumptions already entered, so the
   first turn composes onto a non-empty monoid, not from ε.
3. **Break the part into its turn sequence up front** and write it into the
   goal: "Turn 1: concept X → implement ONLY X. Turn 2: concept Y → ..." with a
   strict order and a single-blocker rule (if a turn's concept is blocked,
   report that one blocker and stop; never sweep ahead). Instruct EVERY turn
   to append `{conclusion, assumption, evidence}` to the ledger.
4. **Dispatch parallel parts on disjoint slices in the same turn.** Runtime
   paths (~/.hermes) vs repo crates vs scratch are disjoint - parallel is safe.
5. **Steer, don't re-dispatch.** If an agent starts sweeping multiple concepts
   per turn, queue a steer with the one-concept-per-turn rule and the numbered
   sequence.
6. **Verify on disk yourself.** A child's self-report is not verification.
   After each batch: check the artifacts exist, spot-grep the contract
   marker, run the real build/test.
7. **Fold the ledger (fan-in at the parent).** Read every part's ledger
   entries, resolve conflicts between competing solutions, choose the
   aggregate, and record the decision - including which rejected options were
   considered and why. Approval-gated steps (tag, push, publish) never leave
   the parent.
8. **Promote durable findings.** A conclusion that will matter beyond this
   task (a new contract, a reusable probe, a pitfall) is written into the
   owning skill's references - the ledger entry becomes a skill asset.

## One-concept-per-turn rules (the core discipline)

- Each turn answers ONE question: "what is the correct design/behavior for
  THIS concept?" - read the exact code for it, reason, make the smallest edit,
  prove it (compile/test/probe), report.
- **Granularity = SUBJECT, never effort.** The purpose is NOT to shorten
  reasoning - it is to ALLOW full reasoning on the smallest coherent subject.
  A turn's subject can be a single design decision, one function's semantics,
  one edge case, one failure mode, one contract choice - even a one-line
  change deserves its own turn if the reasoning about it is what matters.
  What bounds the turn is the subject, never the diff size or elapsed time.
- **Full depth per subject.** Each turn reasons as deeply as the subject
  warrants: read the surrounding code, call sites, tests, and alternative
  approaches - never rush because the change looks small.
- **Be EXPANSIVE within the subject.** The subject bounds WHAT is reasoned
  about, never HOW MANY solutions are considered. Enumerate every plausible
  candidate - design A vs B vs C, each with its trade-offs, failure modes,
  and compatibility cost - then argue for the best. A turn that reports one
  option without exploring alternatives is under-reasoned, even if the chosen
  fix is a one-liner. The option space IS the deliverable; the orchestrator
  picks from the reasoned set (including options the agent rejected, which
  may win on constraints the agent did not see).
- **Different solutions surface across turns, not within one.** The same
  granular subject may be reasoned about from different angles by different
  agents/turns; the ORCHESTRATOR aggregates the best solution from subsequent
  feedback. A turn reports its solution + reasoning; the parent decides.
- Never combine concepts in one turn ("implement the registry AND the bounded
  lock AND the watcher"). That is the shallow-reasoning failure this skill
  exists to prevent.
- A turn's proof is its exit evidence: if the concept compiles/tests/probes
  green, it is done; move to the next. If blocked, report the single blocker.
- Deep reasoning is the POINT of the turn: read the surrounding code, the
  call sites, the tests - not just the edit target.

## Good granular subjects (examples)

A turn subject is one of: a design decision (registry vs single state), a
root-cause hypothesis (lock poisoning vs stale dylib), a contract choice
(fail-open vs fail-closed per hook), one function's semantics
(`with_session` bounded wait), one edge case (empty session_id), one failure
mode (abandoned worker holds lock), one compatibility constraint (FFI surface
unchanged), one verification step (the isolation probe), one integration
point (config watcher all-sessions). If a subject still spans two of these,
split it.

Reusable turn-sequence templates (debugging, implementation, research,
migration): `references/turn-sequence-templates.md`.

## Orchestrator responsibilities

The parent is not a dispatcher - it is the **fold** of the monoid:

- Writes the anchor and the ledger seed (§Procedure 1-2).
- Decides part boundaries by DISJOINT ownership, not by convenience.
- Steers agents back to one-concept-per-turn when they sweep.
- Verifies every self-report on disk.
- Folds the ledger: aggregates solutions, resolves conflicts, records
  rejected options, promotes durable findings to skills.
- Holds every approval-gated step (tag/push/publish/remote side effects).

Full playbook: `references/orchestrator-playbook.md`.

## Pitfalls

- **Shared-crate collisions**: two agents editing the same crate collide.
  Assign disjoint crates/folders or sequence them.
- **Payload size**: delegate_task fails on large payloads with a JSON parse
  error - keep context+goal tight; put detail in a plan file the agent reads,
  or shrink.
- **Self-report ≠ done**: verify with real commands/artifacts, not the
  child's claim.
- **The auto-committer sweeps**: agents leave edits uncommitted; verify by
  content (grep/diff), not git status.
- **Approval-gated steps stay with the parent**: never delegate tag/push/
  publish; the parent presents the Ready-for-approval block.
- **A turn that appends no ledger entry produced no reasoning**: if an agent
  finishes a turn without writing its conclusion/assumption, steer it back.
- **A rejected option discarded from the ledger loses the trade-off**:
  rejected solutions carry the reasoning the final pick needs; keep them.

## Worked example

The Aphrodite hook-wedge (2026-10-18): a Hermes shared-gateway change made the
plugin's single global `Mutex<AphroditeState>` poison itself when a second
session appeared (abandoned timeout worker held the lock forever → every hook
failed closed → "plugin is hung"). The fix decomposed into:

- PART 1 agent (lib.rs + tools.rs): 10 turns, one concept each - registry →
  template → bounded try_lock → fail-open contract → session resolution →
  watcher all-sessions → FFI compat → cargo check → cargo test → clippy +
  probes. Steered mid-flight to one-concept-per-turn.
- PART 2 agent (runtime, ~/.hermes only, parallel): reset defaults → wipe
  state → retire stale proxy → (install gated on PART 1).
- PART 3 (parent-held): gateway restart, benchmark, release tag.

Extended multi-part trace with the ledger entries per turn:
`references/worked-example-hook-wedge.md`.

## Claim-to-test matrix

| Claim                                 | Evidence source                     | Test                                                      | Pass condition             | Failure response                     |
| ------------------------------------- | ----------------------------------- | --------------------------------------------------------- | -------------------------- | ------------------------------------ |
| One concept per turn                  | Goal text + agent transcript        | Count distinct concepts edited per turn in the transcript | ≤1 concept per turn        | Steer with the numbered sequence     |
| Parts are disjoint                    | File-ownership table vs agent edits | Diff each agent's touched paths                           | No overlap                 | Sequence or reassign                 |
| Self-reports verified                 | Real artifacts/builds               | Re-run check/test/probe after each batch                  | Green with recorded output | Fix or re-dispatch                   |
| Blocked turn stops                    | Agent report                        | Any turn reporting a blocker stops the agent              | Single blocker, no sweep   | Unblock or escalate                  |
| Every turn appends to the ledger      | Ledger file                         | Grep the ledger for a conclusion entry per completed turn | One entry per turn         | Steer to append; verify              |
| Merged findings are order-independent | Two parallel parts                  | Fold the ledger in two different orders                   | Identical final Context    | Fix the ledger merge (dedupe by key) |
| Durable findings reach the skill      | Skill references                    | Diff skill references after the task                      | New finding present        | Promote the ledger entry             |
