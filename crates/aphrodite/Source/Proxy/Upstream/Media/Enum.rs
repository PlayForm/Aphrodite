//! The upstream body's delivery shape.
//!
//! A boolean would answer the same question, but the responder's branch is the
//! single most consequential decision in the pipeline - one side must never be
//! buffered, the other must never be streamed - so the two cases are named and
//! matched rather than tested.

/// How the upstream body arrives.
pub(crate) enum Enum {
	/// A server-sent-event stream: relayed chunk by chunk, never buffered,
	/// never cached, never compressed.
	EventStream,
	/// A single buffered body, accumulated under a hard byte cap.
	Buffered,
}
