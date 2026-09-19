//! Bare path-like token predicate (find output / plain `ls`).

/// True when a line is a bare path-like token (find output / plain `ls`): a
/// single whitespace-free token that has an extension or a path separator.
pub(crate) fn is_path_line(line:&str) -> bool {
	let t = line.trim();
	if t.is_empty() || t.contains(char::is_whitespace) {
		return false;
	}
	t.contains('/') || (t.rfind('.').map(|i| i > 0 && i < t.len() - 1).unwrap_or(false))
}