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
//! Layout: `Runtime/` owns the home decision, `Flag/` owns the flag files and
//! their cache, `Chain/` owns the session-parent graph, `Session/` owns the
//! last-seen-session state, `Log/` owns the bridge into the core logger. Each
//! leaf owns one item named by its kind; this facade declares children and
//! keeps the old item names alive for callers.

mod Chain;
pub(crate) mod Flag;
pub(crate) mod Log;
mod Runtime;
pub(crate) mod Session;
