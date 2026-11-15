//! The fresh half of the walk: resolve, scan, substitute.

use std::collections::HashMap;

use crate::{
	Resolve::{One, Parse::Find},
	State::Struct::AphroditeState,
};

/// Resolve `HashValue`, then resolve every marker its content contains.
///
/// Called only for a hash the prelude classified `Fresh`, so the three guards
/// (cycle, depth, cache) are already settled and are not repeated here.
///
/// Two invariants are preserved verbatim because each has a recorded failure
/// behind it. A nested hash already in `Resolved` still contributes a replacement
/// built from the CACHED value - skipping it left the literal marker in the output
/// (F4). And an unresolved nested hash leaves the ORIGINAL marker text untouched
/// - substituting a placeholder here got permanently baked into the stored entry,
///   so a merely evicted or late nested hash could never heal (F1).
///
/// The substitution is a fold, not a loop with an early exit: a document with no
/// markers folds to itself, so "nothing to replace" needs no arm of its own, and
/// the order of the replacements is the order the scanner found them.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	HashValue:&str,
	Depth:usize,
	Resolved:&mut HashMap<String, String>,
	Visited:&mut Vec<String>,
) -> Option<String> {
	let Content = One::Fn(State, HashValue)?;
	Resolved.insert(HashValue.to_string(), Content.clone());

	let NestedMarkers = Find::Fn(&Content);
	let Replacements:Vec<(String, Option<String>)> = NestedMarkers
		.iter()
		.map(|(Marker, NestedHash)| {
			let Nested = match Resolved.get(NestedHash) {
				Some(Cached) => Some(Cached.clone()),
				None => crate::Resolve::Recursive::Recursive::Fn(State, NestedHash, Depth + 1, Resolved, Visited),
			};

			(Marker.clone(), Nested)
		})
		.collect();

	Some(
		Replacements
			.iter()
			.filter_map(|(Marker, Replacement)| Replacement.as_ref().map(|Repl| (Marker, Repl)))
			.fold(Content, |Result, (Marker, Repl)| Result.replace(Marker.as_str(), Repl.as_str())),
	)
}
