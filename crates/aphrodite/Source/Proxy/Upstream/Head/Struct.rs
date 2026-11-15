//! What the upstream said, before its body is touched.
//!
//! Both interpreters need the same three facts and both need them before the
//! body is consumed - the header map cannot be cloned from a `Response` whose
//! stream has been taken, and the status decides the counter and the client
//! status. Extracting the head once is what lets the responder branch exactly
//! once, on delivery shape, instead of re-reading the response in each arm.

use axum::http::{HeaderMap, HeaderValue, StatusCode};

/// The upstream response head.
pub(crate) struct Struct {
	/// The upstream status, passed through to the client unchanged.
	pub(crate) Status:StatusCode,
	/// The full upstream header map, copied onto the client response.
	pub(crate) Headers:HeaderMap,
	/// The upstream `content-type`, if it sent one.
	pub(crate) ContentType:Option<HeaderValue>,
}
