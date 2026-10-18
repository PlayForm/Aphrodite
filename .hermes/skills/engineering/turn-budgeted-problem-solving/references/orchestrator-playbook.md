# Orchestrator Playbook

Owner: `turn-budgeted-problem-solving` (SKILL.md §"Orchestrator
responsibilities"). The parent is the FOLD of the monoid/toroid - not a
dispatcher. This reference is the full parent-side procedure.

## Role definition

- The parent writes the anchor, seeds the ledger, decides part boundaries,
  dispatches, steers, verifies, folds, promotes, and holds approval gates.
- Children never decide boundaries, never approve side effects, never merge
  the ledger.

## Step 0 - Decide whether to decompose

Decompose when: unknown root cause, multi-file/multi-discipline, 5+ distinct
concepts in one block, or you catch yourself re-iterating. Do NOT decompose
trivial single-concept tasks (one turn is enough).

## Step 1 - Write the anchor

The anchor is the binding standard every child needs. It MUST contain:

1. **Root-cause evidence** (or the question to resolve) - log lines, probe
   outputs, timestamps.
2. **The binding contract** - what must NOT change (FFI surface, semantics,
   compatibility).
3. **File-ownership table** - exactly which files/folders/filesystems each
   part owns; "write ONLY inside your folder".
4. **Verification commands** - the real gates with expected output.
5. **The turn sequence** - "Turn 1: concept X only. Turn 2: concept Y only."
6. **The single-blocker rule** - a blocked turn reports that ONE blocker and
   stops; never sweep ahead.
7. **The ledger instruction** - every turn appends
   `{key, conclusion, assumption, evidence}`.

Children know NOTHING of the parent conversation - every context repeats the
anchor. A child that must read a plan file gets the file path AND the
guardrails in the brief.

## Step 2 - Seed the ledger

Create the shared findings file (gitignored scratch, e.g.
`.hermes/tmp/<task>/findings.md`) with the anchor's assumptions already
entered. First turn composes onto a non-empty monoid.

## Step 3 - Decompose into parts (disjoint ownership)

- A part = one agent = one disjoint slice (files, crates, or filesystems).
- Never two agents on the same file/crate/folder (shared-crate collisions).
- Runtime paths (~/.hermes) vs repo crates vs scratch are disjoint - parallel
  is safe.
- Sequential parts stay gated: later parts depend on earlier corrected
  output (install depends on fix; benchmark depends on install; tag depends
  on all).

## Step 4 - Dispatch

- One `delegate_task` call with all parallel parts (disjoint) in the same
  turn.
- Keep payloads tight - delegate_task fails on oversized payloads with a JSON
  parse error; put detail in a plan FILE the agent reads, not in the goal.
- Pass the whole anchor in EVERY context; never rely on a sibling's output.

## Step 5 - Steer, don't re-dispatch

- Watch live transcripts only when a pattern suggests a loop (same command
  re-run with no intervening edit).
- If an agent sweeps multiple concepts per turn, queue ONE steer with the
  one-concept-per-turn rule and the numbered sequence.
- Do not respond to intermediate background notifications from children -
  only the batch completion matters.

## Step 6 - Verify on disk (self-reports are not verification)

After each batch:

1. Find the artifacts (`find`/`ls`/`wc`).
2. Spot-grep the contract marker (the corrected term, a `##` header, the
   expected version string).
3. Run the REAL gate (check/test/probe) yourself and record actual output.
4. Separate source changes from build output before blaming a dirty tree.
5. The auto-committer may sweep mid-flight - verify by content (grep/diff),
   never by `git status`.

## Step 7 - Fold the ledger (fan-in)

1. Read every part's ledger entries.
2. Resolve conflicts between competing solutions (keys collide → keep the
   stronger evidence, record the rejected one with `status: rejected`).
3. Choose the aggregate; record the decision AND the rejected options.
4. Approval-gated steps (tag, push, publish, remote side effects) never leave
   the parent - present the `Ready for approval` block.

## Step 8 - Promote durable findings (toroidal wrap)

A conclusion that matters beyond this task (a new contract, a reusable probe,
a pitfall, a fixed behavior) is written into the owning skill's references -
the ledger entry becomes a skill asset, wrapping back into the anchor of the
next research pass (the parts cycle of the torus).

## Failure responses

| Situation                          | Response                                     |
| ---------------------------------- | -------------------------------------------- |
| Agent sweeps concepts              | Steer with the numbered sequence             |
| Agent reports a blocker            | Trust the single blocker; unblock or stop    |
| Agent claims success               | Verify on disk; re-run the gate              |
| Two agents touch the same file     | Stop, reassign to one, sequence              |
| Write-guard blocks a child's write | Report; never retry via another path         |
| Payload rejected (JSON parse)      | Shrink; retry once; then change the shape    |
| Auto-committer absorbed edits      | Treat as the sweep's work; verify by content |
