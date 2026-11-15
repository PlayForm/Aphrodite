//! Markdown structural-line predicate.
//!
//! A markdown document is recognised by its non-prose lines - list items,
//! links, fences, blockquotes, rules. This predicate votes on those lines; the
//! heading predicate votes separately because a document can be all headings.

/// True for a non-heading markdown structural line.
///
/// Matches list items (`-`, `*`, `+`), blockquotes, fenced code (` ``` `,
/// `~~~`), table rows (`|`), inline links (`[...](...)`), and horizontal rules
/// of three or more `-`/`=` characters. It is not a prose detector: an
/// ordinary sentence returns false, which is what keeps shell output with a
/// leading `-` from being claimed as markdown.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	t.starts_with("- ")
		|| t.starts_with("* ")
		|| t.starts_with("+ ")
		|| t.starts_with("> ")
		|| t.starts_with("```")
		|| t.starts_with("~~~")
		|| t.starts_with("|")
		|| t.starts_with("[") && t.contains("](")
		|| (!t.is_empty() && t.chars().all(|c| c == '-' || c == '=') && t.len() >= 3)
}
