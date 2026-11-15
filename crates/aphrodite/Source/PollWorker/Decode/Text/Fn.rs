//! One optional string field out of the JSON envelope.
//!
//! Absence is not an error: an unwrapped poll result has no envelope at all, and
//! a wrapped one omits the fields it has nothing to say about.

use serde_json::Value;

/// The string at `Field`, when the envelope carries one.
pub(crate) fn Fn(Parsed:&Option<Value>, Field:&str) -> Option<String> {
	Parsed.as_ref()?.get(Field)?.as_str().map(str::to_string)
}
