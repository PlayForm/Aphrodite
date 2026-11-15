//! Extract a single-key success message.

use serde_json::{Map, Value};

/// Extract the success message, typed `text`, from a SINGLE-key object.
///
/// The single-key prerequisite is the whole point: a bare `{"success": true}` is
/// a USER payload, not a success envelope (collapsing it produced
/// `[text:1L 2B | ok]` and hid the entire result), and a multi-key payload like
/// `{"success": "ok", "data": [...]}` must preview as itself rather than as the
/// word `ok`.
///
/// It deliberately does not use `Fragment::Fn`: the original arm rejected only a
/// leading `{`, so a `[`-leading success string is still accepted here and the
/// envelope's classification is unchanged.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	Object
		.get("success")
		.and_then(Value::as_str)
		.filter(|_| Object.len() <= 1)
		.filter(|Message| !Message.is_empty())
		.filter(|Message| !Message.starts_with('{'))
		.map(|Message| (Message.to_string(), "text".to_string()))
}
