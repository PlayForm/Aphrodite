//! Reading from the inline store.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Retrieve from inline store with LRU promotion (O(1) hash lookup).
	pub fn InlineStoreGet(&mut self, hash:&str) -> Option<String> {
		if self.InlineStore.contains_key(hash) {
			// Promote to most-recent (O(1) `LruCache::get`, no retain scan -
			// fix 1) without cloning the content.
			let _ = self.InlineOrder.get(hash);
			self.InlineStore.get(hash).cloned()
		} else {
			None
		}
	}
}
