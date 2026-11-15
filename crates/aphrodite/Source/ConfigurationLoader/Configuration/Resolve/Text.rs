//! String knobs.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve String: override → env → toml[section][key] → default.
	///
	/// A present value wins even when it is the empty string - "explicitly
	/// blank" is a real operator intent (it disables a prompt, an API URL, a
	/// callback), so it must not be treated as "absent". A TOML value that is
	/// not a string is skipped and the default applies.
	pub fn GetString(&self, EnvironmentKey:&str, Section:&str, Key:&str, default:&str) -> String {
		if let Some(v) = self.Overrides.get(EnvironmentKey) {
			return v.clone();
		}
		if let Ok(v) = std::env::var(EnvironmentKey) {
			return v;
		}
		self.Section(Section)
			.and_then(|s| s.get(Key))
			.and_then(|v| v.as_str())
			.map(|v| v.to_string())
			.unwrap_or_else(|| default.to_string())
	}
}
