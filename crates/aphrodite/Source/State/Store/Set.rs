//! Setting the inline store's byte budget.

use super::Struct::Struct;

impl Struct {
	/// Override the inline store's byte budget (e.g. from config); evicts
	/// immediately if the new budget is lower than the current usage.
	pub fn Set(&mut self, Budget:usize) {
		self.InlineStoreByteBudget = Budget;
		self.Evict();
	}
}
