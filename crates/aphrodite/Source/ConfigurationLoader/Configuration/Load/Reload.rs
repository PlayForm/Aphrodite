//! Replacing a loaded configuration from disk.

use super::super::Struct::Struct;

impl Struct {
	/// Reload from disk.
	///
	/// This replaces the whole value, including any runtime overrides - a
	/// reload means "read the file again", not "merge the file into what I
	/// already had".
	pub fn Reload(&mut self) { *self = Self::Load(); }
}
