//! The inline content store's own fields.
//!
//! Split out of `AphroditeState` so a function that only touches the store can
//! take `&mut Store::Struct` instead of the whole session state. Inserting a
//! blob needs the map, its LRU order and the two bounds - none of the marker
//! catalog, the turn counter or the configuration the session also carries.
//!
//! Field comments are load-bearing: they record why a structure was chosen
//! (`HashMap` + `LruCache` over a `VecDeque`) and which bug a later change would
//! reintroduce by "simplifying" it.

use std::collections::HashMap;

/// The inline content store: `{hash: content}`, its LRU order, and the two
/// bounds the one eviction pass enforces.
///
/// `Clone` is derived because a session's state is cloned from the
/// config-seeded template (see `AphroditeState`); at seed time every store is
/// empty, so the clone is cheap.
#[derive(Clone)]
pub struct Struct {
	/// Inline content store: {hash: content}. The `HashMap` gives O(1)
	/// get/put/contains (was an O(n) `VecDeque` linear scan per op - bug
	/// 18-P14); `InlineOrder` preserves LRU recency so eviction can drop
	/// the least-recently-used entry in O(1).
	pub InlineStore:HashMap<String, String>,
	/// LRU recency order of `InlineStore` keys (unit values). Capacity is
	/// `Store::Entry::Const::Cap + 1` so the cache never self-evicts before
	/// the explicit `Evict` pass in `Put` (fix 1: previously a
	/// `VecDeque` whose promotion/upsert needed an O(n) `retain` scan).
	pub InlineOrder:lru::LruCache<String, ()>,
	/// Running total of `content.len()` across every entry in `InlineStore`,
	/// maintained incrementally by `Put` so eviction doesn't
	/// need an O(n) rescan on every insert (report 05 F11).
	pub InlineStoreBytes:usize,
	/// Byte budget for `InlineStore`; entries are evicted from the back
	/// (oldest/least-recently-used) until the running total is at or under
	/// this, in addition to the existing entry-count cap
	/// (`Store::Entry::Const::Cap`). Defaults to `Store::Const::Budget`; see
	/// `Set`.
	pub InlineStoreByteBudget:usize,
}
