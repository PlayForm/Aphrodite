//! CSV preview arm.

use crate::preview::input::Input;

/// CSV preview: row count, column count, header cells.
/// `[csv:4 rows 3 cols | name, age, city]`.
pub(crate) fn build_csv_preview(inp:&Input<'_>) -> String {
	let rows:Vec<&str> = inp.raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
	let cols = rows.first().map(|r| r.split(',').count()).unwrap_or(0);
	let header:Vec<&str> = rows
		.first()
		.map(|r| r.split(',').map(|c| c.trim()).take(5).collect())
		.unwrap_or_default();
	let more = if cols > header.len() {
		format!(" +{} more", cols - header.len())
	} else {
		String::new()
	};
	format!("[csv:{} rows {} cols | {}{}]", rows.len(), cols, header.join(", "), more)
}