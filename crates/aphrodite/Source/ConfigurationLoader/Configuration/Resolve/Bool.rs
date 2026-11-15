//! Boolean knobs.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve bool: override → env → toml[section][key] → default.
	///
	/// A present value never falls through: `false` from an override beats
	/// `true` in the file, which is the whole point of an override. A TOML
	/// value that is not a boolean (`enabled = "yes"`) is not a parse error -
	/// it is skipped and the default applies.
	pub fn GetBool(&self, EnvironmentKey:&str, Section:&str, Key:&str, default:bool) -> bool {
		// Fix 21 (inspection): values are matched case-insensitively -
		// `TRUE`/`True`/`tRuE` previously resolved to false because the
		// exact lowercase comparison missed them. Only `true`/`1` (after
		// ASCII-lowercasing) still count as true; `YES`/`on` stay false.
		match self.Overrides.get(EnvironmentKey) {
			Some(v) => {
				let v = v.to_ascii_lowercase();
				return v == "true" || v == "1";
			},
			None => (),
		}

		match std::env::var(EnvironmentKey) {
			Ok(v) => {
				let v = v.to_ascii_lowercase();
				return v == "true" || v == "1";
			},
			Err(_) => (),
		}

		self.Section(Section)
			.and_then(|s| s.get(Key))
			.and_then(|v| v.as_bool())
			.unwrap_or(default)
	}
}
