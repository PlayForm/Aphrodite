//! The recursive resolution walk.
//!
//! One hash, its nested markers, their nested markers - until the content runs
//! out of markers or the depth limit is reached. Three invariants hold the walk
//! together, and each has a recorded failure behind it: a cycle must terminate
//! rather than recurse, a hash already resolved must reuse the cached value
//! instead of being skipped, and an unresolved nested hash must leave its original
//! marker text in place. It is not a rewriter - see the note at the end of the
//! walk about the write-back that was removed and must not return.

use std::collections::HashMap;

use crate::{
	Resolve::{One, Parse::Find},
	State::Struct::AphroditeState,
};

// Depth limit is `AphroditeState::recursive_depth` - config-driven via
// `APHRODITE_RECURSIVE_DEPTH`, default 5 in `AphroditeState::default()` and
// the config loader, so the pre-port hardcoded constant's value is
// byte-identical when unset. Every entry point in this module takes the
// state, so there is no state-less path needing its own fallback.

/// Recursively resolve a hash and all nested CCR markers.
///
/// Cycle-safe via the `Visited` set. Uses the `Resolved` map as a persistent cache
/// across the entire resolution tree - once a hash is resolved, nested references
/// to it reuse the cached result.
///
/// Returns `Some(resolved_content)` on success, or `None` if the top-level hash
/// could not be found. It is not idempotent by accident: the same call twice must
/// give the same answer, which is exactly why nothing here writes back.
pub fn Fn(
	State:&mut AphroditeState,
	HashValue:&str,
	Depth:usize,
	Resolved:&mut HashMap<String, String>,
	Visited:&mut Vec<String>,
) -> Option<String> {
	// Cycle detection
	match Visited.iter().any(|v| v == HashValue) {
		true => return Resolved.get(HashValue).cloned(),
		false => (),
	}

	Visited.push(HashValue.to_string());

	// Depth limit (config-driven via `state.recursive_depth`,
	// `APHRODITE_RECURSIVE_DEPTH`, default 5 - byte-identical to the
	// pre-port hardcoded constant when unset): return the raw
	// (un-further-expanded) content for this hash rather than falling
	// back to the resolved cache (F9) - a hash that legitimately exists
	// but simply hasn't been visited yet at this depth would otherwise
	// incorrectly resolve to `None`.
	match Depth >= State.RecursiveDepth {
		true => return One::Fn(State, HashValue),
		false => (),
	}

	match Resolved.get(HashValue) {
		Some(Cached) => return Some(Cached.clone()),
		None => (),
	}

	// Resolve the top-level hash
	let Content = One::Fn(State, HashValue)?;
	Resolved.insert(HashValue.to_string(), Content.clone());

	// Find nested CCR markers
	let NestedMarkers = Find::Fn(&Content);
	match NestedMarkers.is_empty() {
		true => return Some(Content),
		false => (),
	}

	// Resolve nested hashes and build replacements. A nested hash already
	// in `Resolved` still needs a replacement pushed using the CACHED value
	// (F4) - previously it was silently skipped, leaving the literal marker
	// in the output (which F1's write-back would then have persisted).
	let Replacements:Vec<(String, Option<String>)> = NestedMarkers
		.iter()
		.map(|(Marker, NestedHash)| {
			let NestedContent = match Resolved.get(NestedHash) {
				Some(Cached) => Some(Cached.clone()),
				None => Fn(State, NestedHash, Depth + 1, Resolved, Visited),
			};

			(Marker.clone(), NestedContent)
		})
		.collect();

	// Apply replacements. An unresolved nested hash leaves the ORIGINAL
	// marker text untouched (F1) - substituting `[CCR_UNRESOLVED:...]` here
	// used to get permanently baked into the store below, so a merely
	// evicted/late nested entry could never heal even after it reappeared.
	let mut Result = Content;
	Replacements
		.iter()
		.filter_map(|(Marker, Replacement)| Replacement.as_ref().map(|Repl| (Marker, Repl)))
		.for_each(|(Marker, Repl)| {
			Result = Result.replace(Marker.as_str(), Repl.as_str());
		});

	// NOTE (F1): the expanded result is intentionally NOT written back over
	// `HashValue` in the store. Doing so previously broke the content-address
	// invariant (the stored bytes no longer matched the hash that names
	// them) and destroyed the pristine original irrecoverably - including
	// any literal `<<<CCR:...>>>`-shaped text the original content merely
	// *contained* (this crate's own test fixtures, docs, echoed tool
	// output), which got silently corrupted on every retrieval.
	Some(Result)
}
