//! Store a response body in the response cache.

use std::time::Instant;

use bytes::Bytes;

use crate::Proxy::{Cache::Entry::Struct::Struct as CacheEntry, State::Struct::Struct as AppState};

impl AppState {
	/// Insert `Body` under `Key`, best effort.
	///
	/// A poisoned cache lock drops the entry rather than failing a request that
	/// otherwise succeeded - observability and caching must never turn a good
	/// answer into a 500. The caller decides *whether* to insert (success status,
	/// byte cap); this method only performs the insert.
	pub(crate) fn CachePut(&self, Key:u64, Body:Bytes, Now:Instant) {
		let _ = self
			.ResponseCache
			.lock()
			.map(|mut Cache| Cache.put(Key, CacheEntry { StoredAt:Now, Body }));
	}
}
