//! CSV shape detector.
//!
//! Two rows with an identical field count is the strongest available signal for
//! CSV, and it is weak enough to need a second test. This detector applies both.
//! It is not a CSV parser: it only decides whether the shape is a grid.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like CSV.
///
/// Two or more rows sharing a field count of at least two, with no `, `
/// (comma-space) anywhere - prose with commas is excluded by both tests. It is
/// not a quoting-aware check, so a quoted field containing a comma can hide a
/// real CSV.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let CSVRows:Vec<&str> = Inp.NonEmpty.iter().map(|l| l.trim()).collect();
	if CSVRows.len() >= 2 && !Inp.raw.contains(", ") {
		let Counts:Vec<usize> = CSVRows.iter().map(|r| r.split(',').count()).collect();
		let First = Counts[0];
		if First >= 2 && Counts.iter().all(|&c| c == First) {
			return true;
		}
	}
	false
}
