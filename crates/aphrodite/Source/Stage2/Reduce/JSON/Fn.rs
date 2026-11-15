//! The JSON reducer.
//!
//! A JSON payload's information is almost entirely its keys and its shape, so
//! the reduction keeps a one-line structural header (key list for an object,
//! item count plus the first item's schema for an array) and then minifies the
//! body. Pretty-printing is what the bytes were spent on; the minified body
//! still parses back to exactly the same value, so nothing is lost.
//!
//! Unparseable content returns `None` rather than a guess: a truncated or
//! malformed payload reduced to "some JSON-looking text" would be worse than
//! leaving the original in place.

/// Reduce JSON content: minify + extract structural keys.
pub fn ReduceJSON(Content:&str) -> Option<String> {
	let Data:serde_json::Value = match serde_json::from_str(Content) {
		Ok(v) => v,
		Err(_) => return None,
	};
	let Structural = match &Data {
		serde_json::Value::Object(Map) => {
			let Keys:Vec<&str> = Map.keys().map(|k| k.as_str()).take(10).collect();
			format!("[json:{} keys: {}]", Map.len(), Keys.join(", "))
		},
		serde_json::Value::Array(Arr) => {
			let mut s = format!("[json_list:{} items]", Arr.len());
			match Arr.first() {
				Some(First) => {
					match First.as_object() {
						Some(Obj) => {
							let Keys:Vec<&str> = Obj.keys().map(|k| k.as_str()).take(10).collect();
							s.push_str(&format!(" schema: {}", Keys.join(", ")));
						},
						None => {},
					}
				},
				None => {},
			}
			s
		},
		_ => format!("[json:{}]", crate::Stage2::Name::Fn::TypeName(&Data)),
	};
	// Minify
	let Minified = serde_json::to_string(&Data).unwrap_or_default();
	Some(format!("{}\n{}", Structural, Minified))
}
