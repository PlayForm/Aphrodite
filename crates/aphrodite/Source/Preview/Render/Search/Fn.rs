//! Render the result tally for either result arm.

use crate::Preview::Summary::Search::Struct::Struct as Summary;

/// Characters of the first location a preview shows: forty-eight.
const LocationShown:usize = 48;

/// Render a result summary as `[<kind>:<n> hits in <n> files | <path>:<line> ...]`.
///
/// `Kind` is the wire word for the arm (`grep` or `search`) and is passed in rather than
/// derived, so the recogniser never has to know which arm it belongs to - that is the
/// whole reason the two producers can share a summary. It is not a ranking: the location
/// is the first hit in input order.
pub(crate) fn Fn(Kind:&str, Summary:&Summary, Total:usize) -> String {
	match (Summary.Hits(), Summary.First()) {
		(0, _) => format!("[{Kind}:{Total}L]"),
		(Hits, Some(First)) => {
			format!(
				"[{Kind}:{Hits} hits in {} files | {} ...]",
				Summary.Files(),
				format!("{}:{}", First.Path, First.Line)
					.chars()
					.take(LocationShown)
					.collect::<String>()
			)
		},
		(Hits, None) => format!("[{Kind}:{Hits} hits in {} files]", Summary.Files()),
	}
}
