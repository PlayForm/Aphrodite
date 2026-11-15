//! The auto-expand size gate.

use std::collections::HashMap;

use crate::{Resolve::Recursive::Recursive, State::Struct::AphroditeState};

/// Resolve `HashValue` and every nested marker, but only when the result fits.
///
/// The contract for auto-expand: same `Visited` set, same `RecursiveDepth` limit, and the
/// resolved content returned ONLY when it fits within `Limit` bytes (pre-Rust `_resolve.py`
/// and `_core/config.py` AUTO_EXPAND_LIMIT). No marker is re-emitted: when the content fits,
/// the returned string IS the fully-resolved content.
///
/// Over-limit results return `None` so the caller keeps the original marker - the stored
/// bytes stay retrievable for a later, explicit resolve. A `None` is also returned when the
/// top-level hash cannot be resolved at all, deliberately indistinguishable from an
/// over-limit result: the caller's marker fallback is correct for both, and telling them
/// apart would only invite a caller to guess.
///
/// It is not a second resolver - every resolution rule lives in [`Recursive::Fn`], and a
/// divergence between the two would mean the same hash expands differently depending on who
/// asked.
pub fn Fn(
	State:&mut AphroditeState,
	HashValue:&str,
	Limit:usize,
	Resolved:&mut HashMap<String, String>,
	Visited:&mut Vec<String>,
) -> Option<String> {
	let Content = Recursive::Fn(State, HashValue, 0, Resolved, Visited)?;

	// The size test becomes the answer: the content is already in hand, so the gate costs a
	// comparison and no second copy.
	(Content.len() <= Limit).then_some(Content)
}
