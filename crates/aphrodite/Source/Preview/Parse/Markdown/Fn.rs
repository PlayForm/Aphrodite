//! Aggregate markdown headings into the document summary.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Parse::Markdown::Heading::Fn::Fn as Heading,
	Summary::Markdown::Struct::Struct as Summary,
};

/// Parse a document into its per-level heading counts and its first heading.
///
/// One fold, so the counts and the first heading are filled in the same pass. It is not
/// a table of contents: only headings are read, so a document that is all prose yields
/// an empty summary rather than an error.
pub(crate) fn Fn(Inp:&Input<'_>) -> Summary {
	Inp.raw
		.lines()
		.filter_map(Heading)
		.fold(Summary::new(), |mut Summary, Heading| {
			Summary.Push(Heading);

			Summary
		})
}
