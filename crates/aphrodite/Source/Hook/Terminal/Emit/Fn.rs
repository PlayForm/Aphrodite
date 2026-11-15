//! Store terminal output and emit its marker payload.

use crate::{
	Hook::{AutoExpand::Content, ToolResult::Classify},
	Marker::Format::Entry::CCRMarker,
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Store `Content`, record the marker, and build the payload the bridge hands
/// to the model - replacing the marker with the resolved content when
/// auto-expand applies.
///
/// The payload shape is a wire contract: `Status`/`Compressed`/`hash`/`type`/
/// `Size`/`Preview`/`Marker` on the default path, plus `auto_expanded`/
/// `Expanded` only when auto-expand actually fired. Adding a field that is
/// always present would change what the bridge forwards on every turn.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	DoAutoExpand:bool,
) -> serde_json::Value {
	let (TypeStr, ClassifyContent) = Classify::Fn::Fn(Content, Classify, true);

	let hash = headroom_core::ccr::compute_key(Content.as_bytes());
	State.InlineStorePut(hash.clone(), Content.to_string());

	let Preview = crate::Preview::Builder::Entry::Fn(&TypeStr, ClassifyContent);
	let Marker = CCRMarker(&hash, &TypeStr, Content.len(), &Preview, None, None, None);

	State.RecordMarker(MarkerEntry {
		hash:hash.clone(),
		CCRType:TypeStr.to_string(),
		Size:Content.len(),
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});

	// Auto-expand (pre-Rust port): store + record exactly as above, but
	// return the RESOLVED content inline instead of the marker so no
	// `<<<CCR:...>>>` reaches the model. Over the limit or disabled, the
	// payload below is byte-identical to the pre-auto-expand shape.
	let Expanded = match DoAutoExpand {
		true => Some(Content::Fn::Fn(State, &hash, Content)),
		false => None,
	};
	// DEV-ONLY debug metrics (crate::debug): count the compress decision.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Compressions);
	let mut Payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": TypeStr,
		"size": Content.len(),
		"preview": Preview,
		"marker": Marker,
	});
	match Expanded {
		Some(Expanded) => {
			Payload["auto_expanded"] = serde_json::Value::Bool(true);
			Payload["expanded"] = serde_json::Value::String(Expanded);
			crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::AutoExpansions);
		},
		None => {},
	}
	Payload
}
