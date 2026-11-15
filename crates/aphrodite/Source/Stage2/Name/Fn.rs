//! The JSON value-kind label.
//!
//! The JSON reducer's header names the kind of a scalar or array root
//! (`[json:string]`, `[json:null]`). serde_json has no `TypeName`, and the
//! `Debug` output of a `Value` prints the payload rather than the kind, so the
//! label is a match over the six variants - the match is exhaustive, so a
//! future serde_json variant cannot silently fall through to a wrong label.

/// The JSON value-kind label for a parsed value.
pub fn TypeName(v:&serde_json::Value) -> &'static str {
	match v {
		serde_json::Value::Null => "null",
		serde_json::Value::Bool(_) => "bool",
		serde_json::Value::Number(_) => "number",
		serde_json::Value::String(_) => "string",
		serde_json::Value::Array(_) => "array",
		serde_json::Value::Object(_) => "object",
	}
}
