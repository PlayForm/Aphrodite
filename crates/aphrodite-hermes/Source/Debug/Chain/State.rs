//! The learned session -> parent edges.
//!
//! A process-global map, because the parent link arrives exactly once per
//! session (through `pre_llm_call`) and later calls must be able to walk it
//! from any thread.

use std::{
	collections::HashMap,
	sync::{Mutex, OnceLock},
};

/// session_id -> parent_session_id map, learned from `pre_llm_call` kwargs.
/// Bounded: a session records its parent exactly once; the map only grows
/// with the number of distinct sessions, which is small per process.
pub(crate) static SESSION_PARENTS:OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
