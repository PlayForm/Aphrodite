//! Resolve which gate refuses, if any.

use crate::{Session::Engine::Gate::Enum::Enum as Gate, State::Struct::AphroditeState};

/// The first gate that refuses, or `None` when the engine may archive.
///
/// The ORDER is the contract and is spelled out as an ordered table: engine-off
/// first (nothing else is worth computing), then the minimum entry count, then
/// the fill ratio, then whether the middle slice is worth compacting. Reordering
/// two rows changes which reason a session reports for the same state, and the
/// `engine_skip` line is what an investigation reads.
///
/// Every row is a pure predicate over the index and the configuration, and every
/// reason is fieldless, so evaluating the whole table costs nothing and cannot
/// skip work a later row needed.
pub(crate) fn Fn(State:&AphroditeState, Total:usize, FillPct:u64, MiddleN:usize) -> Option<Gate> {
	let Disabled = !State.ContextEngineEnabled || State.EngineThresholdPct == 0 || State.EngineThresholdPct >= 100;
	let BelowMinimum = Total < State.EngineMinimumMsgs;
	let BelowThreshold = FillPct < State.EngineThresholdPct;
	let Small = MiddleN < 3;

	[
		(Disabled, Gate::Disabled),
		(BelowMinimum, Gate::BelowMinimum),
		(BelowThreshold, Gate::BelowThreshold),
		(Small, Gate::Small),
	]
	.into_iter()
	.find(|(Hit, _)| *Hit)
	.map(|(_, Reason)| Reason)
}
