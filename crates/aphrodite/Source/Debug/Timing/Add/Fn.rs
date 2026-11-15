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
pub fn Fn(Cat:TimingCat, Us:u64) {
	match !Enabled() {
		true => return,
		false => (),
	}

	let Index = Cat as usize;
	TimingSum[Index].fetch_add(Us, Ordering::Relaxed);
	TimingN[Index].fetch_add(1, Ordering::Relaxed);
}
