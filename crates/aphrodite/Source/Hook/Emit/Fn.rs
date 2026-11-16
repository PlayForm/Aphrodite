//! Store content and emit its marker payload.

use crate::{
	Hook::{
		AutoExpand::Content::Fn::Fn as ResolveInline,
		Classification::{Fn::Fn as Classification, Hint::Enum::Enum as Hint},
	},
	Marker::Format::Entry::Fn as CCRMarker,
	State::{Marker::Struct::Struct as MarkerEntry, Struct::AphroditeState},
};

/// Store the content, record the marker, and build the payload the bridge hands
/// to the model - replacing the marker with the resolved content when
/// auto-expand applies.
///
/// ONE leaf for both transform paths, because the payload is one wire contract:
/// the tool-result and terminal paths differ only in the classification hint
/// they carry, and two copies of a contract this specific drift the first time
/// a field is added to one of them.
///
/// The payload shape is a wire contract: `status`/`compressed`/`hash`/`type`/
/// `size`/`preview`/`marker` on the default path, plus `auto_expanded`/
/// `expanded` only when auto-expand actually fired. Adding a field that is
/// always present would change what the bridge forwards on every turn.
///
/// The inline resolution is produced inside a closure so it runs only when
/// auto-expand is on. A failed resolve degrades to the original content, so the
/// payload still carries a valid marker either way.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	Hint:Hint,
	AutoExpand:bool,
) -> serde_json::Value {
	let Resolved = Classification(Content, Classify, Hint);

	let hash = headroom_core::ccr::compute_key(Content.as_bytes());
	State.Put(hash.clone(), Content.to_string());

	let Preview = crate::Preview::Builder::Entry::Fn(&Resolved.Type, Resolved.Content);
	let Marker = CCRMarker(&hash, &Resolved.Type, Content.len(), &Preview, None, None, None);

	State.RecordMarker(MarkerEntry {
		hash:hash.clone(),
		CCRType:Resolved.Type.clone(),
		Size:Content.len(),
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});

	let Expanded = AutoExpand.then(|| ResolveInline(State, &hash, Content));

	// DEV-ONLY debug metrics (crate::debug): count the compress decision.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Compressions);

	let mut Payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": Resolved.Type,
		"size": Content.len(),
		"preview": Preview,
		"marker": Marker,
	});

	let _ = Expanded.map(|Expanded| {
		Payload["auto_expanded"] = serde_json::Value::Bool(true);
		Payload["expanded"] = serde_json::Value::String(Expanded);
		crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::AutoExpansions);
	});

	Payload
}
