//! `add_timing`: record one sample.

use std::sync::atomic::Ordering;

use crate::Debug::{
	Level::Enabled::Fn::Fn as Enabled,
	Timing::{
		Const::{TimingN, TimingSum},
		Enum::Enum as TimingCat,
	},
};

/// Add a timing sample (no-op when the logger is off).
///
/// The caller measures the duration itself and only when `enabled()` - this
/// function must never be the thing that makes a hot path call `Instant::now`.
/// `then_some` keeps the two `fetch_add`s inside a closure the disabled path never
/// calls, so the gate still costs one relaxed load.
pub fn Fn(Cat:TimingCat, Us:u64) {
	// The slot index is what the gate buys: both tallies land on the same one, and
	// the disabled path has no index at all - one relaxed load, no `fetch_add`.
	Enabled().then_some(Cat as usize).into_iter().for_each(|Index| {
		TimingSum[Index].fetch_add(Us, Ordering::Relaxed);
		TimingN[Index].fetch_add(1, Ordering::Relaxed);
	});
}
