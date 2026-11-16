//! `aphrodite_reclassify` - re-detect type and preview for stored markers.

use crate::Tool::Dispatch::Fn::Handler;

/// The `aphrodite_reclassify` handler.
///
/// Targets are collected before any mutation, then updated one at a time, so a
/// concurrent record appended during the pass cannot be half-reclassified. The
/// stored content and its hash are never touched - only `ccr_type` and `preview`
/// on the catalogue row.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_reclassify", |Args| {
		let Only = Args.get("hash").and_then(|v| v.as_str());

		crate::Session::With::Fn::with_shared(|State| {
			// Collect the hashes to touch FIRST, then recompute type + preview
			// from each one's stored content: a row appended during the pass
			// cannot be half-reclassified.
			let Targets:Vec<String> = State
				.Marker
				.Recent
				.iter()
				.filter(|Marker| Only.is_none_or(|Hash| Marker.hash == Hash))
				.map(|Marker| Marker.hash.clone())
				.collect();

			let Updated = Targets.into_iter().filter_map(|Hash| Reclassify(State, &Hash)).count();

			serde_json::json!({"status": "ok", "reclassified": Updated})
		})
	})
}

/// Recompute one marker's type and preview from its stored content.
///
/// `None` means nothing was updated: the content is gone from the inline store, or
/// the row was removed while the pass ran.
fn Reclassify(State:&mut aphrodite::State::Struct::AphroditeState, Hash:&str) -> Option<()> {
	let Content = State.Store.Get(Hash)?;
	let Detected = aphrodite::Preview::Detect::Type::Fn(&Content);
	let Preview = aphrodite::Preview::Builder::Entry::Fn(&Detected, &Content);
	let Marker = State.Marker.Recent.iter_mut().find(|Marker| Marker.hash == Hash)?;

	Marker.CCRType = Detected;
	Marker.Preview = Preview;

	Some(())
}
