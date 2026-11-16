//! The inline store's byte budget and running total.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Current byte budget for the inline store (report 05 F11).
	pub fn Budget(&self) -> usize { self.InlineStoreByteBudget }
	/// Override the inline store's byte budget (e.g. from config); evicts
	/// immediately if the new budget is lower than the current usage.
	pub fn Set(&mut self, Budget:usize) {
		self.InlineStoreByteBudget = Budget;
		self.Evict();
	}
	/// Current total bytes held across every entry in the inline store.
	pub fn Bytes(&self) -> usize { self.InlineStoreBytes }
}
