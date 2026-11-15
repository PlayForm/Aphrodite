//! Git porcelain / short-status code predicate.
//!
//! `git status --porcelain` writes a two-column code, a space, then a path. The git arm tallies staged
//! against unstaged by reading those two columns, so the reader has to be strict: indented prose is not
//! a status line, and a two-space prefix is not a code.

/// The bytes a status column may carry.
const Codes:&[u8] = b"MADRCU?! T";

/// Git porcelain / short-status code for a line, or `None`.
///
/// Returns the two-character code (`M `, ` M`, `A `, `D `, `R `, `??`, `UU`) when the line is shaped
/// `<col><col> <path>`. It is not a parser of the index: the code is returned verbatim and the caller
/// decides what it means, so an unusual-but-real status is reported rather than dropped.
///
/// The four-byte minimum is a slice pattern rather than a length test, so the three columns this reads
/// are proved present before they are read - and a line that is too short is the same `None` as a line
/// whose columns are wrong.
pub(crate) fn Fn(Line:&str) -> Option<&str> {
	let &[First, Second, Separator, ..] = Line.as_bytes() else { return None };

	let Shaped = (First != b' ' || Second != b' ')
		&& Codes.contains(&First)
		&& Codes.contains(&Second)
		&& Separator == b' '
		&& !Line.get(3..).unwrap_or("").trim().is_empty();

	Shaped.then_some(Line.get(..2)).flatten()
}
