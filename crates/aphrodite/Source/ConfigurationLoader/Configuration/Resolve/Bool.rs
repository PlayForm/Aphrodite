//! Boolean knobs.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve bool: override → env → toml[section][key] → default.
	///
	/// A present value never falls through: `false` from an override beats
	/// `true` in the file, which is the whole point of an override. A TOML
	/// value that is not a boolean (`enabled = "yes"`) is not a parse error -
	/// it is skipped and the default applies.
	///
	/// Fix 21 (inspection): values are matched case-insensitively -
	/// `TRUE`/`True`/`tRuE` previously resolved to false because the exact
	/// lowercase comparison missed them. Only `true`/`1` (after ASCII-lowercasing)
	/// still count as true; `YES`/`on` stay false.
	///
	/// The chain is `or_else`, so the environment is read only when no override was
	/// set and the document is read only when neither was present. The earlier shape
	/// was three `match` statements with an early `return` and an empty `None` arm,
	/// which made the precedence a property of the statement order rather than of the
	/// value.
	pub fn GetBool(&self, EnvironmentKey:&str, Section:&str, Key:&str, default:bool) -> bool {
		let AsBool = |Raw:&str| {
			let Lowered = Raw.to_ascii_lowercase();

			Lowered == "true" || Lowered == "1"
		};

		self.Overrides
			.get(EnvironmentKey)
			.map(|Raw| AsBool(Raw))
			.or_else(|| std::env::var(EnvironmentKey).ok().map(|Raw| AsBool(&Raw)))
			.unwrap_or_else(|| {
				self.Section(Section)
					.and_then(|s| s.get(Key))
					.and_then(|v| v.as_bool())
					.unwrap_or(default)
			})
	}
}
