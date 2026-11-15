//! The most recently seen session and the parent links learned from it.
//!
//! Hermes threads `session_id` (and, on `pre_llm_call`, `parent_session_id`)
//! through the hooks, but not through every hook and never through a tool call.
//! This module is the memory that bridges those gaps: the last id seen, the
//! graph edge it came with, and the persisted copy that survives a process
//! restart (a fresh shim exec starts with empty Rust statics).

pub(crate) mod Last;
pub(crate) mod Record;

#[cfg(debug_assertions)]
pub(crate) mod Root;
#[cfg(debug_assertions)]
pub(crate) mod Set;

mod State;
