//! The bounded upward walk over the session-parent graph.
//!
//! Returns the chain leaf-first so callers can iterate `.rev()` to get
//! root-first - the order flag resolution needs, since the ROOT's scoped flag
//! governs the whole tree.

use std::{collections::HashMap, sync::Mutex};

use crate::Debug::Chain::State::SESSION_PARENTS;

/// Walk session -> parent -> ... -> root, root first.
///
/// Depth-capped at 16 so a malicious/cyclic chain cannot loop forever.
///
/// The chain is not deduplicated beyond the self-reference guard: a cycle
/// deeper than one edge still costs up to 16 hops, which is why the cap exists
/// rather than a visited set (the map is process-global and unbounded, so an
/// allocation per hop would be the bigger cost).
pub(crate) fn Fn(session:&str) -> Vec<String> {
	let map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	let mut chain = vec![session.to_string()];
	let mut cur = session.to_string();

	// `try_for_each` IS the depth cap: the first hop with no parent (or a
	// self-reference) ends the walk, and 16 hops is the ceiling.
	let _ = (0..16).try_for_each(|_| {
		match map.get(&cur) {
			Some(p) if p != &cur => {
				chain.push(p.clone());
				cur = p.clone();

				Ok::<(), ()>(())
			},
			_ => Err(()),
		}
	});

	chain
}
