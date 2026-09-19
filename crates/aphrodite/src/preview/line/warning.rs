//! Compiler warning-line predicate.

/// True for a line that is a real compiler warning line (`warning[`/`warning:`
/// /`Warning:`/`WARNING` or a `file:line: warning:` prefix).
pub(crate) fn is_warning_line(line: &str) -> bool {
	let t = line.trim_start();
	t.starts_with("warning[")
		|| t.starts_with("warning:")
		|| t.starts_with("Warning:")
		|| t.starts_with("WARNING")
		|| t.contains(": warning[")
		|| t.contains(": warning:")
}
