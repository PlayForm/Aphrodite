//! markdown table shape detector.

use crate::preview::input::Input;

/// >=2 `|`-prefixed lines with a separator row.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let md_table_lines = inp.non_empty.iter().filter(|l| l.trim_start().starts_with('|')).count();
	let has_table_sep = inp.non_empty.iter().any(|l| {
		let cells = l.trim().trim_matches('|');
		!cells.is_empty()
			&& cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
			})
	});
	md_table_lines >= 2 && has_table_sep
}