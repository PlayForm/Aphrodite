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
//! bounded acquisition every caller routes through (and the named `Outcome` it
//! answers with), `With/` the two run-under-lock entry points, `Broadcast/` the
//! apply-to-every-session sweep the config watcher needs (with the `Report` that
//! says what the sweep actually reached), `Watcher/` the opt-in reload thread
//! and its lifecycle - readiness, stop flag, joined handle - and `Guard/` the
//! test serialiser that keeps the process-global state deterministic across
//! parallel test threads.
//!
//! Nothing here may widen a lock scope or make a caller wait unboundedly: the
//! bounded lock IS the fix for the gateway wedge (see `Maintain/SECURITY.md`).
//! The corollary runs the other way too - no process-global lock may be held
//! across a call that can block. The registry lock is therefore released before
//! any session's state mutex is taken, and the seed a new session is built from
//! (a TOML load, and the watcher's thread spawn) is built OUTSIDE it.

pub mod Broadcast;
pub mod Guard;
pub mod Lock;
pub mod Registry;
pub mod Watcher;
pub mod With;

#[cfg(test)]
mod Test;
