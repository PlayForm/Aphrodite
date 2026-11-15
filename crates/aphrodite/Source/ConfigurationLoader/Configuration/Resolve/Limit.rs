//! Optional caps - knobs whose absence means "no limit".

use super::super::Struct::Struct;

impl Struct {
	/// Resolve an optional usize: override → toml[section][key]; absent = None.
	/// Used for caps that default to "no limit" (current behavior).
	///
	/// The environment is deliberately not consulted here: these knobs carry
	/// "absent" as a meaningful third state, and an env var cannot express it
	/// (an empty or unset variable is indistinguishable from a typo). Callers
	/// that want an env override read the variable themselves and only fall
	/// back to this chain.
	pub fn GetOptUsize(&self, EnvironmentKey:&str, Section:&str, Key:&str) -> Option<usize> {
		match self.Overrides.get(EnvironmentKey) {
			Some(v) => return v.parse().ok(),
			None => (),
		}

		self.Section(Section)
			.and_then(|s| s.get(Key))
			.and_then(|v| v.as_integer())
			.map(|v| v as usize)
	}
}
