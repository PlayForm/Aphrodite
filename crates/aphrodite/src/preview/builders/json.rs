//! JSON preview arm.

use serde_json::Value as JsonValue;

use crate::preview::input::Input;

/// JSON preview: parse content and show item/object count with top-level keys,
/// matching the quality of stage2's `reduce_json`. Falls back to a crude `{"`
/// count when parsing fails (e.g. truncated or malformed JSON).
pub(crate) fn build_json_preview(inp: &Input<'_>) -> String {
	match serde_json::from_str::<JsonValue>(inp.raw) {
		Ok(JsonValue::Array(arr)) => {
			let keys = arr
				.first()
				.and_then(|v| v.as_object())
				.map(|obj| {
					let ks: Vec<&str> = obj.keys().map(|k| k.as_str()).take(8).collect();
					let more = if ks.len() < obj.len() {
						format!(" +{} more", obj.len() - ks.len())
					} else {
						String::new()
					};
					format!(" | keys: {}{}", ks.join(", "), more)
				})
				.unwrap_or_default();
			format!("[json:{}items {}L{}]", arr.len(), inp.total, keys)
		},
		Ok(JsonValue::Object(obj)) => {
			let ks: Vec<&str> = obj.keys().map(|k| k.as_str()).take(8).collect();
			let more = if ks.len() < obj.len() {
				format!(" +{} more", obj.len() - ks.len())
			} else {
				String::new()
			};
			format!("[json:{}keys {}L | {}{}]", obj.len(), inp.total, ks.join(", "), more)
		},
		_ => {
			// Fallback: crude `{"` count for unparseable content.
			let i = inp.raw.matches("{\"").count();
			format!("[json:~{}items {}L]", i, inp.total)
		},
	}
}
