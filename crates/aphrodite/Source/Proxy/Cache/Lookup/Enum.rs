//! What the response cache said about this request.
//!
//! Three answers, not two. The old `Option<Response>` collapsed "this request
//! was never a cache candidate" and "it was a candidate and nothing was stored"
//! into the same `None`, so the miss counter, the dev log and the cache-insert
//! decision all had to re-derive which case they were in from a separate
//! `Option<u64>`. A hit carries the rendered response because rendering it is
//! what a hit means; nothing downstream re-checks the bytes.

use axum::response::Response;

/// The cache's verdict.
pub(crate) enum Enum {
	/// Not a cache candidate at all - not a Chat Completions request, or
	/// streamed, or unkeyable. Nothing is counted and nothing is inserted.
	Bypassed,
	/// A candidate whose key held no fresh entry.
	Miss { Key:u64 },
	/// A fresh entry, already rendered as the client response.
	Hit(Response),
}
