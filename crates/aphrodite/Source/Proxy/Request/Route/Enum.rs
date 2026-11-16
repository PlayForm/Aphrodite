//! The route a request took, as a value.
//!
//! The catch-all handler serves every path, so "is this the Chat Completions
//! endpoint" was a `bool` threaded through four functions and re-derived at
//! three of them. It is a two-case decision, so it is an enum, matched once
//! where the response is rendered.

/// The proxy route.
#[derive(Clone, Copy)]
pub(crate) enum Enum {
	/// `POST /v1/chat/completions` - the response is a completion envelope the
	/// compression pass understands.
	ChatCompletions,
	/// Any other path - forwarded and returned untouched.
	Other,
}
