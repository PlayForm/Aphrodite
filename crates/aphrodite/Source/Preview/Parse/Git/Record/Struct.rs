//! One recognised `git status` line.

/// A status line reduced to the two facts the preview uses.
///
/// `Code` is the most significant column of the porcelain pair (the first non-space
/// character), and `Path` is the path as git printed it with a rename's `old -> new`
/// already collapsed to `new`. `Path` may be empty - a status line with no path after
/// the code is still a recognised status line, so it still counts in the tally and
/// only fails to become one of the shown paths. It is not a parser of the index: the
/// code is carried verbatim and the renderer decides what it means.
pub(crate) struct Struct {
	pub Code:char,
	pub Path:String,
}
