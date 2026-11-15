//! `aphrodite_reclassify` - re-detect type and preview for stored markers.

use crate::Tool::Dispatch::ToolHandler;

/// The `aphrodite_reclassify` handler.
///
/// Targets are collected before any mutation, then updated one at a time, so a
/// concurrent record appended during the pass cannot be half-reclassified. The
/// stored content and its hash are never touched - only `ccr_type` and `preview`
/// on the catalogue row.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_reclassify", |args| {
		let only_hash = args.get("hash").and_then(|v| v.as_str());
		crate::with_shared(|state| {
			// Collect (hash, fresh content) for markers we will touch, then
			// recompute type + preview from the stored content.
			let targets:Vec<String> = state
				.recent_markers
				.iter()
				.filter(|mk| only_hash.is_none_or(|h| mk.hash == h))
				.map(|mk| mk.hash.clone())
				.collect();

			let mut updated = 0usize;
			for hash in targets {
				if let Some(content) = state.inline_store_get(&hash) {
					let detected = aphrodite::detect_type(&content);
					let preview = aphrodite::build_preview(&detected, &content);
					if let Some(mk) = state.recent_markers.iter_mut().find(|m| m.hash == hash) {
						mk.ccr_type = detected;
						mk.preview = preview;
						updated += 1;
					}
				}
			}
			serde_json::json!({"status": "ok", "reclassified": updated})
		})
	})
}
