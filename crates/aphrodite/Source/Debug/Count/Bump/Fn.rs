//! `bump`: add one to a counter slot.

use std::sync::atomic::Ordering;

use crate::Debug::{
	Count::{Const::Counters, Enum::Enum as Counter},
	Level::Enabled::Fn::Fn as Enabled,
};

/// Increment a cumulative counter. No-op when the logger is off (one
/// relaxed load on the quiet path).
///
/// The gate comes FIRST: with the logger off this must cost one atomic load,
/// never a `fetch_add` on a cache line every thread contends for. That ordering
/// is the whole reason the counters are safe to leave in a release build, and
/// `then_some` is what keeps it: the increment lives in a closure the disabled path
/// never calls.
pub fn Fn(Counter:Counter) {
	// The slot exists only while the logger is on: the gate selects the counter the
	// increment applies to, so the disabled path holds no slot at all and pays one
	// relaxed load instead of a `fetch_add` on a contended line.
	Enabled().then_some(&Counters[Counter as usize]).into_iter().for_each(|Slot| {
		Slot.fetch_add(1, Ordering::Relaxed);
	});
}
