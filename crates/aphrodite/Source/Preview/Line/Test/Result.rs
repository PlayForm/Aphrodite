//! `test <name> ... ok|FAILED|ignored` result-line predicate.

/// The three result markers a bare test log prints.
const Markers:[&str; 3] = ["ok", "FAILED", "ignored"];

/// True for a test result line.
///
/// `test ` then a non-empty name then `...` then one of `ok`, `FAILED` or `ignored` ending at a
/// word boundary. It is not a result counter - the tallies come from the summary line; this
/// predicate only decides whether a line belongs to the per-test stream.
///
/// The name is measured in characters and then used as a BYTE offset, so the suffix is taken
/// through `str::get` rather than by indexing: a non-ASCII test name used to make this slice
/// panic inside the preview path, and `get` answers `None` instead - the line is then simply not
/// a result line, which is the same verdict a malformed one gets.
pub(crate) fn Fn(Trimmed:&str) -> bool {
	let Rest = Trimmed.strip_prefix("test ").unwrap_or("");
	let NameLength = Rest.chars().take_while(|Character| !Character.is_whitespace()).count();
	let After = Rest.get(NameLength..).unwrap_or("").trim_start();
	let After = After.strip_prefix("...").unwrap_or("").trim_start();

	NameLength > 0
		&& Markers
			.into_iter()
			.find_map(|Marker| After.strip_prefix(Marker))
			.is_some_and(|Tail| {
				Tail.chars()
					.next()
					.map(|Character| !(Character.is_alphanumeric() || Character == '_'))
					.unwrap_or(true)
			})
}
