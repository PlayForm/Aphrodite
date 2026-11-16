//! The two kinds of line a marked output contains.

/// A line of marked output: a segment boundary or text belonging to the part
/// being built.
///
/// The split is driven by this classification rather than by a comparison at the
/// split site, because a line is exactly one of two things and only one of them
/// carries content. The `Text` arm holds the line verbatim - the caller appends it
/// with its newline - so neither arm has to re-read what the line was.
pub(crate) enum Enum<'a> {
	/// A marker line: closes the part being built and advances the index.
	Boundary,
	/// A content line, kept verbatim.
	Text(&'a str),
}
