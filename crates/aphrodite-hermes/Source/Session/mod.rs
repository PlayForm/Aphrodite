//! Per-session state registry and its bounded-lock discipline.
//!
//! Hermes runs a SHARED-GATEWAY plugin model: ONE dylib instance serves ALL
//! sessions (parent + subagents) in the gateway process. A single
//! process-global `AphroditeState` (the pre-fix model) meant a heavy hook in
//! ONE session (subagent `pre_llm_call` -> `build_turn_context`) held the
//! only lock; when Hermes' `plugins.hook_callback_timeout` abandoned that
//! worker thread the mutex stayed locked forever, and every later hook in
//! EVERY session blocked on it (gateway log: "3 abandoned workers", permanent
//! skip until restart). State is therefore keyed by session_id: each session
//! owns an `Arc<Mutex<AphroditeState>>` cloned from a config-seeded template,
//! and hooks/tools lock ONLY their own session's mutex with a BOUNDED
//! try_lock (a wedged session degrades fail-open, never blocks the gateway).
//!
//! The registry mutex itself is held only for the HashMap get-or-create -
//! never while running hook work - so it cannot be wedged by an abandoned
//! worker; it blocks with poison recovery like any plain static mutex.
//!
//! Leaves: `Registry/` owns the map and its get-or-create, `Lock/` the
//! bounded acquisition every caller routes through, `With/` the two
//! run-under-lock entry points, `Broadcast/` the apply-to-every-session sweep
//! the config watcher needs, `Watcher/` the opt-in reload thread, and
//! `Guard/` the test serialiser that keeps the process-global state
//! deterministic across parallel test threads.
//!
//! Nothing here may widen a lock scope or make a caller wait unboundedly: the
//! bounded lock IS the fix for the gateway wedge (see `Maintain/SECURITY.md`).

pub mod Broadcast;
pub mod Guard;
pub mod Lock;
pub mod Registry;
pub mod Watcher;
pub mod With;

#[cfg(test)]
mod Test;
