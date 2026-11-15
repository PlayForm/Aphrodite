//! The bounded upward walk over the session-parent graph.
//!
//! The walk is split in two on purpose. [`In::Fn::Fn`] is PURE over a supplied
//! map; [`Fn`] is the thin locking wrapper for callers that hold no guard. A
//! caller that already holds the `SESSION_PARENTS` guard can only reach the
//! non-locking half, because the locking half would re-acquire the same
//! non-reentrant `std::sync::Mutex` and block that thread forever - while
//! holding the process-global lock every other session's hook needs.

use std::{collections::HashMap, sync::Mutex};

use crate::Debug::Chain::{In, State::SESSION_PARENTS};

/// Walk session -> parent -> ... -> root through the process-global graph.
///
/// The locking wrapper, for callers that hold no guard of their own. The guard
/// is held across [`In::Fn::Fn`] only, which locks nothing; callers that already
/// hold it must call [`In::Fn::Fn`] directly instead of nesting this call.
pub(crate) fn Fn(Session:&str) -> Vec<String> {
	let Map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);

	In::Fn::Fn(&Map, Session)
}
