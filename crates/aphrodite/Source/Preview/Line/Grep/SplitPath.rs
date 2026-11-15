//! Shared `path:line:` split for the grep and search predicates.

/// Split a `path:line:...` candidate into its path and line fields.
///
/// Returns `None` when the path is missing, whitespace-containing or empty, or
/// when the middle field is not a line number, or when nothing follows it. It
/// is not a path validator: the path is returned as written, so a Windows drive
/// letter or a URL scheme would split wrongly - neither appears in the tool
/// output this reads.
pub(super) fn Fn(Line:&str) -> Option<(&str, &str)> {
	let mut It = Line.splitn(3, ':');
	let path = match It.next() {
		Some(p) if !p.trim().is_empty() && !p.contains(' ') => p,
		_ => return None,
	};
	match It.next() {
		// `path:line:...` - middle is a line number, third segment exists.
		Some(Mid) if Mid.chars().all(|c| c.is_ascii_digit()) && !Mid.is_empty() && It.next().is_some() => {
			Some((path, Mid))
		},
		_ => None,
	}
}
