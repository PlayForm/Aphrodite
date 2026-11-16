//! Inserting into the inline store.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Insert into inline store with LRU + byte-budget eviction (report 05
	/// F11: previously bounded by entry count only - `aphrodite_prefetch`
	/// admits files up to 10MB each and the ABI admits blobs up to 16MB, so
	/// 500 entries at the large end is a multi-GB worst case).
	pub fn Put(&mut self, hash:String, Content:String) {
		// O(1) upsert: drop any prior entry so the running byte total stays
		// in sync, then re-insert at the most-recent position. `LruCache::put`
		// promotes an existing key in O(1) - the old O(n) `VecDeque::retain`
		// scan is gone (fix 1).
		// The prior entry decides the subtraction: `None` contributes zero bytes
		// rather than a branch, so the running total is corrected in one place.
		let Previous = self.InlineStore.remove(&hash).map_or(0, |Old| Old.len());

		self.InlineStoreBytes = self.InlineStoreBytes.saturating_sub(Previous);
		self.InlineStoreBytes += Content.len();
		self.InlineStore.insert(hash.clone(), Content);
		// Capacity INLINE_MAX + 1 guarantees this never self-evicts before
		// the explicit evict loop below.
		self.InlineOrder.put(hash, ());
		self.Evict();
	}
}
