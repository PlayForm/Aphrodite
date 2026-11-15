//! The three counts a test run reports.

/// Pass, fail and ignored counts.
///
/// `Default` is the all-zero tally, which is what a run with no summary line contributes
/// when a failing-test name is still available - the name alone is a real signal and the
/// counts are honestly unknown. It is not a verdict: the counts come from whichever summary
/// shape was recognised, and no shape recognised means no counts rather than a guess.
#[derive(Default)]
pub(crate) struct Struct {
	pub Pass:usize,
	pub Fail:usize,
	pub Ignored:usize,
}
