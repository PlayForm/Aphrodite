//! Find the content behind a hash: inline store first, then the CCR backend.

use std::sync::atomic::Ordering;

use crate::{
	Proxy::State::{CCR::Get::Fn::Fn as CCRGet, Struct::Struct as AppState},
	Retrieve::Handle::Failure,
};

/// Resolve `hash` to its content, or name the failure.
///
/// Two stores, one order, and the order is the contract: the inline store holds
/// this session's own entries and is checked first, so a resolve never pays a
/// backend round trip for content the process already has. Both a hit and a miss
/// are counted per store, which is what makes the hit-rate observable.
///
/// The mutex guard is taken in its own block and dropped before the backend call.
/// That is not tidiness: a `MutexGuard` held across an `.await` is a lock held
/// across a suspension point, which is a deadlock risk in any runtime. The
/// poisoned case is a failure value rather than a cache miss - the old
/// `.lock().ok().and_then(...)` turned a broken process into a 404, so every
/// request fell through to the backend and callers went looking for entries that
/// were never missing (Fix 20).
///
/// The two closures of `map_or_else` are what keep the counters honest: the miss
/// counter runs only when the backend answered nothing, and the hit counter only
/// when it answered. Neither is a boolean the caller has to interpret.
pub(crate) async fn Fn(State:&AppState, hash:&str) -> Result<String, Failure::Enum::Enum> {
	let InlineHit = {
		let mut Map = State.InlineCCR.lock().map_err(|_| Failure::Enum::Enum::LockPoisoned)?;

		// `get` takes `&mut self` on the cache: a hit promotes recency.
		Map.get(hash).cloned()
	};

	match InlineHit {
		Some(Cached) => {
			State.InlineCCRHits.fetch_add(1, Ordering::Relaxed);
			State.CCRHits.fetch_add(1, Ordering::Relaxed);

			Ok(Cached)
		},
		None => {
			State.InlineCCRMisses.fetch_add(1, Ordering::Relaxed);

			// The guard is out of scope, so this call may await.
			let Backend = match State.ccr.as_ref() {
				Some(ccr) => CCRGet(ccr, hash).await,
				None => None,
			};

			Backend.map_or_else(
				|| {
					State.CCRMisses.fetch_add(1, Ordering::Relaxed);

					Err(Failure::Enum::Enum::NotFound(hash.to_string()))
				},
				|Content| {
					State.CCRHits.fetch_add(1, Ordering::Relaxed);

					Ok(Content)
				},
			)
		},
	}
}
