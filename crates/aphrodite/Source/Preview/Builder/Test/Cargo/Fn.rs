//! cargo's `test result:` summary shape.

use crate::Preview::{Input::Struct::Struct as Input, Text::NumberBefore::Exact};
use super::super::Totals::Struct::Struct as Totals;

/// Parse every `test result:` line into one summed tally.
///
/// A cargo test run can print several summary lines (one per test binary), and the previous
/// reader summed all of them, so this does too. It is not a verdict on the run: `0 failed`
/// on the last binary does not make a run clean, which is why the failure marker is read
/// separately.
pub(crate) fn Fn(Inp:&Input<'_>) -> Option<Totals> {
	let Lines:Vec<&str> = Inp.raw.lines().filter_map(|Line| Line.split("test result:").nth(1)).collect();

	(!Lines.is_empty()).then(|| {
		Totals {
			Pass:Lines.iter().map(|Rest| Exact::Fn(Rest, "passed")).sum(),
			Fail:Lines.iter().map(|Rest| Exact::Fn(Rest, "failed")).sum(),
			Ignored:Lines.iter().map(|Rest| Exact::Fn(Rest, "ignored")).sum(),
		}
	})
}
