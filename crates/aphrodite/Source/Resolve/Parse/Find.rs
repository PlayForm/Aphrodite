//! The marker scanner.
//!
//! Resolved content is arbitrary text that may contain zero, one or many markers,
//! in any position, on any line. This module walks that text once and returns each
//! complete marker with the hash it carries, which is what the recursive resolver
//! substitutes. It is not a parser of the content itself - it knows nothing about
//! what surrounds a marker.
//!
//! The scanning rule matters more than it looks: after a match the cursor moves
//! past the closing delimiter, so a prefix inside an already-consumed marker is
//! never re-matched, and an unclosed prefix is stepped over instead of ending the
//! scan. Getting that wrong either panics on a malformed blob or silently drops
//! the markers that follow it - both of which turn a resolve into data loss.

use super::{Hash, Prefix, Suffix};

/// Find all CCR markers in content. Returns `(full_marker, hash)` pairs.
///
/// The full marker is returned alongside the hash because the resolver replaces
/// markers by literal text: the same hash may appear several times and only the
/// exact marker string can be substituted in place. It is not a de-duplicator -
/// a marker appearing twice is returned twice, and the caller replaces both.
///
/// A marker whose body carries no hash contributes nothing, and `extend` says so
/// without an arm: the hash extractor's `Option` is already the answer to "is this
/// a marker worth substituting", so there is no second test to keep in step.
pub(crate) fn Fn(Content:&str) -> Vec<(String, String)> {
	let mut Markers = Vec::new();
	let mut SearchFrom = 0;

	while let Some(Start) = Content[SearchFrom..].find(Prefix::Const) {
		let AbsStart = SearchFrom + Start;
		let AfterPrefix = AbsStart + Prefix::Const.len();

		// An unclosed prefix is stepped over, not treated as the end of the text:
		// guessing at it would resolve a marker the writer never finished emitting.
		let Some(End) = Content[AfterPrefix..].find(Suffix::Const) else {
			SearchFrom = AfterPrefix;

			continue;
		};

		let AbsEnd = AfterPrefix + End + Suffix::Const.len();
		let FullMarker = Content[AbsStart..AbsEnd].to_string();

		Markers.extend(Hash::Fn(&FullMarker).map(|hash| (FullMarker, hash)));

		SearchFrom = AbsEnd;
	}

	Markers
}
