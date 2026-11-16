//! Whether the request may be answered from, and written to, the response
//! cache - and under which identity.
//!
//! `Cacheable` carries the whole identity, not a bare digest: the digest only
//! indexes the cache, so a stage handed just the digest would have nothing to
//! compare against and would end up treating a bucket match as equality - which
//! is how a request differing only in `max_tokens` or `stop` was served another
//! request's answer. `Uncacheable` is one variant rather than a `None` beside an
//! `Option<...>`, because "no identity" and "not a candidate" are the same answer
//! to every consumer.

use crate::Proxy::Cache::Identity::Struct::Struct as CacheIdentity;

/// The request's cache standing.
pub(crate) enum Enum {
	/// A cache candidate, identified by its canonical request bytes.
	Cacheable { Identity:CacheIdentity },
	/// Never cached: not a Chat Completions request, streamed, or another route.
	Uncacheable,
}
