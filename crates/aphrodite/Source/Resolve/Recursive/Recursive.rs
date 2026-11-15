//! The recursive resolution walk.
//!
//! One hash, its nested markers, their nested markers - until the content runs
//! out of markers or the depth limit is reached. Three invariants hold the walk
//! together, and each has a recorded failure behind it: a cycle must terminate
//! rather than recurse, a hash already resolved must reuse the cached value
//! instead of being skipped, and an unresolved nested hash must leave its original
//! marker text in place. It is not a rewriter - see the note in `Walk/` about the
//! write-back that was removed and must not return.
//!
//! This leaf is the dispatch and nothing else: `Prior/` answers what is already
//! known about the hash, and `Walk/` does the work when nothing is. The three
//! guards used to be three `match <bool>` blocks at the top of this function, so
//! the fact that they formed an ordered prelude with three distinct answers was
//! something a reader had to reconstruct from the statement sequence.

use std::collections::HashMap;

use crate::State::Struct::AphroditeState;
use super::{Prior, Walk};

/// Recursively resolve a hash and all nested CCR markers.
///
/// Cycle-safe via the `Visited` set. Uses the `Resolved` map as a persistent cache
/// across the entire resolution tree - once a hash is resolved, nested references
/// to it reuse the cached result.
///
/// Returns `Some(resolved_content)` on success, or `None` if the top-level hash
/// could not be found. It is not idempotent by accident: the same call twice must
/// give the same answer, which is exactly why nothing here writes back.
///
/// The depth limit is `AphroditeState::RecursiveDepth` - config-driven via
/// `APHRODITE_RECURSIVE_DEPTH`, default 5 in `AphroditeState::default()` and the
/// config loader, so the pre-port hardcoded constant's value is byte-identical
/// when unset. Every entry point in this module takes the state, so there is no
/// state-less path needing its own fallback.
pub fn Fn(
	State:&mut AphroditeState,
	HashValue:&str,
	Depth:usize,
	Resolved:&mut HashMap<String, String>,
	Visited:&mut Vec<String>,
) -> Option<String> {
	match Prior::Classify::Fn::Fn(Visited, Resolved, HashValue, Depth, State.RecursiveDepth) {
		// A revisit answers from the cache and may legitimately miss.
		Prior::Enum::Enum::Revisit => Resolved.get(HashValue).cloned(),
		// At the cap the entry is answered as stored, never expanded further.
		Prior::Enum::Enum::AtCap => crate::Resolve::One::Fn(State, HashValue),
		Prior::Enum::Enum::Cached(Cached) => Some(Cached),
		Prior::Enum::Enum::Fresh => Walk::Fn::Fn(State, HashValue, Depth, Resolved, Visited),
	}
}
