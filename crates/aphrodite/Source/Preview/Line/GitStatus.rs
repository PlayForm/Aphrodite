//! Git porcelain / short-status code predicate.
//!
//! `git status --porcelain` writes a two-column code, a space, then a path.
//! The git arm tallies staged against unstaged by reading those two columns,
//! so the reader has to be strict: indented prose is not a status line, and a
//! two-space prefix is not a code.

/// Git porcelain / short-status code for a line, or `None`.
///
/// Returns the two-character code (`M `, ` M`, `A `, `D `, `R `, `??`, `UU`)
/// when the line is shaped `<col><col> <path>`. It is not a parser of the
/// index: the code is returned verbatim and the caller decides what it means,
/// so an unusual-but-real status is reported rather than dropped.
pub(crate) fn Fn(Line:&str) -> Option<&str> {
	let b = Line.as_bytes();
	if b.len() < 4 {
		return None;
	}
	let Codes = *b"MADRCU?! T";
	let C0 = b[0];
	let C1 = b[1];
	// Reject an all-space prefix (that's just indented prose).
	if (C0 == b' ' && C1 == b' ') || !Codes.contains(&C0) || !Codes.contains(&C1) {
		return None;
	}
	// Column 3 must be a space separating code from path, and a path follows.
	if b[2] != b' ' || Line[3..].trim().is_empty() {
		return None;
	}
	// A two-char code of at least one real status letter (not `  `).
	Line.get(..2)
}
