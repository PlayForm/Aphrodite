//! Extract a single-key success message.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::Matcher::Sole;

/// Extract the success message, typed `text`, from a SOLE-key object.
///
/// The sole-key prerequisite is the whole point: a bare `{"success": true}` is
/// a USER payload, not a success envelope (collapsing it produced
/// `[text:1L 2B | ok]` and hid the entire result), and a multi-key payload like
/// `{"success": "ok", "data": [...]}` must preview as itself rather than as the
/// word `ok`. `Sole` decides that, so the two `filter`s below are about the
/// MESSAGE - empty, or a re-embedded payload - and never about the object.
///
/// It deliberately does not use `Fragment::Fn`: the original arm rejected only a
/// leading `{`, so a `[`-leading success string is still accepted here and the
/// envelope's classification is unchanged.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	let Sole = Sole::Fn::Fn(Object)?;

	Sole.get("success")
		.and_then(Value::as_str)
		.filter(|Message| !Message.is_empty())
		.filter(|Message| !Message.starts_with('{'))
		.map(|Message| (Message.to_string(), "text".to_string()))
}
