//! CSV preview arm.
//!
//! A CSV payload is a grid, and its useful shape is how big the grid is and what its
//! header calls the columns. This arm renders that shape. It is not a CSV parser -
//! quoting, embedded commas and line continuations are not honoured, so a quoted field
//! containing a comma inflates the column count. That is accepted because the alternative
//! is a parser for a preview line.

use crate::Preview::Input::Struct::Struct as Input;
use super::More::Fn::Fn as More;

/// Header cells the preview shows: five.
const HeaderShown:usize = 5;

/// Build a preview of CSV content.
///
/// Promises the row count, the column count and up to five header cells. It is not a
/// validating reader: a malformed row is counted like any other, and the column count
/// comes from the first row alone, so a ragged file reports the width of its header
/// rather than a range.
///
/// The rows come from the shared input view rather than a second `lines()` pass, so the
/// column count and the row count cost one scan of the payload and no row vector.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let First = Inp.NonEmpty.first().copied();
	let Columns = First.map(|Row| Row.split(',').count()).unwrap_or(0);
	let Header:Vec<&str> = First
		.map(|Row| Row.split(',').map(|Cell| Cell.trim()).take(HeaderShown).collect())
		.unwrap_or_default();

	format!(
		"[csv:{} rows {Columns} cols | {}{}]",
		Inp.NonEmpty.len(),
		Header.join(", "),
		More(Columns, Header.len())
	)
}
