//! One metadata field.
//!
//! The producers used to append pre-formatted strings to a caller-owned
//! `&mut Vec<String>`, which meant the shape of a field (`key=value`), the
//! separator between fields, and the escaping of a pipe were all conventions
//! repeated in thirteen files and enforced nowhere. A field owns its two halves;
//! the renderer owns the separator; nothing owns a half-built string.
//!
//! The key is `&'static str` because it is part of the marker grammar the model
//! reads - a computed key would be a second vocabulary.

/// One metadata fact.
pub(crate) struct Struct {
	/// The field name, as it appears in the marker.
	Key:&'static str,
	/// The field value, already escaped for the marker's delimiters.
	Value:String,
}

impl Struct {
	/// Build a field.
	pub(crate) fn New(Key:&'static str, Value:impl Into<String>) -> Self { Struct { Key, Value:Value.into() } }

	/// Render the field for the marker's `;`-separated list.
	pub(crate) fn Render(&self) -> String { format!("{}={}", self.Key, self.Value) }
}
