//! The first failing test's name.

/// The three shapes a runner names a failure in, in the order they are tested.
const Shapes:[&str; 3] = ["--- FAIL: ", "test ", "FAILED "];

/// The name of the first failing test, or `None`.
///
/// Go writes `--- FAIL: Name`, cargo writes `test Name ... FAILED`, and some runners write
/// `FAILED Name`; the first line matching any of the three wins, and the name is its first
/// whitespace-separated token. It is not a result counter - the tallies come from the summary
/// line - and it exists so a run that failed without a summary can never look clean.
pub(crate) fn Fn(Content:&str) -> Option<String> {
	Content
		.lines()
		.find_map(|Line| {
			let Trimmed = Line.trim();

			Trimmed
				.strip_prefix(Shapes[0])
				.or_else(|| Trimmed.strip_prefix(Shapes[1]).filter(|_| Trimmed.ends_with("FAILED")))
				.or_else(|| Trimmed.strip_prefix(Shapes[2]))
				.map(|Rest| Rest.split_whitespace().next().unwrap_or("").to_string())
		})
		.filter(|Name| !Name.is_empty())
}
