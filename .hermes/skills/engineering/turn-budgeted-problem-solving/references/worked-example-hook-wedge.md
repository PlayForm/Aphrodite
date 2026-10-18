# Worked Example - the Aphrodite Hook-Wedge (2026-10-18)

Owner: `turn-budgeted-problem-solving` (SKILL.md §"Worked example"). The
extended multi-part trace of the session that birthed this skill: how a
single-session diagnosis that was stuck for 40+ turns decomposed into a
parallel, one-concept-per-turn, ledger-accumulating research complex.

## The problem

Hermes Agent updated (06:04) to a shared-gateway plugin model. Immediately
after the first subagent spawn (06:28:45), every tool call failed with:

```
pre_tool_call plugin callback timed out or is still running
```

The gateway log showed the cascade:

```
06:29:40 Hook 'pre_llm_call' callback _dispatch timed out after 50s - skipping
06:29:41 Hook 'post_llm_call' callback _dispatch timed out after 50s - skipping
06:30:34 Hook 'pre_tool_call' callback _dispatch timed out after 50s - skipping
07:10:08 Hook 'pre_tool_call' callback _dispatch (hermes_plugins.aphrodite)
         skipped: 3 abandoned worker(s) still running - the plugin is hung
```

## The single-session iteration trap (what this skill prevents)

The parent session spent ~40 turns inline: disabled the plugin (worked),
re-enabled (wedged), restarted the gateway (worked briefly), probed the dylib
directly (7ms - fast!), read the Hermes dispatch code, checked config
timeouts, diffed the installed loader. Each turn advanced one fact but held
the WHOLE problem in one head - shallow reasoning spread across many
concepts, and the root cause only emerged after a dedicated multi-part fan-out.

## Root cause (the anchor's evidence)

- Hermes runs ONE plugin instance per gateway process for ALL sessions
  (commit `11ba615c3c`, Oct 16 - landed in the 06:04 update).
- The dylib kept ONE process-global `Mutex<AphroditeState>` (`shared()` /
  `with_shared`) - designed for one-session-per-process.
- Subagent spawn → two sessions → the subagent's `pre_llm_call` ran
  `build_turn_context` (context-engine archiving) under the single global
  lock; the call exceeded Hermes' `hook_callback_timeout` (50s).
- Hermes ABANDONED the worker - but the Rust mutex stayed locked by the
  abandoned thread forever.
- Every later hook blocked on `shared().lock()` → 50s timeouts → abandoned
  workers piled up → "3 abandoned workers - the plugin is hung" → permanent
  skip until gateway restart.

Decisive probe: direct FFI calls (`aphrodite_hermes_call_hook` with realistic
payloads, incl. 40-turn histories, on a worker thread) return in ~7ms - the
dylib work is fast; the hang is pure lock contention/poisoning.

## The decomposition (three parts, disjoint)

```
PROBLEM: hook-wedge fix + runtime reset + release unblock
├─ PART 1 (sa-0-92b77723)  lib.rs + tools.rs - the state fix
│    10 turns, one concept each (below)
├─ PART 2 (sa-0-f3ed94e5)  ~/.hermes only - runtime reset (parallel, disjoint)
│    Turn 1: shipped defaults → Turn 2: wipe state → Turn 3: retire proxy
└─ PART 3 (parent-held)    install → gateway restart → benchmark → release tag
```

## Part 1 turn sequence (one concept per turn)

| Turn | Concept                       | Deliverable                                                                      |
| ---- | ----------------------------- | -------------------------------------------------------------------------------- |
| 1    | Session-keyed registry design | `Mutex<HashMap<String, Arc<Mutex<AphroditeState>>>>`                             |
| 2    | Config template               | One `Config::load()` seed; each session clones it (test builds keep `default()`) |
| 3    | Bounded try_lock              | `with_session(sid, f) -> Option<T>`, ~50×2ms, fail-open on None                  |
| 4    | Fail-open contract per hook   | pre_tool_call → null; transforms → original; pre_llm_call → no context           |
| 5    | Session resolution chain      | `session_id` → `last_session()` → `"default"`                                    |
| 6    | Watcher all-sessions          | Config reload iterates every session, try_lock each                              |
| 7    | FFI compatibility             | Same extern C names, same JSON shapes, single-session identical                  |
| 8    | cargo check                   | Green                                                                            |
| 9    | cargo test --lib              | All pass (existing hook/tool tests)                                              |
| 10   | clippy -D warnings + probes   | Green + isolation probe + wedged-lock degrade probe                              |

Steered mid-flight: the agent was sweeping multiple concepts per turn; one
steer added the numbered sequence and the single-blocker rule.

## Ledger entries (abridged - full schema in monoidal-reasoning-complex.md)

```yaml
anchor.root-cause:        accepted - single global mutex poisons under shared gateway
p1.t1.registry:           accepted - per-session registry, disjoint locks
p1.t3.bounded-lock:       accepted - try_lock ~100ms then None; never block to 50s
p1.t4.fail-open:          accepted - pre_tool_call None = tool proceeds (optimization, not security)
p2.t1.reset:              accepted - shipped defaults, ccr.db wiped, proxy retired
anchor.rejected.alt:      rejected - "single state + bounded lock only": cross-session isolation still needed the registry
```

## The fold

- Part 1's ten turns compose into the fix (registry + bounded lock + fail-open).
- Part 2 ran in parallel on the disjoint runtime; its reset is now verified
  on disk (defaults present, state wiped, no proxy, plugin disabled).
- The orchestrator holds: install-from-Development (gated on Part 1 green),
  gateway restart, benchmark (harness already prepared - experiment record +
  runbook + 7 fixtures in `.hermes/tmp/benchmark-v1.7.0/`), and the release
  tag approval.
- Toroidal wrap: the durable findings (the shared-gateway poison mechanism,
  the per-session fix shape, the one-concept-per-turn discipline) are now
  THIS skill and its references - the next research pass starts from them,
  not from ε.

## What the session proved

1. A root cause that took 40+ inline turns emerged once decomposed: one part
   owned the dylib, one owned the runtime, the parent held the gates.
2. One-concept-per-turn allowed full depth per subject (each of the 10 turns
   reasoned deeply about its single concept instead of shallowly about all).
3. Parallel parts on disjoint slices are safe and fast.
4. The ledger made assumptions first-class: `transform_terminal_output`'s
   missing session_id was recorded as an assumption, verified against the
   hook-contracts reference, and drove the session-resolution design.
5. Durable findings wrapped into the skill (toroidal loop) - the next
   debugging session starts from this reference, not from scratch.
