//! The two body shapes `/ccr/create` accepts.
//!
//! The endpoint documents both, and they differ in more than parsing: a JSON
//! create carries a caller-chosen key, TTL and tags and records a ratio sample
//! and a webhook, while a raw create is content only. Naming the two cases means
//! the later stages can branch once, on a value, instead of re-deriving which
//! branch they are in from the content type.

use crate::Proxy::CCR::Create::Request::Struct::Struct as CreateRequest;

/// A decoded `/ccr/create` body.
pub(crate) enum Enum {
	/// A structured request.
	Json(CreateRequest),
	/// The raw body, treated as the content itself.
	Opaque(String),
}
