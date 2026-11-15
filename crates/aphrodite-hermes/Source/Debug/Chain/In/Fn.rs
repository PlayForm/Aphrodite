//! The pure upward walk over the session-parent graph.

use std::collections::HashMap;

/// Walk `Session` -> parent -> ... -> root over `Map`, leaf first.
///
/// Pure: no lock, no global, no allocation beyond the chain itself. That is what
/// makes it callable with the `SESSION_PARENTS` guard ALREADY HELD - and it is
/// the only walk such a caller may use, since `MutexGuard` lives to the end of
/// its scope and a second acquisition of the same mutex on the same thread is a
/// permanent hang, not an error.
///
/// `successors` IS the depth cap: the chain is the seed plus 16 hops, and the
/// walk ends as soon as a session has no parent or names itself as one. It is
/// deliberately not a visited set - a cycle deeper than one edge still costs at
/// most 16 hops, which is cheaper than an allocation per hop over a
/// process-global map.
///
/// The chain is leaf-first so callers can iterate `.rev()` for root-first, the
/// order flag resolution needs: the ROOT's scoped flag governs the whole tree.
pub(crate) fn Fn(Map:&HashMap<String, String>, Session:&str) -> Vec<String> {
	std::iter::successors(Some(Session.to_owned()), |Current| {
		Map.get(Current).filter(|Parent| Parent.as_str() != Current.as_str()).cloned()
	})
	.take(17)
	.collect()
}
