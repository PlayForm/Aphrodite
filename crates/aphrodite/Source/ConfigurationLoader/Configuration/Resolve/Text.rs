//! String knobs.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve String: override → env → toml[section][key] → default.
	///
	/// A present value wins even when it is the empty string - "explicitly
	/// blank" is a real operator intent (it disables a prompt, an API URL, a
	/// callback), so it must not be treated as "absent". A TOML value that is
	/// not a string is skipped and the default applies.
	///
	/// `Option<String>` keeps the empty case distinct from the absent one for free:
	/// `Some("")` is a winner and only `None` continues the chain, so no emptiness
	/// test is needed anywhere.
	pub fn Text(&self, EnvironmentKey:&str, Section:&str, Key:&str, Fallback:&str) -> String {
		self.Overrides
			.get(EnvironmentKey)
			.cloned()
			.or_else(|| std::env::var(EnvironmentKey).ok())
			.unwrap_or_else(|| {
				self.Section(Section)
					.and_then(|Table| Table.get(Key))
					.and_then(|Value| Value.as_str())
					.map(|Value| Value.to_string())
					.unwrap_or_else(|| Fallback.to_string())
			})
	}
}
