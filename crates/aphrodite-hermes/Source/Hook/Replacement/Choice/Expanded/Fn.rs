//! The auto-expanded replacement.

use serde_json::Value;

/// The resolved text of an auto-expanded compression.
///
/// Auto-expand stores the content and resolves it inline, carrying the full text
/// back in `expanded` - returning that is what keeps a `<<<CCR:...>>>` marker out
/// of the model's view. A payload that flags `auto_expanded` WITHOUT a usable
/// `expanded` string returns `None`, so it falls through to the marker path and
/// degrades rather than replacing the output with nothing.
pub(crate) fn Fn(Result:&Value) -> Option<&str> {
	Result
		.get("auto_expanded")
		.and_then(Value::as_bool)
		.unwrap_or(false)
		.then(|| Result.get("expanded").and_then(Value::as_str))
		.flatten()
}
