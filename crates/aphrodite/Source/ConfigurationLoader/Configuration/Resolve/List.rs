//! Array knobs - the only family with no environment form.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve a TOML array of strings.
	///
	/// No env or override form exists: a list has no natural single-value
	/// spelling, so this reads the file directly. A non-array value and an
	/// absent key both yield an empty list, and non-string elements are
	/// dropped rather than failing the whole list.
	pub fn GetStringList(&self, Section:&str, Key:&str) -> Vec<String> {
		self.Section(Section)
			.and_then(|s| s.get(Key))
			.and_then(|v| v.as_array())
			.map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default()
	}
}
