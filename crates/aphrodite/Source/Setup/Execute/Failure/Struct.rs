//! A failed step, with the steps that ran before it.

use crate::Setup::Plan::Step;

/// Partial progress, preserved across a failure.
///
/// `Execute` used to return `anyhow::Result<Report>`, so the report was dropped by `?`
/// exactly when it mattered: the doc comment claimed "a failed run reports exactly the
/// prefix it finished", but the prefix lived in a value the error path never returned.
/// A setup that failed at step 6 of 9 left five artifacts on disk and said only
/// "failed", which is the one thing an operator cannot act on - re-running overwrites
/// them, so knowing which ones are already there is the difference between a repair and
/// a guess.
///
/// There is still no rollback: nothing records what a step replaced, so this says what
/// happened, not how to undo it.
pub(crate) struct Struct {
	/// The steps that returned `Ok` before the failure, in execution order.
	pub Completed:Vec<Step::Enum::Enum>,
	/// The failure itself, with its context chain intact.
	pub Error:anyhow::Error,
}
