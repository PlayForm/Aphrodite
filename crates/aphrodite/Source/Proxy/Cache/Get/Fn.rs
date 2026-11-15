//! Read the response cache, honouring the TTL.
//!
//! A hit is only a hit while it is fresh; an expired entry is reported as a
//! miss and evicted so the writer replaces it. Returns the stored bytes as a
//! `Bytes` handle, so a replayed response shares the cached allocation instead
//! of copying it.
//!
//! The expiry probe uses `peek`, which deliberately does NOT promote recency -
//! an about-to-be-evicted expired entry must not be kept warm just because it
//! was inspected. Only a live hit calls `get`, which does promote, so the LRU
//! order reflects entries that were actually served.

use std::time::Instant;

use bytes::Bytes;

use crate::Proxy::{
	Cache::Freshness::{Classify::Fn::Fn as ClassifyFreshness, Enum::Enum as Freshness},
	State::Struct::Struct as AppState,
};

/// Look up `Key` in the response cache, treating an entry older than the
/// configured TTL as a miss and evicting it.
pub(crate) fn Fn(State:&AppState, Key:u64, Now:Instant) -> Option<Bytes> {
	State.ResponseCache.lock().ok().and_then(|mut Cache| {
		let Standing = Cache.peek(&Key).map_or(Freshness::Expired, |Entry| {
			ClassifyFreshness(Entry.StoredAt, Now, State.ResponseCacheTtl)
		});

		match Standing {
			Freshness::Expired => {
				Cache.pop(&Key);

				None
			},
			Freshness::Fresh => Cache.get(&Key).map(|Entry| Entry.Body.clone()),
		}
	})
}
