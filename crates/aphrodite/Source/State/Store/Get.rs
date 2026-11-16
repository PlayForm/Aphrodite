//! Reading from the inline store.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Retrieve from inline store with LRU promotion (O(1) hash lookup).
	///
	/// One map lookup answers the question; the promotion is a second,
	/// best-effort step that cannot change it. Testing `contains_key` and then
	/// looking the entry up again asked the map the same question twice, and the
	/// promotion is a no-op for a hash the store does not hold.
	pub fn Get(&mut self, hash:&str) -> Option<String> {
		// Promote to most-recent (O(1) `LruCache::get`, no retain scan -
		// fix 1) without cloning the content.
		let _ = self.InlineOrder.get(hash);

		self.InlineStore.get(hash).cloned()
	}
}
