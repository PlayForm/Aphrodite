//! The opt-in intent-hint line.

use crate::State::Struct::AphroditeState;

/// The last three marker hashes, truncated to twelve characters each.
///
/// Opt-in through the prompt knob `[prompts] catalog_intent_hints`; default off
/// is the current behaviour. Pre-Rust port: `intent=read | recent CCRs: <hashes>`.
/// The hashes are re-reversed into chronological order so the newest sits last -
/// the same order the marker delta line uses - and truncated because twelve
/// characters is enough for the model to name one in a retrieval request while
/// keeping the line short.
pub(crate) fn Fn(State:&AphroditeState, Current:usize) -> Option<String> {
	(State.CatalogIntentHints && Current > 0).then(|| {
		let Recent:Vec<&crate::State::Marker::Struct::MarkerEntry> = State.RecentMarkers.iter().rev().take(3).collect();
		let Hashes:Vec<String> = Recent
			.iter()
			.rev()
			.map(|Marker| Marker.hash.chars().take(12).collect())
			.collect();

		format!("intent=read | recent CCRs: {}", Hashes.join(" "))
	})
}
