//! Array knobs - the only family with no environment form.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve a TOML array of strings.
	///
	/// No env or override form exists: a list has no natural single-value
	/// spelling, so this reads the file directly. A non-array value and an
	/// absent key both yield an empty list, and non-string elements are
	/// dropped rather than failing the whole list.
	pub fn List(&self, Section:&str, Key:&str) -> Vec<String> {
		self.Section(Section)
			.and_then(|Table| Table.get(Key))
			.and_then(|Value| Value.as_array())
			.map(|Array| {
				Array
					.iter()
					.filter_map(|Element| Element.as_str().map(|Text| Text.to_string()))
					.collect()
			})
			.unwrap_or_default()
	}
}
