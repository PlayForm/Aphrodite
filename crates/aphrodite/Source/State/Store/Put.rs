//! Inserting into the inline store.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Insert into inline store with LRU + byte-budget eviction (report 05
	/// F11: previously bounded by entry count only - `aphrodite_prefetch`
	/// admits files up to 10MB each and the ABI admits blobs up to 16MB, so
	/// 500 entries at the large end is a multi-GB worst case).
	pub fn InlineStorePut(&mut self, hash:String, Content:String) {
		// O(1) upsert: drop any prior entry so the running byte total stays
		// in sync, then re-insert at the most-recent position. `LruCache::put`
		// promotes an existing key in O(1) - the old O(n) `VecDeque::retain`
		// scan is gone (fix 1).
		match self.InlineStore.remove(&hash) {
			Some(Old) => {
				self.InlineStoreBytes = self.InlineStoreBytes.saturating_sub(Old.len());
			},
			None => (),
		}
		self.InlineStoreBytes += Content.len();
		self.InlineStore.insert(hash.clone(), Content);
		// Capacity INLINE_MAX + 1 guarantees this never self-evicts before
		// the explicit evict loop below.
		self.InlineOrder.put(hash, ());
		self.EvictOverBudget();
	}
}
