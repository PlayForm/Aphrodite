//! Classify a hash against what the walk already knows.

use std::cmp::Ordering;

use crate::Resolve::Traversal::Struct::Struct as Traversal;
use super::super::Enum::Enum;

/// Decide how the walk should treat `HashValue`.
///
/// The order of the three questions is the contract and is preserved exactly: cycle, then
/// depth, then cache. It is not the obvious order - the cache is consulted only when the cap
/// has not been reached - and both halves have a recorded failure behind them (F9: a capped
/// hash answered "absent" when it fell back to the cache; F4: a nested hash already loaded was
/// skipped instead of substituted from the cache).
///
/// This function ASKS; it does not mark. Entering the path belongs to the walk that must also
/// leave it (`Recursive::Walk::Fn`), because a prelude that marked on the way past would mark
/// a hash whose expansion never started - which is exactly what the old never-popped set did
/// when a capped hash was recorded as visited for the rest of the tree.
pub(crate) fn Fn(Traversal:&Traversal, HashValue:&str, Depth:usize) -> Enum {
	if Traversal.OnPath(HashValue) {
		return Enum::Revisit;
	}

	// The cap is compared as an ordering rather than as a boolean so the "there is depth
	// left" case is the only one that continues.
	let Ordering::Less = Depth.cmp(&Traversal.Cap()) else {
		return Enum::AtCap;
	};

	Traversal
		.Cached(HashValue)
		.map_or(Enum::Fresh, |Cached| Enum::Cached(Cached.clone()))
}
