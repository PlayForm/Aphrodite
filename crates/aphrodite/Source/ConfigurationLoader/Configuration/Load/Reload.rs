//! Replacing a loaded configuration from disk.

use super::super::Struct::Struct;

impl Struct {
	/// Read the file again, replacing this value.
	///
	/// This replaces the whole value, including any runtime overrides - a
	/// refresh means "read the file again", not "merge the file into what I
	/// already had". `Apply::Reload::Fn` is the other half: it re-applies a
	/// document to a live session, which this never does.
	pub fn Refresh(&mut self) { *self = Self::Load(); }
}
