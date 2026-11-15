//! Read the response cache, honouring the TTL.
//!
//! A hit is only a hit while it is fresh; an expired entry is reported as a
//! miss and left for the writer to replace. Returns the stored bytes verbatim
//! so a replayed response is byte-identical to the original.

use crate::Proxy::State::Struct::Struct as AppState;

/// Look up `Ck` in `state.response_cache`, treating an entry older than
/// `state.response_cache_ttl` as a miss and evicting it (report 06 F5) -
/// without this, a marker minted at minute 0 of a long session gets its
/// cached response replayed unchanged at minute 90, silently diverging from
/// what a fresh (possibly temperature>0) upstream call would return.
///
/// W2-9: the expiry probe uses `peek`, which deliberately does NOT promote
/// recency (it is the lru crate's non-promoting read) - an about-to-be-
/// evicted expired entry is not kept warm just because it was inspected.
/// A live hit uses `get`, which DOES promote the entry to
/// most-recently-used, so the LRU eviction order stays honest and only
/// reflects entries that are actually served.
pub(crate) fn Fn(State:&AppState, Ck:u64) -> Option<Vec<u8>> {
	State.ResponseCache.lock().ok().and_then(|mut Cache| {
		let Expired = Cache
			.peek(&Ck)
			.map(|(InsertedAt, _)| InsertedAt.elapsed() >= State.ResponseCacheTtl)
			.unwrap_or(false);
		match Expired {
			true => {
				Cache.pop(&Ck);

				None
			},
			false => Cache.get(&Ck).map(|(_, body)| body.clone()),
		}
	})
}
