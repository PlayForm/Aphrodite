//! The inline store's seed: an empty map, its LRU order, and both bounds.

use super::{Const::Budget, Entry::Const::Cap, Struct::Struct};

impl Default for Struct {
	fn default() -> Self {
		Self {
			Map:std::collections::HashMap::with_capacity(Cap),
			// +1 headroom: the LruCache must never self-evict before the
			// explicit evict loop enforces the entry cap and the byte budget.
			Order:lru::LruCache::new(std::num::NonZeroUsize::new(Cap + 1).unwrap()),
			Bytes:0,
			Budget,
		}
	}
}
