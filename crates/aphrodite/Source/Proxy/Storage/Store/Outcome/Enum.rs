//! What a store attempt produced.
//!
//! `Failed` is a separate case from the two successes rather than a `None`
//! beside them: the caller has to count a failed write differently (it is a
//! miss that produced nothing), and folding it into the absence of an answer is
//! how a failed put came to look like a cache hit.

use crate::Proxy::Storage::Evidence::Struct::Struct as StoreEvidence;

/// The result of ensuring content is stored.
pub(crate) enum Enum {
	/// Already readable under the hash.
	Cached(StoreEvidence),
	/// Written now and readable.
	Stored(StoreEvidence),
	/// Neither readable nor writable - the content must not be replaced by a
	/// marker.
	Failed,
}
