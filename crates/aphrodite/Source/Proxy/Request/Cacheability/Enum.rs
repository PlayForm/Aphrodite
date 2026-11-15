//! Whether the request may be answered from, and written to, the response
//! cache - and under which key.
//!
//! `Cacheable` carries the key so a later stage cannot derive a second one: the
//! key is computed once, from the same parsed body the delivery mode came from.
//! `Uncacheable` is one variant rather than a `None` beside an `Option<u64>`,
//! because "no key" and "not a candidate" are the same answer to every
//! consumer.

/// The request's cache standing.
pub(crate) enum Enum {
	/// A cache candidate, keyed by the digest of its parameters.
	Cacheable { Key:u64 },
	/// Never cached: not a Chat Completions request, streamed, or unkeyable.
	Uncacheable,
}
