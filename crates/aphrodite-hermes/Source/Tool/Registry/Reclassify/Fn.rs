//! `aphrodite_reclassify` - re-detect type and preview for stored markers.

use crate::Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_reclassify` handler.
///
/// Targets are collected before any mutation, then updated one at a time, so a
/// concurrent record appended during the pass cannot be half-reclassified. The
/// stored content and its hash are never touched - only `ccr_type` and `preview`
/// on the catalogue row.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_reclassify", |Args| {
		let Only = Args.get("hash").and_then(|v| v.as_str());

		crate::Session::With::Fn::with_shared(|State| {
			// Collect the hashes to touch FIRST, then recompute type + preview
			// from each one's stored content: a row appended during the pass
			// cannot be half-reclassified.
			let Targets:Vec<String> = State
				.RecentMarkers
				.iter()
				.filter(|Marker| Only.is_none_or(|Hash| Marker.hash == Hash))
				.map(|Marker| Marker.hash.clone())
				.collect();

			let mut Updated = 0usize;

			Targets.into_iter().for_each(|Hash| {
				if let Some(Content) = State.InlineStoreGet(&Hash) {
					let Detected = aphrodite::Preview::Detect::Type::Fn(&Content);
					let Preview = aphrodite::Preview::Builder::Entry::Fn(&Detected, &Content);

					if let Some(Marker) = State.RecentMarkers.iter_mut().find(|Marker| Marker.hash == Hash) {
						Marker.CCRType = Detected;
						Marker.Preview = Preview;
						Updated += 1;
					}
				}
			});

			serde_json::json!({"status": "ok", "reclassified": Updated})
		})
	})
}
