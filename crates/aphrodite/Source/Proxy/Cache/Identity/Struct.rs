//! The identity of a cacheable request: a digest to index by, and the canonical
//! bytes that decide equality.
//!
//! A digest is an index, not equality evidence, and the response cache used to
//! confuse the two: a 64-bit FNV match over a hand-listed eight-field subset of
//! the body was treated as proof that two requests were the same. `Identity`
//! carries both halves, so the cache can be keyed by the whole thing - `Key` is
//! what the map hashes on, `Fingerprint` is what the map compares. A digest
//! collision is then two entries in one bucket, not one request served another's
//! answer.
//!
//! `Hash` hashes `Key` alone and `Eq` compares both. That is the one contract
//! `Hash`/`Eq` impose - equal values hash equal - and it is why the expensive
//! part (comparing, and therefore retaining, the canonical request) happens only
//! on a bucket hit.
//!
//! Cost, stated plainly: an entry retains its request's canonical bytes for as
//! long as it is cached, so the response cache's memory is now bounded by the
//! request side as well as the response side. The alternative - a second digest
//! standing in for equality - would be the same confusion in a new place.

use bytes::Bytes;

use crate::Proxy::Cache::{Canonical::Fn::Fn as CanonicalBytes, Hash::Fn::Fn as Fnv1a64};

/// What makes two requests the same cacheable request.
#[derive(Clone)]
pub struct Struct {
	/// The digest the cache indexes by.
	Key:u64,
	/// The canonical request bytes equality is decided on.
	Fingerprint:Bytes,
}

impl Struct {
	/// The identity of a Chat Completions body under `APIKey`.
	///
	/// Total, not `Option`: the caller has already established that the body is
	/// a Chat Completions request, and a body of that shape always has an
	/// identity. An `Option` here would be a second place deciding a question
	/// `Request::Semantics` has answered.
	pub(crate) fn New(Body:&serde_json::Value, APIKey:&str) -> Self {
		let Fingerprint = Bytes::from(CanonicalBytes(Body, APIKey));

		Struct { Key:Fnv1a64(&Fingerprint), Fingerprint }
	}

	/// The digest the cache indexes by.
	pub(crate) fn Key(&self) -> u64 { self.Key }
}

impl std::hash::Hash for Struct {
	// Fully qualified: the trait is not in scope for method resolution inside its
	// own impl block, and `self.Key.hash(..)` would not resolve without an import.
	fn hash<H:std::hash::Hasher>(&self, State:&mut H) { std::hash::Hash::hash(&self.Key, State); }
}

impl PartialEq for Struct {
	fn eq(&self, Other:&Self) -> bool { self.Key == Other.Key && self.Fingerprint == Other.Fingerprint }
}

impl Eq for Struct {}
