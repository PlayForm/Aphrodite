//! Runtime overrides - the highest-precedence source.

use super::super::Struct::Struct;

impl Struct {
	/// Set a runtime override (equivalent to Python's _settings store).
	///
	/// The key is the ENV VAR name, not the TOML key: an override is
	/// consulted first in every resolution chain, ahead of the environment
	/// itself, so setting one is the way to force a value that the process
	/// environment and the file would otherwise disagree about.
	pub fn Set(&mut self, Key:&str, Value:&str) { self.Overrides.insert(Key.to_string(), Value.to_string()); }
}
