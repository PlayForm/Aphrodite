//! A cached entry's standing against the clock.
//!
//! Two cases, no third: an entry is either servable or it is not. The
//! classification carries no side effect, which is what lets `Lookup` decide
//! whether to evict and whether to promote without the classifier touching the
//! LRU behind its back.

/// Whether a stored entry is still servable.
pub(crate) enum Enum {
	/// Inside the TTL - servable, and promoting it keeps the LRU order honest.
	Fresh,
	/// At or past the TTL - not servable, and left for the writer to replace.
	Expired,
}
