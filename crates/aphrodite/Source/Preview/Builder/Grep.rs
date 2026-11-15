//! grep/ripgrep preview arm.
//!
//! Parse each hit with the file-ish-path rule, aggregate the tally, then render it. It is
//! not a match extractor: the first location is shown and no match text is parsed.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render, Summary};

/// Build a preview of grep/ripgrep output.
///
/// Promises the hit count, the number of distinct files and the first location. It is not
/// a relevance ranking: hits are counted as printed, so a file mentioned twice counts once
/// in the file total and twice in the hit total.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Summary = Inp.raw.lines().filter_map(Parse::Search::Grep::Fn::Fn).fold(
		Summary::Search::Struct::Struct::new(),
		|mut Summary, Hit| {
			Summary.Push(Hit);

			Summary
		},
	);

	Render::Search::Fn::Fn("grep", &Summary, Inp.Total)
}
