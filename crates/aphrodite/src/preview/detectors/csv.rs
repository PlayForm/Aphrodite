//! csv shape detector.

use crate::preview::input::Input;

/// ≥2 rows with an identical comma-field count (≥2 fields) and no `, `
/// (comma-space) - prose with commas is excluded by both tests.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let csv_rows:Vec<&str> = inp.non_empty.iter().map(|l| l.trim()).collect();
	if csv_rows.len() >= 2 && !inp.raw.contains(", ") {
		let counts:Vec<usize> = csv_rows.iter().map(|r| r.split(',').count()).collect();
		let first = counts[0];
		if first >= 2 && counts.iter().all(|&c| c == first) {
			return true;
		}
	}
	false
}
