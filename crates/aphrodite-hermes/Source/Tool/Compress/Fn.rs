//! The store-and-marker step itself.

use aphrodite::State::{Marker::Struct::Struct as MarkerEntry, Struct::AphroditeState};

use crate::Tool::Compress::Classified;

/// Store content in the session's inline store and record a catalog marker.
/// `Center` is an optional caller-supplied string that travels with the
/// marker (schema param `_ccr_center`) - previously documented but silently
/// dropped (F11); now threaded through to both the recorded catalog entry
/// and the rendered marker.
/// Returns `{hash, type, size, preview, marker}`.
///
/// It does not check for duplicates: compressing the same bytes twice records two
/// catalogue rows over one inline-store key, which is what makes the catalogue a
/// history of what the session did rather than a set of distinct blobs.
///
/// What the preview renders and what the type is filed under are
/// `Classified::Resolve`'s decision; this leaf hashes and stores the ORIGINAL
/// `Content`, never `Classified.Body`, so retrieval returns exactly what was
/// passed in.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, Hint:&str, Center:Option<&str>) -> serde_json::Value {
	let Resolved = Classified::Resolve::Fn::Fn(Content, Hint);

	let Hash = aphrodite::Hook::Hash::Fn::Fn(Content);
	let Preview = aphrodite::Preview::Builder::Entry::Fn(&Resolved.Kind, &Resolved.Body);
	let Marker =
		aphrodite::Marker::Format::Entry::Fn(&Hash, &Resolved.Kind, Content.len(), &Preview, None, None, Center);

	State.Put(Hash.clone(), Content.to_string());
	State.RecordMarker(MarkerEntry {
		hash:Hash.clone(),
		CCRType:Resolved.Kind.clone(),
		Size:Content.len(),
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:Center.map(|Center| Center.to_string()),
		meta:None,
	});

	serde_json::json!({
		"hash": Hash,
		"type": Resolved.Kind,
		"size": Content.len(),
		"preview": Preview,
		"marker": Marker,
	})
}
