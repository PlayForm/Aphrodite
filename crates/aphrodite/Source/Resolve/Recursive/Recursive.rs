//! The recursive resolution walk.
//!
//! One hash, its nested markers, their nested markers - until the content runs out of
//! markers or the depth limit is reached. Three invariants hold the walk together, and each
//! has a recorded failure behind it: a cycle must terminate rather than recurse, a hash
//! already resolved must reuse the cached value instead of being skipped, and an unresolved
//! nested hash must leave its original marker text in place. It is not a rewriter - see the
//! note in `Walk/` about the write-back that was removed and must not return.
//!
//! This leaf is the dispatch and nothing else: `Prior/` answers what is already known about
//! the hash, and `Walk/` does the work when nothing is. The three guards used to be three
//! `match <bool>` blocks at the top of this function, so the fact that they formed an ordered
//! prelude with three distinct answers was something a reader had to reconstruct from the
//! statement sequence.
//!
//! The answer is `Result<Expansion, Halt>` rather than `Option<String>`: what came back
//! (loaded or fully expanded) and why nothing came back (absent, or abandoned for budget) are
//! two different questions, and a nested reference has to answer both.

use crate::Resolve::{Expansion, Halt, Traversal::Struct::Struct as Traversal};
use super::{Prior, Walk};

/// Recursively resolve a hash and all nested CCR markers.
///
/// Cycle-safe via the traversal's path, and the cache it owns is shared across the whole
/// tree: once a hash is loaded, nested references to it reuse the cached value. Both belong
/// to the traversal, so a guard from one retrieval can never reach the next.
///
/// `Err(Halt::Absent)` means the top-level hash is not in the store; `Err(Halt::Budget)`
/// means a bounded traversal abandoned its output. It is not idempotent by accident: the same
/// call twice must give the same answer, which is exactly why nothing here writes back.
///
/// The depth limit is `AphroditeState::RecursiveDepth` - config-driven via
/// `APHRODITE_RECURSIVE_DEPTH`, default 5 in `AphroditeState::default()` and the config
/// loader, so the pre-port hardcoded constant's value is byte-identical when unset. Every
/// entry point takes a traversal, which reads the cap from the state it holds, so there is no
/// state-less path needing its own fallback.
pub(crate) fn Fn(Traversal:&mut Traversal, Hash:&str, Depth:usize) -> Result<Expansion::Enum::Enum, Halt::Enum::Enum> {
	match Prior::Classify::Fn::Fn(Traversal, Hash, Depth) {
		// On the path already: the entry as loaded is the only answer that terminates.
		Prior::Enum::Enum::Revisit => Traversal.Cached(Hash).cloned().ok_or(Halt::Enum::Enum::Absent),
		// At the cap the entry is answered as stored, never expanded further.
		Prior::Enum::Enum::AtCap => {
			Traversal
				.Load(Hash)
				.map(Expansion::Enum::Enum::Loaded)
				.ok_or(Halt::Enum::Enum::Absent)
		},
		// Resolved earlier in this tree, off the current path: reuse it.
		Prior::Enum::Enum::Cached(Cached) => Ok(Cached),
		Prior::Enum::Enum::Fresh => Walk::Fn::Fn(Traversal, Hash, Depth),
	}
}
