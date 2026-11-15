//! One stored response: when it was written, and the bytes that were served.
//!
//! The body is `Bytes` so a replay hands the response a refcounted handle instead
//! of copying the buffer, and so a cache insertion and a response body can share
//! one allocation. The timestamp is what freshness is classified from - it is
//! deliberately not a deadline, because the TTL is a property of the cache, not
//! of the entry.
//!
//! `pub` because it is the value type of a public field on `AppState`; the fields
//! stay crate-private, so only this module and the cache methods can build one.

use std::time::Instant;

use bytes::Bytes;

/// A cached response body and the instant it was stored.
pub struct Struct {
	/// When the entry was written.
	pub(crate) StoredAt:Instant,
	/// The serialized response body, exactly as it was served.
	pub(crate) Body:Bytes,
}
