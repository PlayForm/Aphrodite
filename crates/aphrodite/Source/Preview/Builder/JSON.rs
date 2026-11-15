//! JSON preview arm.
//!
//! A JSON payload is best described by how many items it holds and what its
//! top-level keys are called. This arm renders that, matching the quality of
//! stage2's `ReduceJSON`. It is not a schema: nesting is reported as depth of
//! shape, never as types, and an unparseable payload falls back to a crude count
//! rather than failing.

use serde_json::Value as JSONValue;

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of JSON content.
///
/// Promises the item or object count plus the top-level keys. It is not a
/// validator: content that does not parse still gets a preview from a crude
/// brace/quote count, so a truncated payload is described approximately instead
/// of being reported as unreadable.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	match serde_json::from_str::<JSONValue>(Inp.raw) {
		Ok(JSONValue::Array(Arr)) => {
			let Keys = Arr
				.first()
				.and_then(|v| v.as_object())
				.map(|Obj| {
					let Ks:Vec<&str> = Obj.keys().map(|k| k.as_str()).take(8).collect();
					let More = if Ks.len() < Obj.len() {
						format!(" +{} more", Obj.len() - Ks.len())
					} else {
						String::new()
					};
					format!(" | keys: {}{}", Ks.join(", "), More)
				})
				.unwrap_or_default();
			format!("[json:{}items {}L{}]", Arr.len(), Inp.Total, Keys)
		},
		Ok(JSONValue::Object(Obj)) => {
			let Ks:Vec<&str> = Obj.keys().map(|k| k.as_str()).take(8).collect();
			let More = if Ks.len() < Obj.len() {
				format!(" +{} more", Obj.len() - Ks.len())
			} else {
				String::new()
			};
			format!("[json:{}keys {}L | {}{}]", Obj.len(), Inp.Total, Ks.join(", "), More)
		},
		_ => {
			// Fallback: crude `{"` count for unparseable content.
			let i = Inp.raw.matches("{\"").count();
			format!("[json:~{}items {}L]", i, Inp.Total)
		},
	}
}
