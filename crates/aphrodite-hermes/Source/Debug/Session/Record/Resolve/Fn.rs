//! The root a recorded session belongs to, learning the edge on the way.

use std::{collections::HashMap, sync::Mutex};

use crate::Debug::Chain::{self, State::SESSION_PARENTS};

/// Resolve the ROOT of `Session`'s tree, recording the `Session -> Parent` edge
/// first when there is one.
///
/// A root session (no parent, or a self-reference) IS its own root, so no edge is
/// inserted for it - the toggle is root-scoped and a self-edge would make the
/// walk stop on it anyway.
///
/// The edge and the root come from ONE acquisition, ONE scope. The walk goes
/// through `Chain::In::Fn::Fn`, which takes the map by reference and locks
/// nothing.
///
/// This must never become `Chain::Fn::Fn` (the locking wrapper) while the guard
/// is alive: that re-locks `SESSION_PARENTS`, `std::sync::Mutex` is not
/// reentrant, and the nested acquisition blocked the thread FOREVER while holding
/// the process-global lock - every later hook in the gateway process queued
/// behind it, the host abandoned the worker on its hook timeout, and the plugin
/// failed closed until a restart (the wedge signature, not a slow path).
pub(crate) fn Fn(Session:&str, Parent:&str) -> String {
	(Parent.is_empty() || Parent == Session)
		.then(|| Session.to_string())
		.or_else(|| {
			// The guard is a closure-local: it drops when this block yields, so
			// the caller's log line is emitted with no lock held.
			let mut Map = SESSION_PARENTS
				.get_or_init(|| Mutex::new(HashMap::new()))
				.lock()
				.unwrap_or_else(std::sync::PoisonError::into_inner);

			Map.entry(Session.to_string()).or_insert_with(|| Parent.to_string());

			Chain::In::Fn::Fn(&Map, Session).last().cloned()
		})
		.unwrap_or_else(|| Session.to_string())
}
