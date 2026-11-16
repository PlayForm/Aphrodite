//! The one eviction pass both inline-store bounds share.

use std::ops::ControlFlow;

use super::{Entry::Const::Cap, Struct::Struct};

/// Whether either bound is exceeded with something left to evict.
fn OverBudget(State:&Struct) -> bool { State.Map.len() > Cap || (State.Bytes > State.Budget && !State.Map.is_empty()) }

/// One eviction step.
///
/// The popped entry is the value: `Some` means a bound was exceeded and the LRU
/// left the order, `None` means the pass is finished - both bounds satisfied, or
/// the order ran dry. `Break` carries the finish because it is not a failure.
fn Step(State:&mut Struct) -> ControlFlow<()> {
	let Over = OverBudget(State);

	match Over.then(|| State.Order.pop_lru()).flatten() {
		Some((lru, _)) => {
			let Evicted = State.Map.remove(&lru).map_or(0, |c| c.len());

			State.Bytes = State.Bytes.saturating_sub(Evicted);

			ControlFlow::Continue(())
		},
		None => ControlFlow::Break(()),
	}
}

impl Struct {
	/// Evict the least-recently-used entry until both the entry-count cap
	/// (`Entry::Const::Cap`) and the byte budget (`Store::Budget`) are
	/// satisfied. `inline_order.pop_lru` removes the LRU in O(1); the pass ends
	/// when the order is empty (map and order are kept in sync, so this guards
	/// against divergence rather than a reachable state).
	pub(super) fn Evict(&mut self) {
		let Passes = self.Order.len();

		// One entry leaves the order per step, so its length bounds the steps this
		// pass can take; `Break` ends it earlier when the bounds are satisfied.
		let _ = (0..Passes).try_for_each(|_| Step(self));
	}
}
