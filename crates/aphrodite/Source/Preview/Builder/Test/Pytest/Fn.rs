//! pytest's `N passed, M failed` summary shape.

use crate::Preview::{Input::Struct::Struct as Input, Text::NumberBefore::Exact};
use super::super::Totals::Struct::Struct as Totals;

/// Parse the first summary line that carries a non-zero count.
///
/// A line is a candidate when it mentions `passed` or `failed` AND the count in front of one
/// of them is non-zero - so prose that merely contains the word does not become a tally, and
/// a `0 passed, 0 failed` collection line does not either. It is not a jest reader: jest
/// writes the number after the keyword and has its own detector predicate.
pub(crate) fn Fn(Inp:&Input<'_>) -> Option<Totals> {
	let Hits:Vec<(usize, usize)> = Inp
		.raw
		.lines()
		.filter(|Line| Line.contains("passed") || Line.contains("failed"))
		.map(|Line| (Exact::Fn(Line, "passed"), Exact::Fn(Line, "failed")))
		.filter(|(Pass, Fail)| *Pass > 0 || *Fail > 0)
		.collect();

	(!Hits.is_empty()).then(|| {
		Totals {
			Pass:Hits.iter().map(|(Pass, _)| Pass).sum(),
			Fail:Hits.iter().map(|(_, Fail)| Fail).sum(),
			Ignored:0,
		}
	})
}
