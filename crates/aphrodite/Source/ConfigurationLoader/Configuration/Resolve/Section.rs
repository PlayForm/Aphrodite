//! The one place a TOML section is looked up.

use super::super::Struct::Struct;

impl Struct {
	/// Get a TOML section as a table, or empty if missing.
	///
	/// A section that exists but is not a table (e.g. `compression = 3`) is
	/// treated exactly like an absent one: `None`, and every knob under it
	/// resolves to its default rather than erroring.
	pub(crate) fn Section(&self, name:&str) -> Option<&toml::Table> { self.raw.get(name).and_then(|v| v.as_table()) }
}
