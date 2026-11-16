//! The opt-in intent-hint line.

use std::collections::VecDeque;

use crate::State::Marker::Struct::Struct as Entry;

/// The last three marker hashes, truncated to twelve characters each.
///
/// Takes the knob's value and the ring it reads rather than the session, so this
/// section cannot reach a field it does not report on.
///
/// Opt-in through the prompt knob `[prompts] catalog_intent_hints`; default off
/// is the current behaviour. Pre-Rust port: `intent=read | recent CCRs: <hashes>`.
/// The hashes are re-reversed into chronological order so the newest sits last -
/// the same order the marker delta line uses - and truncated because twelve
/// characters is enough for the model to name one in a retrieval request while
/// keeping the line short.
pub(crate) fn Fn(Enabled:bool, Markers:&VecDeque<Entry>, Current:usize) -> Option<String> {
	(Enabled && Current > 0).then(|| {
		// `rev().take(3).rev()` is the last three markers in insertion order: the
		// ring is oldest-first, so the newest three are at the back and the double
		// reversal restores the order the marker delta line uses - without the
		// intermediate `Vec<&Entry>` a collect-then-reverse would need.
		let Hashes:Vec<String> = Markers
			.iter()
			.rev()
			.take(3)
			.rev()
			.map(|Marker| Marker.hash.chars().take(12).collect())
			.collect();

		format!("intent=read | recent CCRs: {}", Hashes.join(" "))
	})
}
