//! One listing entry.

use crate::Preview::Parse::Listing::Kind::Enum::Enum as Kind;

/// A listing line reduced to its name and whether it is a directory.
///
/// `Name` is the basename as printed (the last whitespace-separated column of an
/// `ls -l` line, or the whole token of a bare listing), kept whole because the
/// extension cut is a display decision. It is not a filesystem entry: nothing is
/// stat'ed, so a listing of files that no longer exist parses the same way.
pub(crate) struct Struct {
	pub Name:String,
	pub Kind:Kind,
}
