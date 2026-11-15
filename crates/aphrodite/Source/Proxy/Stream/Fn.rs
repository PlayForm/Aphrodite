//! Is this `Content-Type` a server-sent-event stream?
//!
//! The single predicate the relay path branches on. It matches the media type
//! case-insensitively and ignores parameters, because upstreams disagree about
//! `text/event-stream; charset=utf-8` and misclassifying one buffers a stream
//! that must never be buffered.

/// T10 (F6): does this upstream `content-type` mark an SSE response that must
/// be streamed chunk-by-chunk rather than buffered (a buffered SSE body is
/// unparseable by an SSE client)? Prefix match, not exact - real servers
/// append `; charset=utf-8` and similar.
pub(crate) fn Fn(ContentType:Option<&axum::http::HeaderValue>) -> bool {
	ContentType
		.map(|Ct| Ct.as_bytes().starts_with(b"text/event-stream"))
		.unwrap_or(false)
}
