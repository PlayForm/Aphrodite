//! The two things a webhook delivery can do.
//!
//! `notify_success` and `notify_failure` are the metrics, so the delivery has to
//! answer with one of them - a boolean would work only as long as nobody added a
//! third case (a redirect loop, a DNS failure) that is neither.

/// A webhook delivery's result.
pub(crate) enum Enum {
	/// The subscriber accepted it.
	Delivered,
	/// The request failed or the subscriber refused it.
	Failed,
}
