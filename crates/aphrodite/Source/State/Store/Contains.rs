//! Non-promoting membership test for the inline store.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Non-promoting membership test for the inline store (P4/T12): unlike
	/// `InlineStoreGet`, this does NOT move the entry to the front, so the
	/// recall renderer can check resolvability without perturbing LRU order.
	pub fn InlineStoreContains(&self, hash:&str) -> bool { self.InlineStore.contains_key(hash) }
}
