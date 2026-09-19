//! Markdown structural-line predicate (list item, link, fence, blockquote,
//! horizontal rule).

/// True for a non-heading markdown structural line (list item, link, fence,
/// blockquote, horizontal rule).
pub(crate) fn is_md_structure(line:&str) -> bool {
	let t = line.trim_start();
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