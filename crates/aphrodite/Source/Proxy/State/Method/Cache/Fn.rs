//! Store a response body in the response cache.

use std::time::Instant;

use bytes::Bytes;

use crate::Proxy::{
	Cache::{Entry::Struct::Struct as CacheEntry, Identity::Struct::Struct as CacheIdentity},
	State::Struct::Struct as AppState,
};

impl AppState {
	/// Insert `Body` under `Identity`, best effort.
	///
	/// The entry is keyed by the identity itself, so the digest indexes it and the
	/// canonical request bytes decide whether a later lookup is the same request -
	/// storing the identity here is what makes that comparison possible, and
	/// storing only its digest is what made a digest match look like equality.
	///
	/// A poisoned cache lock drops the entry rather than failing a request that
	/// otherwise succeeded - observability and caching must never turn a good
	/// answer into a 500. The caller decides *whether* to insert (success status,
	/// byte cap); this method only performs the insert.
	pub(crate) fn CachePut(&self, Identity:CacheIdentity, Body:Bytes, Now:Instant) {
		let _ = self
			.ResponseCache
			.lock()
			.map(|mut Cache| Cache.put(Identity, CacheEntry { StoredAt:Now, Body }));
	}
}
