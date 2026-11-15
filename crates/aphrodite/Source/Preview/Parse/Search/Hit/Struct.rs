//! One recognised result line.

/// A result line reduced to its path and its line field.
///
/// Both halves are kept as text: the line field is numeric in every real result but the
/// preview prints it verbatim, and a non-numeric middle is the parser's problem, not the
/// renderer's. It is not a match: the matched text is never read, so a result whose
/// match spans several lines still yields exactly one record.
pub(crate) struct Struct {
	pub Path:String,
	pub Line:String,
}
