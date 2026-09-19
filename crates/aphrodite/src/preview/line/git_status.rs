//! Git porcelain / short-status code predicate.

/// Git porcelain / short-status code for a line (`M `, ` M`, `A `, `D `, `R `,
/// `??`, `UU`, etc.), or `None`. Two leading columns (staged, unstaged) then a
/// space then a path.
pub(crate) fn git_status_code(line: &str) -> Option<&str> {
	let b = line.as_bytes();
	if b.len() < 4 {
		return None;
	}
	let codes = [b'M', b'A', b'D', b'R', b'C', b'U', b'?', b'!', b' ', b'T'];
	let c0 = b[0];
	let c1 = b[1];
	// Reject an all-space prefix (that's just indented prose).
	if (c0 == b' ' && c1 == b' ') || !codes.contains(&c0) || !codes.contains(&c1) {
		return None;
	}
	// Column 3 must be a space separating code from path, and a path follows.
	if b[2] != b' ' || line[3..].trim().is_empty() {
		return None;
	}
	// A two-char code of at least one real status letter (not `  `).
	line.get(..2)
}
