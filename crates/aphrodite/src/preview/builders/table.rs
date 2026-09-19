//! Markdown table preview arm.

use crate::preview::input::Input;

/// Markdown table preview: column count, row count, header cells.
/// `[table:3 cols 4 rows | Name, Age, City]`. Rows = non-empty lines minus
/// separator rows (header + data rows).
pub(crate) fn build_table_preview(inp: &Input<'_>) -> String {
	let is_sep = |l: &str| {
		let cells = l.trim().trim_matches('|');
		!cells.is_empty()
			&& cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
			})
	};
	let rows: Vec<&str> = inp
		.raw
		.lines()
		.map(|l| l.trim())
		.filter(|l| !l.is_empty() && !is_sep(l))
		.collect();
	let header = rows.first().unwrap_or(&"").trim_matches('|');
	let cells: Vec<&str> = header.split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
	let cols = cells.len();
	let shown: Vec<&str> = cells.iter().take(5).copied().collect();
	let more = if cols > shown.len() {
		format!(" +{} more", cols - shown.len())
	} else {
		String::new()
	};
	format!("[table:{} cols {} rows | {}{}]", cols, rows.len(), shown.join(", "), more)
}
