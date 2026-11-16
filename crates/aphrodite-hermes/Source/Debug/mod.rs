//! Per-session debug toggle (flag files, NOT env).
//!
//! The gateway process env is fixed at launch - an env var exported
//! mid-session never reaches the running plugin, so the per-session switch
//! is a set of flag files under `~/.hermes/aphrodite/`:
//!
//!   debug.<root_session_id>   per-session-scoped (wins when present)
//!   debug                     global fallback (applies to every session)
//!
//! Content "on"/"1"/"debug" = enabled, anything else = off. Resolution walks
//! the session tree: Hermes threads `parent_session_id` through
//! `pre_llm_call` (turn_context.py:698-706) and every agent records its
//! parent, so `enabled_for(session)` walks session -> parent -> ... -> root
//! and checks the ROOT's scoped flag - meaning a toggle on the root session
//! applies to that session AND all its subagents/delegated tasks, but never
//! to other sessions. Both flag reads are mtime-cached (one stat() per call
//! when unchanged), mirroring the Python side's `_sync_debug`.
//!
//! When enabled, `debug_line` returns a `[aphrodite-debug ...]` line that
//! `replacement_from` prepends before the CCR marker, so the session sees
//! the debug output in the tool-result stream (before/after the markers).
//! Toggle from a session:
//!   echo on  > ~/.hermes/aphrodite/debug.<root-session-id>
//!   echo off > ~/.hermes/aphrodite/debug.<root-session-id>
//!
//! Logging is `tracing` (§4.5): `Init/` installs the one process-wide
//! `EnvFilter` subscriber and RECORDS who ended up owning the dispatcher
//! (`Ownership/` - `Installed` when this crate won, `Host` when the host
//! got there first), `Log/Bridge` emits each bridge event as a `tracing` event
//! and appends the same fields to the per-session debug file, and `Warn/` is the
//! warning that still lands when the dispatcher belongs to a host that may drop
//! it. `has_been_set()` is deliberately not consulted: it answers "is there a
//! subscriber", not "will this warning be seen". The `[aphrodite-debug ...]`
//! prefix line is NOT a log sink - it is a returned payload declared in the
//! `aphrodite_debug` tool schema (`Schema/Debug/`), which is why it stays a
//! string rather than becoming an event.
//!
//! Layout: `Runtime/` owns the home decision, `Flag/` owns the flag files, their
//! cache and the three-valued state they report, `Chain/` owns the
//! session-parent graph, `Session/` owns the last-seen-session state, `Log/`
//! owns the bridge into the core logger, `Init/` owns the subscriber and its
//! recorded ownership, `Warn/` the always-visible diagnostic. Each leaf owns one
//! item named by its kind; this facade declares children and keeps the old item
//! names alive for callers.

mod Chain;
pub(crate) mod Flag;
pub(crate) mod Init;
pub(crate) mod Log;
mod Runtime;
pub(crate) mod Session;
pub(crate) mod Warn;

#[cfg(test)]
mod Test;
