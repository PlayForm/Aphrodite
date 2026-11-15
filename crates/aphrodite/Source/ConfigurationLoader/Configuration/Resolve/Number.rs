//! Integer knobs.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve u64: override → env → toml[section][key] → default.
	///
	/// A present value that does not parse as an integer (`"fast"` in the
	/// environment, a string in the file) silently falls back to the default
	/// rather than failing the load: a typo in one knob must not take the
	/// whole configuration down.
	///
	/// "Falls back to the default" is exact, and it is why each source maps through
	/// `parse().unwrap_or(default)` rather than through `and_then(parse().ok())`: the
	/// latter would let an unparseable override fall through to the environment, which
	/// is a different knob value than the one the operator set.
	pub fn GetU64(&self, EnvironmentKey:&str, Section:&str, Key:&str, default:u64) -> u64 {
		self.Overrides
			.get(EnvironmentKey)
			.map(|Raw| Raw.parse().unwrap_or(default))
			.or_else(|| std::env::var(EnvironmentKey).ok().map(|Raw| Raw.parse().unwrap_or(default)))
			.unwrap_or_else(|| {
				self.Section(Section)
					.and_then(|s| s.get(Key))
					.and_then(|v| v.as_integer())
					.map(|v| v as u64)
					.unwrap_or(default)
			})
	}
}
