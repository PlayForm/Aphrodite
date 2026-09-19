//! Markdown heading-line predicate.

/// True for a markdown heading line (`# `, `## `, ... `###### `).
pub(crate) fn is_md_heading(line:&str) -> bool {
	let t = line.trim_start();
	let n = t.chars().take_while(|c| *c == '#').count();
	(1..=6).contains(&n) && t.len() > n && t[n..].starts_with(' ') && !t[n..].trim().is_empty()
}