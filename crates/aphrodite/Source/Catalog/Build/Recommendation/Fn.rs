//! The retrieve recommendations a TOC catalog carries.

use crate::{State::Struct::AphroditeState, StructExtract::Format::Floor::Fn as FloorBoundary};

/// The ten most recent entries as one-line retrieve suggestions, or `None` when there
/// is nothing to suggest.
///
/// `None` rather than an empty vector, because the caller adds the key only when it
/// has something to say: a `"recommendations": []` field is a claim that the catalog
/// looked and found nothing worth retrieving, while an absent field is the pre-Rust
/// shape a caller already handles. The two are different answers and the type says so.
pub(crate) fn Fn(State:&AphroditeState) -> Option<Vec<String>> {
	let Recommendations:Vec<String> = State
		.Marker
		.Recent
		.iter()
		.take(10)
		.map(|Marker| format!("{} ({}B) - {}", Marker.CCRType, Marker.Size, FloorBoundary(&Marker.Preview, 60)))
		.collect();

	(!Recommendations.is_empty()).then_some(Recommendations)
}
