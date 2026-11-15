//! The ratio rule that moves the split threshold.
//!
//! Two thresholds, not a gradient: at or above 50% retrieval the agent is
//! using the segments, so split more eagerly; below 25% it is ignoring them, so
//! only split long chains. The dead band between the two is deliberate - a
//! single noisy window must not oscillate the threshold every turn, and the
//! `SplitAdaptMinimumEvents` floor keeps a one-off split from being read as a
//! trend at all.

use crate::State::{Const::SplitAdaptMinimumEvents, Struct::AphroditeState};

impl AphroditeState {
	/// Consequence-driven threshold adaptation (Tier 1, invisible): with at
	/// least `SplitAdaptMinimumEvents` split events recorded, compute the
	/// retrieval ratio (retrieved/produced) over the window. Agents that
	/// retrieve the segments (> 50%) keep the threshold at the floor (split
	/// eagerly); agents that ignore them (< 25%) raise it (only split long
	/// chains), capped by `ChainSplitMaximumSegments`. The threshold is
	/// internal machinery - never rendered to the LLM.
	pub(super) fn AdaptChainSplitThreshold(&mut self) {
		if self.SplitEvents.len() < SplitAdaptMinimumEvents {
			return;
		}
		let Produced:usize = self.SplitEvents.iter().map(|e| e.Produced).sum();
		let Retrieved:usize = self.SplitEvents.iter().map(|e| e.Retrieved).sum();
		if Produced == 0 {
			return;
		}
		let Ratio = Retrieved as f64 / Produced as f64;
		if Ratio >= 0.5 && self.ChainSplitMinimumSegments > self.ChainSplitFloor {
			// Segments are used - split more eagerly.
			self.ChainSplitMinimumSegments -= 1;
		} else if Ratio < 0.25 && self.ChainSplitMinimumSegments < self.ChainSplitMaximumSegments {
			// Segments are ignored - only split very long chains.
			self.ChainSplitMinimumSegments += 1;
		}
	}
}
