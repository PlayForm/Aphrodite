//! One request, interpreted once.
//!
//! Produced by `Semantics::Fn::Fn` and consumed by every later stage: the
//! forwarding client comes from `Delivery`, the cache policy from
//! `Cacheability`, and the response-compression gate from `Route`. Nothing
//! downstream re-reads the body to answer any of those - which is what keeps one
//! request from being classified two ways.

use crate::Proxy::Request::{
	Body::Enum::Enum as RequestBody,
	Cacheability::Enum::Enum as RequestCacheability,
	Delivery::Enum::Enum as RequestDelivery,
	Route::Enum::Enum as RequestRoute,
};

/// The request's interpretation.
pub(crate) struct Struct {
	/// Which route the request took.
	pub(crate) Route:RequestRoute,
	/// The body's structural kind.
	pub(crate) Body:RequestBody,
	/// Whether the caller asked for a streamed response.
	pub(crate) Delivery:RequestDelivery,
	/// Whether the request may use the response cache, and under which key.
	pub(crate) Cacheability:RequestCacheability,
}
