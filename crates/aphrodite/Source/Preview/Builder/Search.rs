//! Search preview arm.
//!
//! Parse each result with the plain rule (no file-ish path requirement), aggregate the
//! tally, then render it with the `search` wire word. It is not a ranking and it does not
//! read match text.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render, Summary};

/// Build a preview of search results.
///
/// Promises the hit count, the number of distinct files and the first match location. It
/// is not the grep arm: the path here need not look file-ish, so a result whose path has
/// no extension still counts - which is the whole difference between the two shapes.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Summary = Inp.raw.lines().filter_map(Parse::Search::Plain::Fn::Fn).fold(
		Summary::Search::Struct::Struct::new(),
		|mut Summary, Hit| {
			Summary.Push(Hit);

			Summary
		},
	);

	Render::Search::Fn::Fn("search", &Summary, Inp.Total)
}
