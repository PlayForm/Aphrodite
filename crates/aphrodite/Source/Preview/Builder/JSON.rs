//! JSON preview arm.
//!
//! A JSON payload is best described by how many items it holds and what its top-level keys
//! are called. This arm renders that, matching the quality of stage2's JSON reducer. It is
//! not a schema: nesting is reported as depth of shape, never as types, and an unparseable
//! payload falls back to a crude count rather than failing.

use serde_json::Value as JSONValue;

use crate::Preview::Input::Struct::Struct as Input;
use super::More::Fn::Fn as More;

/// Keys the preview names: eight.
const KeysShown:usize = 8;

/// The names of the first `KeysShown` keys, plus the hidden-key suffix.
///
/// Both the array schema and the object header need exactly this, so it exists once: the
/// bounded key list and its ` +N more` can never disagree about how many were shown.
fn Keys(Object:&serde_json::Map<String, JSONValue>) -> String {
	let Names:Vec<&str> = Object.keys().map(|Key| Key.as_str()).take(KeysShown).collect();

	format!("{}{}", Names.join(", "), More(Object.len(), Names.len()))
}

/// Build a preview of JSON content.
///
/// Promises the item or object count plus the top-level keys. It is not a validator:
/// content that does not parse still gets a preview from a crude brace/quote count, so a
/// truncated payload is described approximately instead of being reported as unreadable.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	match serde_json::from_str::<JSONValue>(Inp.raw) {
		Ok(JSONValue::Array(Items)) => {
			let Schema = Items
				.first()
				.and_then(|Item| Item.as_object())
				.map(|Object| format!(" | keys: {}", Keys(Object)))
				.unwrap_or_default();

			format!("[json:{}items {}L{Schema}]", Items.len(), Inp.Total)
		},
		Ok(JSONValue::Object(Object)) => {
			format!("[json:{}keys {}L | {}]", Object.len(), Inp.Total, Keys(&Object))
		},
		// Fallback: crude `{"` count for unparseable content.
		_ => format!("[json:~{}items {}L]", Inp.raw.matches("{\"").count(), Inp.Total),
	}
}
