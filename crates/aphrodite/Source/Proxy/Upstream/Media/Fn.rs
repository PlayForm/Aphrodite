//! Classify the upstream `Content-Type` as a stream or a buffered body.
//!
//! A prefix match, not equality: real servers append `; charset=utf-8` and
//! similar, and misclassifying one of those buffers a stream that an SSE client
//! can then not parse at all.

use axum::http::HeaderValue;

use crate::Proxy::Upstream::Media::Enum::Enum as UpstreamMedia;

/// Does this upstream `content-type` mark a server-sent-event stream?
pub(crate) fn Fn(ContentType:Option<&HeaderValue>) -> UpstreamMedia {
	ContentType
		.map(|Ct| Ct.as_bytes().starts_with(b"text/event-stream"))
		.filter(|EventStream| *EventStream)
		.map_or(UpstreamMedia::Buffered, |_| UpstreamMedia::EventStream)
}
