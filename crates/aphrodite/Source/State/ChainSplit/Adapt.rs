//! The ratio rule that moves the split threshold.
//!
//! Two thresholds, not a gradient: at or above 50% retrieval the agent is
//! using the segments, so split more eagerly; below 25% it is ignoring them, so
//! only split long chains. The dead band between the two is deliberate - a
//! single noisy window must not oscillate the threshold every turn, and the
//! `SplitAdaptMinimumEvents` floor keeps a one-off split from being read as a
//! trend at all.

use super::Trend::{Decide::Fn::Fn as Decide, Enum::Enum as Trend};
use crate::State::{Const::SplitAdaptMinimumEvents, Struct::AphroditeState};

impl AphroditeState {
	/// Consequence-driven threshold adaptation (Tier 1, invisible): with at
	/// least `SplitAdaptMinimumEvents` split events recorded, compute the
	/// retrieval ratio (retrieved/produced) over the window. Agents that
	/// retrieve the segments (> 50%) keep the threshold at the floor (split
	/// eagerly); agents that ignore them (< 25%) raise it (only split long
	/// chains), capped by `ChainSplitMaximumSegments`. The threshold is
	/// internal machinery - never rendered to the LLM.
	///
	/// The two early returns of the original are the two `Option` stages below:
	/// too few events means no window at all, and a window that produced nothing
	/// has no ratio. Both collapse to `Hold`, which is the honest answer for
	/// "there is nothing to adapt from" - and `then`/`filter` keep the fold from
	/// running when either condition already failed.
	pub(super) fn Adapt(&mut self) {
		let Trend = (self.SplitEvents.len() >= SplitAdaptMinimumEvents)
			.then(|| {
				self.SplitEvents.iter().fold((0usize, 0usize), |(Produced, Retrieved), Event| {
					(Produced + Event.Produced, Retrieved + Event.Retrieved)
				})
			})
			.filter(|(Produced, _)| *Produced > 0)
			.map(|(Produced, Retrieved)| Retrieved as f64 / Produced as f64)
			.map(|Ratio| {
				Decide(
					Ratio,
					self.ChainSplitMinimumSegments,
					self.ChainSplitFloor,
					self.ChainSplitMaximumSegments,
				)
			})
			.unwrap_or(Trend::Hold);

		match Trend {
			Trend::Decrease => self.ChainSplitMinimumSegments -= 1,
			Trend::Increase => self.ChainSplitMinimumSegments += 1,
			Trend::Hold => (),
		}
	}
}
