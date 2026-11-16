//! The `[templates]` section, deserialised into its typed model.

use super::super::Struct::Struct;

impl Struct {
	/// Parse the `[templates]` section (pre-Rust port) into typed structs.
	/// Absent section = None (hardcoded builders remain the default).
	///
	/// This is the one section that bypasses the per-knob chain: it is
	/// deserialised whole. A `[templates]` section that fails to deserialise
	/// therefore also yields `None` - the hardcoded builders stay in force,
	/// which is the same outcome as an absent section.
	pub fn Template(&self) -> Option<crate::State::Template::Struct::Struct> {
		self.Section("templates").and_then(|Table| Table.clone().try_into().ok())
	}
}
