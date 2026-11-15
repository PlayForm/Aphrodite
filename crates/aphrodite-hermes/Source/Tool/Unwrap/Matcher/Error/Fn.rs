//! Extract an error string as the payload.

use serde_json::{Map, Value};

use crate::Tool::Unwrap::Fragment;

/// Extract the error text, typed `text`.
///
/// It reads the message as prose only: an `error` whose value is itself a JSON
/// object or array restates a structure the caller already holds, so it falls
/// through to the next matcher instead of previewing as a brace.
///
/// A dedicated `Found ... matches` arm used to sit in front of this one, because a
/// "not found" search result arrives as an `error` string. It was removed as
/// dead code: a message that starts with `Found` is by definition non-empty and
/// does not start with `{` or `[`, so this predicate already accepted it and
/// produced the identical `(text, "text")` pair.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)> {
	Object
		.get("error")
		.and_then(Value::as_str)
		.filter(|Message| Fragment::Fn::Fn(Message))
		.map(|Message| (Message.to_string(), "text".to_string()))
}
