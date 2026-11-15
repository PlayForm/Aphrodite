//! Markdown table preview arm.
//!
//! A table is described by its width, its height and its header cells. This arm
//! renders those. It is not a grid reader: cell contents beyond the header are
//! not shown, so a table's data previews as its shape.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of a markdown table.
///
/// Promises the column count, the row count and up to five header cells. It is
/// not a data sample: rows are counted as non-empty lines minus separator rows,
/// so a table with wrapped cells reports more rows than it has.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let IsSep = |l:&str| {
		let Cells = l.trim().trim_matches('|');
		!Cells.is_empty()
			&& Cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|Character| matches!(Character, '-' | ':' | ' '))
			})
	};
	let Rows:Vec<&str> = Inp
		.raw
		.lines()
		.map(|l| l.trim())
		.filter(|l| !l.is_empty() && !IsSep(l))
		.collect();
	let Header = Rows.first().unwrap_or(&"").trim_matches('|');
	let Cells:Vec<&str> = Header.split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
	let Cols = Cells.len();
	let Shown:Vec<&str> = Cells.iter().take(5).copied().collect();
	let More = match Cols > Shown.len() {
		true => format!(" +{} more", Cols - Shown.len()),
		false => String::new(),
	};

	format!("[table:{} cols {} rows | {}{}]", Cols, Rows.len(), Shown.join(", "), More)
}
