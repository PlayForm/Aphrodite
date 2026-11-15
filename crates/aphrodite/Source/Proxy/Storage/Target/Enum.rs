//! The two places content can live.
//!
//! Content below the inline cutoff is kept in a process-memory LRU so a
//! retrieval needs no backend round-trip; above it, the durable backend is the
//! only copy. The two have different failure semantics - an inline insert cannot
//! fail, a durable one can - so the choice is a value the caller matches rather
//! than a length comparison it re-derives.

/// Where the content is stored.
pub(crate) enum Enum {
	/// The inline map, and best-effort in the durable backend too.
	Inline,
	/// The durable backend, where a failed write is the caller's problem.
	Durable,
}
