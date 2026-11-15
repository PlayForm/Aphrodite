//! The resolved configuration path and whether a file is there.

use super::super::Found;

/// Where the configuration is read from, and whether anything is there.
///
/// `Path` is always the path an operator should be told about, even when the
/// file does not exist: an explicit `APHRODITE_CONFIG_PATH` that is wrong is
/// reported as that path, never silently replaced by a different file. `Found`
/// is the only thing `Load` branches on.
pub(crate) struct Struct {
	/// The path the configuration would be read from.
	pub Path:String,
	/// Whether a file exists there.
	pub Found:Found::Enum::Enum,
}
