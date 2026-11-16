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
/// `then` keeps the two `fetch_add`s inside a closure the disabled path never
/// calls, so the gate still costs one relaxed load.
pub fn Fn(Cat:TimingCat, Us:u64) {
	if Enabled() {
		let Index = Cat as usize;

		TimingSum[Index].fetch_add(Us, Ordering::Relaxed);
		TimingN[Index].fetch_add(1, Ordering::Relaxed);
	}
}
