//! Whether the caller asked for a streamed response.
//!
//! One value decides two things that must never disagree: which HTTP client
//! carries the request (`Buffered` uses the bounded client, `Streaming` the
//! client with no total timeout) and whether the response may be buffered and
//! cached at all.

/// The delivery mode the request asked for.
#[derive(Clone, Copy)]
pub(crate) enum Enum {
	/// A single buffered response body.
	Buffered,
	/// A server-sent-event stream, relayed chunk by chunk.
	Streaming,
}
