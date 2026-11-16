//! Shared `path:line:` split for the grep and search predicates.

/// Split a `path:line:...` candidate into its path and line fields.
///
/// Returns `None` when the path is missing, whitespace-containing or empty, or when the middle
/// field is not a line number, or when nothing follows it. It is not a path validator: the path
/// is returned as written (untrimmed), so a Windows drive letter or a URL scheme would split
/// wrongly - neither appears in the tool output this reads.
pub(super) fn Fn(Line:&str) -> Option<(&str, &str)> {
	let mut Fields = Line.splitn(3, ':');
	let Path = Fields.next().filter(|Path| !Path.trim().is_empty() && !Path.contains(' '))?;
	let Number = Fields
		.next()
		.filter(|Field| !Field.is_empty() && Field.chars().all(|Character| Character.is_ascii_digit()))?;

	Fields.next()?;

	Some((Path, Number))
}
