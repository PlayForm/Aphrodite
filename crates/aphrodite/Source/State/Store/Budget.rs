//! The inline store's byte budget and running total.

use super::super::AphroditeState;

impl AphroditeState {
	/// Current byte budget for the inline store (report 05 F11).
	pub fn InlineStoreByteBudget(&self) -> usize { self.InlineStoreByteBudget }
	/// Override the inline store's byte budget (e.g. from config); evicts
	/// immediately if the new budget is lower than the current usage.
	pub fn SetInlineStoreByteBudget(&mut self, Budget:usize) {
		self.InlineStoreByteBudget = Budget;
		self.EvictOverBudget();
	}
	/// Current total bytes held across every entry in the inline store.
	pub fn InlineStoreBytes(&self) -> usize { self.InlineStoreBytes }
}
