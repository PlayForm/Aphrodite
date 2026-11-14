//! Hermes wrapper-envelope JSON object predicate.

use serde_json::Value as JsonValue;

/// True when a JSON object is a Hermes wrapper envelope whose raw-JSON preview
/// is intentional (the payload is inside the wrapper, not the key list).
pub(crate) fn is_envelope_json_object(obj:&serde_json::Map<String, JsonValue>) -> bool {
	const GUARD:&[&str] = &[
		"output",
		"exit_code",
		"diff",
		"error",
		"success",
		"total_count",
		"matches",
		"matches_text",
		"content",
		"total_lines",
		"result",
		"message",
		"found",
		"preview",
		"name",
		"description",
	];
	obj.keys().any(|k| GUARD.contains(&k.as_str()))
}
