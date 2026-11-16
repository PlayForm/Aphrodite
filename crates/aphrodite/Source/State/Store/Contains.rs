//! Non-promoting membership test for the inline store.

use super::Struct::Struct;

impl Struct {
	/// Non-promoting membership test for the inline store (P4/T12): unlike
	/// `Get`, this does NOT move the entry to the front, so the
	/// recall renderer can check resolvability without perturbing LRU order.
	pub fn Contains(&self, hash:&str) -> bool { self.Map.contains_key(hash) }
}
