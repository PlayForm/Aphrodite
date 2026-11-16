//! The single-marker replacement.

use serde_json::Value;

/// The CCR marker of an ordinary compression.
///
/// `None` when the payload did not compress or carries no marker: the caller then
/// renders `Passthrough`, which leaves the original output alone.
pub(crate) fn Fn(Result:&Value) -> Option<&str> {
	Result
		.get("compressed")
		.and_then(Value::as_bool)
		.unwrap_or(false)
		.then(|| Result.get("marker").and_then(Value::as_str))
		.flatten()
}
