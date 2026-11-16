//! A path together with the evidence that named it.

use crate::Hook::Path::Source::Enum::Enum as Source;

/// A resolved file path and how it was found.
///
/// The source travels with the string because a caller that stores the path
/// (`File::Recent`, the `WrotePath` column) has to be able to say whether it
/// is recording a declaration or a guess - and because a later change to the
/// heuristic must not silently reclassify paths that were always explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
	/// The path, as it will be recorded.
	pub Path:String,
	/// The evidence that produced it.
	pub Source:Source,
}
