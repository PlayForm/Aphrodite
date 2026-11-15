//! Render a test-run preview from the first summary shape that answers.

use crate::Preview::Input::Struct::Struct as Input;
use super::super::{
	Bare::Fn::Fn as BareTotals,
	Cargo::Fn::Fn as CargoTotals,
	Duration::Fn::Fn as RunDuration,
	Failure::Fn::Fn as FirstFailure,
	Pytest::Fn::Fn as PytestTotals,
};

/// Characters of the failing-test name the preview shows: forty.
const FailureShown:usize = 40;

/// Build a preview of test output.
///
/// Promises pass/fail/ignored tallies plus the first failing test's name or the run duration. The
/// three summary shapes are tried in order and the first that finds a tally wins, so a later shape
/// never runs once an earlier one answered - which is both the previous precedence and the reason the
/// fallbacks cost nothing on the common path.
///
/// It is not a verdict: a run whose summary line is missing reports whatever its bare result lines say,
/// and a run that failed without any tally still names its failure, so a failing run can never look
/// clean.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Totals = CargoTotals(Inp).or_else(|| PytestTotals(Inp)).or_else(|| BareTotals(Inp));
	let Failure = FirstFailure(Inp.raw);
	let Duration = RunDuration(Inp.raw);

	match (Totals, Failure) {
		(None, None) => format!("[test:{}L]", Inp.Total),
		(Totals, Failure) => {
			let Totals = Totals.unwrap_or_default();
			let Tail = Failure
				.map(|Name| format!(" | FAIL {}", Name.chars().take(FailureShown).collect::<String>()))
				.or_else(|| Duration.map(|Duration| format!(" | {Duration}")))
				.unwrap_or_default();

			format!(
				"[test:{} pass {} fail {} ignored{Tail}]",
				Totals.Pass, Totals.Fail, Totals.Ignored
			)
		},
	}
}
