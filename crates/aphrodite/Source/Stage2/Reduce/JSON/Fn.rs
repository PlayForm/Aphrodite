//! The JSON reducer.
//!
//! A JSON payload's information is almost entirely its keys and its shape, so the reduction keeps a
//! one-line structural header (key list for an object, item count plus the first item's schema for
//! an array) and then minifies the body. Pretty-printing is what the bytes were spent on; the
//! minified body still parses back to exactly the same value, so nothing is lost.
//!
//! Unparseable content returns `None` rather than a guess: a truncated or malformed payload reduced
//! to "some JSON-looking text" would be worse than leaving the original in place.

use crate::Stage2::Name::Fn::TypeName;

/// Keys the structural header names: ten.
const KeysShown:usize = 10;

/// Reduce JSON content: minify plus extract structural keys.
///
/// It is not a schema: the keys named are the ones the value has, not the ones it is required to
/// have, and a scalar root reports only its kind.
pub fn ReduceJSON(Content:&str) -> Option<String> {
	let Data:serde_json::Value = serde_json::from_str(Content).ok()?;

	let Structural = match &Data {
		serde_json::Value::Object(Object) => {
			let Keys:Vec<&str> = Object.keys().map(|Key| Key.as_str()).take(KeysShown).collect();

			format!("[json:{} keys: {}]", Object.len(), Keys.join(", "))
		},
		serde_json::Value::Array(Items) => {
			let Schema = Items
				.first()
				.and_then(|Item| Item.as_object())
				.map(|Object| {
					let Keys:Vec<&str> = Object.keys().map(|Key| Key.as_str()).take(KeysShown).collect();

					format!(" schema: {}", Keys.join(", "))
				})
				.unwrap_or_default();

			format!("[json_list:{} items{Schema}]", Items.len())
		},
		Other => format!("[json:{}]", TypeName(Other)),
	};

	// Minify. A serialisation failure is impossible for a value that just parsed, so the empty
	// string stands in rather than an `unwrap` on a preview path.
	let Minified = serde_json::to_string(&Data).unwrap_or_default();

	Some(format!("{Structural}\n{Minified}"))
}
