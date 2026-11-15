//! The store-and-marker step itself.

use aphrodite::State::{Marker::Struct::MarkerEntry, Struct::AphroditeState};

use crate::Tool::Unwrap;

/// Store content in the session's inline store and record a catalog marker.
/// `center` is an optional caller-supplied string that travels with the
/// marker (schema param `_ccr_center`) - previously documented but silently
/// dropped (F11); now threaded through to both the recorded catalog entry
/// and the rendered marker.
/// Returns `{hash, type, size, preview, marker}`.
///
/// It does not check for duplicates: compressing the same bytes twice records two
/// catalogue rows over one inline-store key, which is what makes the catalogue a
/// history of what the session did rather than a set of distinct blobs.
pub(crate) fn Fn(state:&mut AphroditeState, content:&str, hint:&str, center:Option<&str>) -> serde_json::Value {
	// Hermes wraps tool results in JSON. Unwrap to find real content for
	// classification/preview (terminal output, diffs, etc.) instead of a
	// meaningless "[json:1items 1L]" - but always hash and store the
	// ORIGINAL `content`, never the extracted piece: retrieval must return
	// exactly what was passed in, including wrapper metadata like
	// `exit_code`/`error`, and legitimate caller JSON that merely matches a
	// wrapper shape (e.g. `{"content":"hi","id":42}`) must round-trip intact.
	let (classify_content, eff_type) = if let Some((c, t)) = Unwrap::Fn::Fn(content) {
		// ISSUE-11 WS1 (caller-hint-wins): the caller's explicit `type`
		// hint is intent, the unwrap is a guess - when a non-default hint
		// is present it must win. For JSON payloads the preview is built
		// from the FULL content: an unwrap fragment ("ok") produces a
		// wrong `[text:1L 2B | ok]` that counts the fragment, not the
		// stored payload, and drops the shape signal entirely.
		if !hint.is_empty() && hint != "text" {
			if content.trim_start().starts_with('{') {
				(content.to_string(), hint.to_string())
			} else {
				(c, hint.to_string())
			}
		} else {
			(c, t)
		}
	} else {
		let detected = aphrodite::Preview::Detect::Type::Fn(content);
		let ccr_type = if hint.is_empty() || hint == "text" { detected } else { hint.to_string() };
		(content.to_string(), ccr_type)
	};

	let hash = aphrodite::Hook::Hash::Fn::Fn(content);
	let preview = aphrodite::Preview::Builder::Entry::Fn(&eff_type, &classify_content);
	let marker =
		aphrodite::Marker::Format::Entry::CCRMarker(&hash, &eff_type, content.len(), &preview, None, None, center);

	state.InlineStorePut(hash.clone(), content.to_string());
	state.RecordMarker(MarkerEntry {
		hash:hash.clone(),
		CCRType:eff_type.clone(),
		Size:content.len(),
		Preview:preview.clone(),
		Turn:state.TurnCounter,
		Center:center.map(|c| c.to_string()),
		meta:None,
	});

	serde_json::json!({
		"hash": hash,
		"type": eff_type,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	})
}
