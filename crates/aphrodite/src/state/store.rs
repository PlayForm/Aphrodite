use super::{AphroditeState, INLINE_MAX};

impl AphroditeState {
	/// Current byte budget for the inline store (report 05 F11).
	pub fn inline_store_byte_budget(&self) -> usize { self.inline_store_byte_budget }
	/// Override the inline store's byte budget (e.g. from config); evicts
	/// immediately if the new budget is lower than the current usage.
	pub fn set_inline_store_byte_budget(&mut self, budget:usize) {
		self.inline_store_byte_budget = budget;
		self.evict_over_budget();
	}
	/// Current total bytes held across every entry in the inline store.
	pub fn inline_store_bytes(&self) -> usize { self.inline_store_bytes }
	/// Evict the least-recently-used entry until both the entry-count cap
	/// (`INLINE_MAX`) and the byte budget (`inline_store_byte_budget`) are
	/// satisfied. `inline_order.pop_lru` removes the LRU in O(1); break when
	/// the order is empty (map and order are kept in sync, so this guards
	/// against divergence rather than a reachable state).
	fn evict_over_budget(&mut self) {
		while self.inline_store.len() > INLINE_MAX
			|| (self.inline_store_bytes > self.inline_store_byte_budget && !self.inline_store.is_empty())
		{
			if let Some((lru, _)) = self.inline_order.pop_lru() {
				if let Some(c) = self.inline_store.remove(&lru) {
					self.inline_store_bytes = self.inline_store_bytes.saturating_sub(c.len());
				}
			} else {
				break;
			}
		}
	}
	/// Insert into inline store with LRU + byte-budget eviction (report 05
	/// F11: previously bounded by entry count only - `aphrodite_prefetch`
	/// admits files up to 10MB each and the ABI admits blobs up to 16MB, so
	/// 500 entries at the large end is a multi-GB worst case).
	pub fn inline_store_put(&mut self, hash:String, content:String) {
		// O(1) upsert: drop any prior entry so the running byte total stays
		// in sync, then re-insert at the most-recent position. `LruCache::put`
		// promotes an existing key in O(1) - the old O(n) `VecDeque::retain`
		// scan is gone (fix 1).
		if let Some(old) = self.inline_store.remove(&hash) {
			self.inline_store_bytes = self.inline_store_bytes.saturating_sub(old.len());
		}
		self.inline_store_bytes += content.len();
		self.inline_store.insert(hash.clone(), content);
		// Capacity INLINE_MAX + 1 guarantees this never self-evicts before
		// the explicit evict loop below.
		self.inline_order.put(hash, ());
		self.evict_over_budget();
	}
	/// Retrieve from inline store with LRU promotion (O(1) hash lookup).
	pub fn inline_store_get(&mut self, hash:&str) -> Option<String> {
		if self.inline_store.contains_key(hash) {
			// Promote to most-recent (O(1) `LruCache::get`, no retain scan -
			// fix 1) without cloning the content.
			let _ = self.inline_order.get(hash);
			self.inline_store.get(hash).cloned()
		} else {
			None
		}
	}
	/// Non-promoting membership test for the inline store (P4/T12): unlike
	/// `inline_store_get`, this does NOT move the entry to the front, so the
	/// recall renderer can check resolvability without perturbing LRU order.
	pub fn inline_store_contains(&self, hash:&str) -> bool { self.inline_store.contains_key(hash) }
}
