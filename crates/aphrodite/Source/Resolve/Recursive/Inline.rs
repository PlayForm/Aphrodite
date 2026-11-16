//! The auto-expand size gate, enforced while the content is built.

use crate::{
	Resolve::{Expansion, Recursive::Recursive, Traversal::Struct::Struct as Traversal},
	State::Struct::AphroditeState,
};

/// Resolve `Hash` and every nested marker, but only when the result fits.
///
/// The contract for auto-expand: the same `RecursiveDepth` limit, and the resolved content
/// returned ONLY when it fits within `Limit` bytes (pre-Rust `_resolve.py` and
/// `_core/config.py` AUTO_EXPAND_LIMIT). No marker is re-emitted: when the content fits, the
/// returned string IS the fully-resolved content.
///
/// `Limit` governs CONSTRUCTION, not retention. The traversal carries the budget into every
/// level of the walk, so an expansion that cannot fit abandons its output as soon as the
/// bytes it would append would exceed the cap - it never builds the whole document and then
/// measures it. That is what keeps a nested expansion inside a 100 KiB budget from
/// materialising a multi-megabyte string first.
///
/// `None` means "the caller keeps its marker", and it covers every way that can happen: an
/// absent hash, a reference the store does not hold, and an abandoned over-budget expansion.
/// They are deliberately indistinguishable here - the caller's fallback is correct for all of
/// them, and telling them apart would only invite a caller to guess.
///
/// It is not a second resolver - every resolution rule lives in [`Recursive::Fn`], and a
/// divergence between the two would mean the same hash expands differently depending on who
/// asked.
pub fn Fn(State:&mut AphroditeState, Hash:&str, Limit:usize) -> Option<String> {
	Recursive::Fn(&mut Traversal::Bounded(State, Limit), Hash, 0)
		.ok()
		.map(Expansion::Enum::Enum::Owned)
}
