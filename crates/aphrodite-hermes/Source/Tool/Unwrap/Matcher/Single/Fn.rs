//! Extract the first meaningful priority field of a single-key object.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::{Fragment, Matcher::Sole};

/// The field names that carry a payload in the remaining Hermes tool results
/// (`skill_view` ships `description`, `aphrodite_retrieve` ships `content` under
/// `found`, and so on). Order is the priority.
const PRIORITY:&[&str] = &["description", "summary", "result", "message", "preview", "found"];

/// Extract the first priority field that holds prose, typed `text`.
///
/// Only SOLE-key objects collapse here - `Sole` decides that, before any field is
/// read. A multi-key payload like `{"result": "ok", "data": {...}}` must preview
/// as itself, not as the fragment `ok`: the same bug class as the deleted
/// success-bool collapse.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	let Sole = Sole::Fn::Fn(Object)?;

	PRIORITY
		.iter()
		.filter_map(|Key| Sole.get(*Key).and_then(Value::as_str))
		.find(|Text| Fragment::Fn::Fn(Text))
		.map(|Text| (Text.to_string(), "text".to_string()))
}
