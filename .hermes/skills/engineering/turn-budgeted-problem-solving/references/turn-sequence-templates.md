# Turn-Sequence Templates

Owner: `turn-budgeted-problem-solving` (SKILL.md §"Good granular subjects").
Reusable one-concept-per-turn sequences for the common task shapes. Each
template is a strict order; a turn covers exactly one numbered concept, and a
blocked turn reports the single blocker and stops (never sweeps ahead).

## Template A - Debugging + fix + verify (unknown root cause)

The classic shape this skill exists for.

1. **Symptom classification** - the ONE observed symptom (not a suspected
   cause). Evidence: the exact failing message, the failing tool, the timing.
2. **Root-cause hypothesis** - enumerate 2-3 hypotheses; argue for the
   strongest; record the others in the ledger as rejected-with-reason.
3. **Smoking-gun probe** - the smallest discriminating test that separates
   the hypotheses (e.g. direct FFI call vs gateway context; fresh process vs
   long-lived process).
4. **Contract statement** - one sentence: "for input X, component Y must
   produce Z without invoking W."
5. **Fix design** - the minimal change satisfying the contract, with
   alternatives A/B/C and trade-offs.
6. **Implement** - the smallest edit (may be one line; reason as deeply as it
   needs).
7. **Verify** - the real gate (check/test/probe), ACTUAL output recorded.
8. **Update the contract** - if behavior differed from docs, patch the owning
   skill's contract + test matrix (never an unstructured note).

## Template B - Implementation with design reasoning

When the design is the hard part and the code is small.

1. **Requirement restatement** - the acceptance criterion in one sentence.
2. **Ownership map** - which files/crates exist today, which are new, who
   touches what (disjoint).
3. **API surface** - the public signatures that must not change (compat
   contract) vs what is internal.
4. **State model** - what state exists, where it lives, how it is keyed
   (the "registry vs single state" decision class).
5. **Concurrency model** - locks, bounded waits, fail-open/fail-closed per
   call site.
6. **Failure modes** - what can wedge, what degrades, what the user sees.
7. **Implement (core)** - the smallest coherent change.
8. **Implement (integration)** - the call sites that consume it.
9. **Gate battery** - check, test, clippy, the two probes.

## Template C - Research / literature / evidence gathering

1. **Question** - the exact question, not the topic.
2. **Evidence inventory** - what is already known (ledger seed), what is
   claim vs verified.
3. **Primary sources** - the authoritative paths (repo code, logs, probes),
   not prose.
4. **Discriminating experiment** - the probe that decides between readings.
5. **Conclusion + assumption split** - what is proven vs presumed; the
   assumption becomes the next turn's target.
6. **Dispersion** - which parts/research would benefit; write the finding to
   the ledger and the owning skill's reference.

## Template D - Migration / port / rewrite

1. **Baseline** - the pre-migration commit/tree, frozen and read-only.
2. **Inventory** - `git log -S` for the lost features; file counts.
3. **Semantic map** - old behavior → new behavior per feature (one concept
   per feature).
4. **Compatibility contract** - what consumers must keep seeing.
5. **Port (feature 1..N)** - one feature per turn.
6. **Verify per feature** - the feature's own probe, not a global build.
7. **Consistency pass** - unify format/framing across the ported features
   (real work, not append).

## Template E - Parallel multi-part dispatch

Used when a problem splits into disjoint slices (files, crates, filesystems).

1. **Anchor** - root cause, contract, ownership table, verification commands.
2. **Ledger seed** - assumptions already entered.
3. **Part decomposition** - one part per disjoint slice; each part gets its
   own template (A-D) as its turn sequence.
4. **Dispatch** - all parts in one turn (parallel).
5. **Steer-as-needed** - one-concept-per-turn enforcement.
6. **Fold** - read every part's ledger; aggregate; record rejected options.
7. **Promote** - durable findings into the owning skill's references.

## Choosing a template

- Unknown root cause → A.
- Known design, code ahead → B.
- Facts unknown, sources exist → C.
- Moving content between trees → D.
- Big, multi-slice, parallelizable → E (parts use A-D internally).

A template is a starting point, not a straitjacket - a turn subject may be
re-split when it still spans two concepts (SKILL.md §"Good granular
subjects").
