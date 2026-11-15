//! First meaningful content line.
//!
//! Pretty-printed payloads open with structural punctuation - a lone `{`, a
//! lone `[`, a closing brace - and previewing that line says nothing about the
//! content. This module skips such noise and returns the first line that
//! actually carries content. It is not a parser and it does not know the
//! payload's type; it only knows that a line consisting of one bracket is
//! structural, so a preview built on it describes the payload instead of its
//! opening brace.

/// First non-empty line whose trimmed form is not structural noise.
///
/// Structural noise is a lone `{`/`}`/`[`/`]`/`(`/`)` with an optional
/// trailing `,` or `;`. Pretty-printed JSON opens with a lone `{`, and
/// previewing that line alone is the misleading-preview bug this function
/// exists to prevent: the generic arm would show the first line, so the first
/// line has to be a real one. Returns `None` when every line is noise, and the
/// caller must then fall back rather than invent content.
pub(crate) fn Fn(Content:&str) -> Option<String> {
	Content.lines().find_map(|l| {
		let t = l.trim();
		if t.is_empty() {
			return None;
		}
		let core = t.trim_end_matches([',', ';']).trim();
		let mut It = core.chars();
		match (It.next(), It.next()) {
			(Some('{' | '}' | '[' | ']' | '(' | ')'), None) => None,
			_ => Some(t.to_string()),
		}
	})
}
