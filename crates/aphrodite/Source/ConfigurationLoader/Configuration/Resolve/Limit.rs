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
	///
	/// An override that is present but unparseable answers `None` and does **not**
	/// fall through to the document, which is why the override is `Option<Option<_>>`
	/// and unwrapped rather than flattened: a flatten would turn the operator's typo
	/// into whatever the file happens to say.
	pub fn Limit(&self, EnvironmentKey:&str, Section:&str, Key:&str) -> Option<usize> {
		self.Overrides
			.get(EnvironmentKey)
			.map(|Raw| Raw.parse().ok())
			.unwrap_or_else(|| {
				self.Section(Section)
					.and_then(|Table| Table.get(Key))
					.and_then(|Value| Value.as_integer())
					.map(|Integer| Integer as usize)
			})
	}
}
