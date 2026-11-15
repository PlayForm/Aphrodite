//! Render the markdown outline.

use crate::Preview::Summary::Markdown::Struct::Struct as Summary;

/// Characters of the first heading a preview shows: forty-eight.
const HeadingShown:usize = 48;

/// Render a document summary as `[md:<n>L h1×n h2×n | <first heading>]`.
///
/// The tally lists only the levels that occurred, in level order, and the heading block
/// is omitted when no heading was found. It is not a table of contents: counts plus one
/// heading, which is what the arm has always promised.
pub(crate) fn Fn(Summary:&Summary, Total:usize) -> String {
	let Tally:Vec<String> = Summary.Tally().map(|(Level, Count)| format!("h{Level}×{Count}")).collect();

	match (Tally.as_slice(), Summary.First()) {
		([], _) => format!("[md:{Total}L]"),
		(_, Some(First)) => {
			format!(
				"[md:{Total}L {} | {}]",
				Tally.join(" "),
				First.chars().take(HeadingShown).collect::<String>()
			)
		},
		(_, None) => format!("[md:{Total}L {}]", Tally.join(" ")),
	}
}
