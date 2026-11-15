//! Classify a hash against what the walk already knows.

use std::collections::HashMap;

use super::super::Enum::Enum;

/// Decide how the walk should treat `HashValue`, marking it as visited on the way.
///
/// The order of the three questions is the contract and is preserved exactly:
/// cycle, then depth, then cache. It is not the obvious order - marking happens
/// BEFORE the depth check, so a hash answered at the cap counts as visited for the
/// rest of the walk, and the cache is consulted only when the cap has not been
/// reached. Both are load-bearing and both have a recorded failure behind them
/// (F9: a capped hash answered `None` when it fell back to the cache; F4: a
/// nested hash already resolved was skipped instead of substituted from the
/// cache).
///
/// Marking is inside this function because it belongs to the same decision: a
/// revisit must NOT be re-marked, and splitting the two would let a caller mark a
/// hash the walk had already seen.
pub(crate) fn Fn(
	Visited:&mut Vec<String>,
	Resolved:&HashMap<String, String>,
	HashValue:&str,
	Depth:usize,
	Cap:usize,
) -> Enum {
	let Seen = Visited.iter().any(|Visited| Visited == HashValue);

	// A hash already on the path terminates the walk and is already marked.
	let Some(HashValue) = (!Seen).then_some(HashValue) else {
		return Enum::Revisit;
	};

	Visited.push(HashValue.to_string());

	// The cap is compared as an ordering rather than as a boolean so the "there is
	// depth left" case is the only one that continues.
	let std::cmp::Ordering::Less = Depth.cmp(&Cap) else {
		return Enum::AtCap;
	};

	match Resolved.get(HashValue) {
		Some(Cached) => Enum::Cached(Cached.clone()),
		None => Enum::Fresh,
	}
}
