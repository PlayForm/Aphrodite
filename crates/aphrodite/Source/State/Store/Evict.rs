//! The one eviction pass both inline-store bounds share.

use crate::State::{Const::InlineMaximum, Struct::AphroditeState};

impl AphroditeState {
	/// Evict the least-recently-used entry until both the entry-count cap
	/// (`InlineMaximum`) and the byte budget (`InlineStoreByteBudget`) are
	/// satisfied. `inline_order.pop_lru` removes the LRU in O(1); break when
	/// the order is empty (map and order are kept in sync, so this guards
	/// against divergence rather than a reachable state).
	pub(super) fn EvictOverBudget(&mut self) {
		loop {
			match self.InlineStore.len() > InlineMaximum
				|| (self.InlineStoreBytes > self.InlineStoreByteBudget && !self.InlineStore.is_empty())
			{
				true => {
					match self.InlineOrder.pop_lru() {
						Some((lru, _)) => {
							match self.InlineStore.remove(&lru) {
								Some(c) => {
									self.InlineStoreBytes = self.InlineStoreBytes.saturating_sub(c.len());
								},
								None => (),
							}
						},
						None => break,
					}
				},
				false => break,
			}
		}
	}
}
