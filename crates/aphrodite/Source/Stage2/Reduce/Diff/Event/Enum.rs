//! One classified diff line.

/// A diff line, classified by the only distinction the file summariser needs.
///
/// `File` opens a file's section, `Hunk` is one `@@` header, and `Other` is everything else. The
/// three cases replace the two independent `starts_with` tests the reducer used to run per line,
/// so a line is classified once and the state machine `match`es a value.
///
/// It is not a hunk parser: the hunk's coordinates and its body are never read.
pub(crate) enum Enum<'a> {
	/// A `diff --git ...` header - the boundary between two files.
	File { Header:&'a str },
	/// An `@@ ...` hunk header.
	Hunk,
	/// Any other line.
	Other,
}
