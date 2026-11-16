//! The one eviction pass both inline-store bounds share.

use std::ops::ControlFlow;

use crate::State::{Const::InlineMaximum, Struct::AphroditeState};

/// Whether either bound is exceeded with something left to evict.
fn OverBudget(State:&AphroditeState) -> bool {
	State.InlineStore.len() > InlineMaximum
		|| (State.InlineStoreBytes > State.InlineStoreByteBudget && !State.InlineStore.is_empty())
}

/// One eviction step.
///
/// The popped entry is the value: `Some` means a bound was exceeded and the LRU
/// left the order, `None` means the pass is finished - both bounds satisfied, or
/// the order ran dry. `Break` carries the finish because it is not a failure.
fn Step(State:&mut AphroditeState) -> ControlFlow<()> {
	let Over = OverBudget(State);

	match Over.then(|| State.InlineOrder.pop_lru()).flatten() {
		Some((lru, _)) => {
			let Evicted = State.InlineStore.remove(&lru).map_or(0, |c| c.len());

			State.InlineStoreBytes = State.InlineStoreBytes.saturating_sub(Evicted);

			ControlFlow::Continue(())
		},
		None => ControlFlow::Break(()),
	}
}

impl AphroditeState {
	/// Evict the least-recently-used entry until both the entry-count cap
	/// (`InlineMaximum`) and the byte budget (`InlineStoreByteBudget`) are
	/// satisfied. `inline_order.pop_lru` removes the LRU in O(1); the pass ends
	/// when the order is empty (map and order are kept in sync, so this guards
	/// against divergence rather than a reachable state).
	pub(super) fn Evict(&mut self) {
		let Passes = self.InlineOrder.len();

		// One entry leaves the order per step, so its length bounds the steps this
		// pass can take; `Break` ends it earlier when the bounds are satisfied.
		let _ = (0..Passes).try_for_each(|_| Step(self));
	}
}
