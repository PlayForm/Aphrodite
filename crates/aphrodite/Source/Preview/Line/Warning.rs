//! Compiler warning-line predicate.
//!
//! Warnings are reported as a count alongside errors. The same line-based rule
//! applies: a word containing "warning" is not a warning line.

/// True for a line that is a real compiler warning line.
///
/// Matches `warning[`/`warning:`/`Warning:`/`WARNING` and a
/// `file:line: warning:` prefix. It is not a substring match, so a prose line
/// mentioning a warning is not counted - the tally is presented as fact.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	t.starts_with("warning[")
		|| t.starts_with("warning:")
		|| t.starts_with("Warning:")
		|| t.starts_with("WARNING")
		|| t.contains(": warning[")
		|| t.contains(": warning:")
}
