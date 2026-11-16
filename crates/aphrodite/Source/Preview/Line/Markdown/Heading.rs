//! Markdown heading-line predicate.
//!
//! The markdown arm reports heading count and the first heading. That count is
//! presented to the model as fact, so `#hashtag` and `# `-less hashes must not
//! be counted as headings.

/// True for a markdown heading line (`# `, `## `, ... `###### `).
///
/// Requires one to six hashes, a following space, and non-empty text after it.
/// It is not a markdown parser: setext headings and fenced code are out of
/// scope, and a line inside a fence that happens to look like a heading is
/// counted - the arm's count is an approximation the preview never claims to be
/// exact beyond the headings it prints.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	let n = t.chars().take_while(|c| *c == '#').count();
	(1..=6).contains(&n) && t.len() > n && t[n..].starts_with(' ') && !t[n..].trim().is_empty()
}
