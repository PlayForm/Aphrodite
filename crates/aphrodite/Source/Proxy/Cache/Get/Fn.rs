//! Read the response cache, honouring the TTL.
//!
//! A hit is only a hit while it is fresh; an expired entry is reported as a
//! miss and evicted so the writer replaces it. Returns the stored bytes as a
//! `Bytes` handle, so a replayed response shares the cached allocation instead
//! of copying it.
//!
//! The lookup is by IDENTITY, not by digest. `ResponseCache` is keyed by
//! `Cache::Identity`, whose `Hash` is the digest and whose `Eq` compares the
//! canonical request bytes - so "found" means the two requests really are the
//! same request, and a digest collision is two entries in one bucket rather than
//! one request answered with another's response.
//!
//! The expiry probe uses `peek`, which deliberately does NOT promote recency -
//! an about-to-be-evicted expired entry must not be kept warm just because it
//! was inspected. Only a live hit calls `get`, which does promote, so the LRU
//! order reflects entries that were actually served.

use std::time::Instant;

use bytes::Bytes;

use crate::Proxy::{
	Cache::{
		Freshness::{Classify::Fn::Fn as ClassifyFreshness, Enum::Enum as Freshness},
		Identity::Struct::Struct as CacheIdentity,
	},
	State::Struct::Struct as AppState,
};

/// Look up `Identity` in the response cache, treating an entry older than the
/// configured TTL as a miss and evicting it.
pub(crate) fn Fn(State:&AppState, Identity:&CacheIdentity, Now:Instant) -> Option<Bytes> {
	State.ResponseCache.lock().ok().and_then(|mut Cache| {
		let Standing = Cache.peek(Identity).map_or(Freshness::Expired, |Entry| {
			ClassifyFreshness(Entry.StoredAt, Now, State.ResponseCacheTtl)
		});

		match Standing {
			Freshness::Expired => {
				Cache.pop(Identity);

				None
			},
			Freshness::Fresh => Cache.get(Identity).map(|Entry| Entry.Body.clone()),
		}
	})
}
