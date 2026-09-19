//! First meaningful content line (skips structural noise).

/// First non-empty line whose trimmed form is not structural noise (a lone
/// `{`/`}`/`[`/`]`/`(`/`)` with optional trailing `,`/`;`). Pretty-printed
/// JSON opens with a lone `{` - previewing that line alone is the ISSUE-11
/// residual #1 MISLEADING bug (RC-D: the generic arm showed the first line).
/// Skips to the first real content line (first key, first statement).
pub(crate) fn first_meaningful_line(content: &str) -> Option<String> {
	content.lines().find_map(|l| {
		let t = l.trim();
		if t.is_empty() {
			return None;
		}
		let core = t.trim_end_matches([',', ';']).trim();
		let mut it = core.chars();
		match (it.next(), it.next()) {
			(Some('{' | '}' | '[' | ']' | '(' | ')'), None) => None,
			_ => Some(t.to_string()),
		}
	})
}
