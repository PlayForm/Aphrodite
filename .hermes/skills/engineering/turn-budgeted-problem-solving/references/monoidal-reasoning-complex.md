# The Monoidal / Toroidal Reasoning Complex - formal definition

Owner: `turn-budgeted-problem-solving` (the SKILL.md's §"The monoidal complex").
This reference is the formal core: the algebra, the ledger schema, the
toroidal extension, and a worked trace. Rules stay in SKILL.md; this file
carries the formalism and the worked instances.

## 1. The algebra (monoid)

A reasoning task is a **monoid** `(Ledger, ⊕, ε)`:

```
Ledger L   = ordered set of entries e = {key, conclusion, assumption, evidence}
             (key = unique string: "part.turn.concept"; conclusion = the
             result reached; assumption = what was presumed but not proven;
             evidence = the probe/command/read that grounds the conclusion)

Merge ⊕    : L × L → L  - append entries from the second ledger, dedupe by
             key (first wins on collision), keep both when keys differ.

Identity ε : the empty ledger.  ε ⊕ L = L ⊕ ε = L.

Associativity : (A ⊕ B) ⊕ C = A ⊕ (B ⊕ C).  Proven by construction: the
             merge only appends + dedupes by key, so grouping never changes
             the resulting key→entry map.
```

Each **turn** is a function on the monoid:

```
T : (Context, Ledger) → (Context', Ledger')
   Context' = Context enriched by this turn's reasoning/edits
   Ledger'   = Ledger ⊕ {e_turn}
```

Composition `T2 ∘ T1` is the sequential application of two turns. Because ⊕
is associative, `T3 ∘ (T2 ∘ T1) = (T3 ∘ T2) ∘ T1` - the accumulated ledger is
identical regardless of how turns are grouped. **This is the license for
parallelism**: parts may merge in any order and the research result is the
same. The orchestrator is the **fold**:

```
fold(ε, [T1 .. Tn]) = Tn( ... T2(T1(ε, anchor)) ... )  - final Context
```

## 2. The ledger schema (one entry)

```yaml
key: "part1.turn3.session-resolver"
conclusion: "session_id from parsed args, empty falls back to last_session(), then 'default'"
assumption: "transform_terminal_output never receives session_id (terminal_tool_result.py:144)"
evidence: "lib.rs:740; hook-contracts reference"
status: accepted | rejected | superseded-by:<key>
```

`status: rejected` entries are kept, not deleted - a rejected option carries
the trade-off reasoning the final pick needs. `superseded-by` links an
assumption to the turn that verified or refuted it.

## 3. The toroidal extension (two independent cycles)

A torus is `S¹ × S¹` - two independent loops. The reasoning complex becomes
**toroidal** when the ledger surface supports BOTH cycles:

```
Sequence cycle  : turn n → turn n+1, and turn N wraps back to the anchor of
                  the same part (re-reasoning a concept with new evidence is
                  a lap around this loop).
Parts cycle     : part A's findings wrap to inform part B, and the final
                  fold wraps back into the anchor of the NEXT research pass
                  (the durable skill reference is the wrapped-around surface).

A finding sits at (position along sequence, position along parts) - a point
on the torus. Routing a finding = choosing which later turns AND which later
parts it reaches. More configurations than a linear monoid: a conclusion can
simultaneously feed the next turn of its own part AND another part's first
turn AND the next research pass's anchor.
```

Practical consequences:

- The ledger is never "finished". Durable findings wrap into the anchor of
  the next research pass - the skill's `references/` is that wrapped surface
  (this file is a lap around the parts cycle).
- A turn can be re-entered (lap) when new evidence from another part refutes
  its assumption - the sequence cycle closes, not dead-ends.
- Assumptions are first-class: on the torus, an assumption is a point whose
  verification can come from ANY other turn, not just the next one.

## 4. Worked trace (hook-wedge decomposition, 2026-10-18)

Anchor (ledger seed - written by the orchestrator before dispatch):

```yaml
key: "anchor.root-cause"
conclusion: "single global Mutex<AphroditeState> poisons under Hermes shared-gateway multi-session"
assumption: "Hermes gateway runs ONE plugin instance for all sessions (parent + subagents)"
evidence: "gateway.error.log 06:29:40 pre_llm_call 50s timeout after subagent spawn; direct FFI probes 7ms"
status: accepted
```

Part 1 (lib.rs/tools.rs) ledger entries (abridged):

```yaml
key: "p1.t1.registry"
conclusion: "per-session registry Mutex<HashMap<String, Arc<Mutex<AphroditeState>>>>"
assumption: "hooks receive session_id except transform_terminal_output"
evidence: "lib.rs:582,699,740,769; hook-contracts"
status: accepted

key: "p1.t3.bounded-lock"
conclusion: "with_session uses try_lock ~50×2ms then None (fail-open); with_shared shim degrades on a throwaway clone"
assumption: "a wedged abandoned worker holds the lock; bounded wait must never reach Hermes' 50s timeout"
evidence: "plugins_dispatch.py _run_hook_callback_bounded; _HOOK_TIMEOUT_SUPPRESSION_SECONDS=60"
status: accepted

key: "p1.t4.fail-open"
conclusion: "pre_tool_call None → no directive (tool proceeds); transforms return original; pre_llm_call no context"
assumption: "fail-open is safe here because the policy hook's job is optimization (auto-background), not security"
evidence: "hook-contracts pre_tool_call safety boundary"
status: accepted
```

Part 2 (runtime, ~/.hermes) - parallel, disjoint:

```yaml
key: "p2.t1.reset"
conclusion: "runtime reset to shipped defaults, ccr.db wiped, stale proxy retired"
assumption: "APHRODITE_API_KEY stays env-exported; never written into toml"
evidence: "aphrodite.toml grep engine_threshold_pct=30; ps no binaries/aphrodite"
status: accepted
```

Fold (orchestrator): reads all entries, accepts the Part-1 set, keeps
rejected alternates (e.g. "single state + bounded lock only" was rejected -
cross-session isolation needs the registry), promotes the durable findings to
this skill's references (toroidal wrap → next research pass).

## 5. Verification of the algebra

- Associativity: fold the same two parts' ledgers in both orders; the
  merged entry set must be identical (test in the claim-to-test matrix).
- Identity: run a turn with an empty body; the ledger must be unchanged.
- Dispersion: grep the ledger for one entry per completed turn.
