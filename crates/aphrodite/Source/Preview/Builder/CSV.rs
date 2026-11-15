//! CSV preview arm.
//!
//! A CSV payload is a grid, and its useful shape is how big the grid is and
//! what its header calls the columns. This arm renders that shape. It is not a
//! CSV parser - quoting, embedded commas and line continuations are not
//! honoured, so a quoted field containing a comma inflates the column count.
//! That is accepted because the alternative is a parser for a preview line.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of CSV content.
///
/// Promises the row count, the column count and up to five header cells. It is
/// not a validating reader: a malformed row is counted like any other, and the
/// column count comes from the first row alone, so a ragged file reports the
/// width of its header rather than a range.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Rows:Vec<&str> = Inp.raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
	let Cols = Rows.first().map(|r| r.split(',').count()).unwrap_or(0);
	let Header:Vec<&str> = Rows
		.first()
		.map(|r| r.split(',').map(|c| c.trim()).take(5).collect())
		.unwrap_or_default();
	let More = if Cols > Header.len() {
		format!(" +{} more", Cols - Header.len())
	} else {
		String::new()
	};
	format!("[csv:{} rows {} cols | {}{}]", Rows.len(), Cols, Header.join(", "), More)
}
