//! Markdown table preview arm.
//!
//! A table is described by its width, its height and its header cells. This arm renders
//! those. It is not a grid reader: cell contents beyond the header are not shown, so a
//! table's data previews as its shape.

use crate::Preview::Input::Struct::Struct as Input;

/// Header cells the preview shows: five.
const CellsShown:usize = 5;

/// True for a separator row (`|---|---|`).
///
/// It is not a cell parser: the row is a separator when every cell is made only of dashes,
/// colons and spaces, which is the whole of the markdown separator grammar.
fn Separator(Line:&str) -> bool {
	let Cells = Line.trim().trim_matches('|');

	!Cells.is_empty()
		&& Cells.split('|').all(|Cell| {
			let Trimmed = Cell.trim();

			!Trimmed.is_empty() && Trimmed.chars().all(|Character| matches!(Character, '-' | ':' | ' '))
		})
}

/// Build a preview of a markdown table.
///
/// Promises the column count, the row count and up to five header cells. It is not a data
/// sample: rows are counted as non-empty lines minus separator rows, so a table with
/// wrapped cells reports more rows than it has.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Rows:Vec<&str> = Inp
		.raw
		.lines()
		.map(|Line| Line.trim())
		.filter(|Line| !Line.is_empty() && !Separator(Line))
		.collect();
	let Header = Rows.first().unwrap_or(&"").trim_matches('|');
	let Cells:Vec<&str> = Header
		.split('|')
		.map(|Cell| Cell.trim())
		.filter(|Cell| !Cell.is_empty())
		.collect();
	let Columns = Cells.len();
	let Shown:Vec<&str> = Cells.iter().take(CellsShown).copied().collect();
	let More = Columns
		.checked_sub(Shown.len())
		.filter(|More| *More > 0)
		.map(|More| format!(" +{More} more"))
		.unwrap_or_default();

	format!("[table:{Columns} cols {} rows | {}{More}]", Rows.len(), Shown.join(", "))
}
