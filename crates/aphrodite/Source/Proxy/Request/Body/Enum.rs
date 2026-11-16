//! The request body's structural kind.
//!
//! Deliberately tolerant: an arbitrary proxy body is not required to be a Chat
//! Completions request, so "valid JSON of another shape" and "not JSON at all"
//! are first-class answers instead of parse failures forced into a
//! chat-completion schema.
//!
//! The kind carries no payload. It used to carry the parsed `Value` for
//! `ChatCompletion`, which kept the entire request JSON alive in
//! `Request::Context` for the whole request - long after the only consumer (the
//! cache identity, derived inside `Request::Semantics::Fn`) had finished with it.
//! The interpretation is temporary: what travels onwards is the answer, not the
//! input.

/// The body's structural kind.
pub(crate) enum Enum {
	/// A Chat Completions request: a string `model` and a `messages` member.
	ChatCompletion,
	/// Valid JSON of any other shape - forwarded verbatim, never keyed.
	Json,
	/// Not JSON - forwarded verbatim, never keyed, never compressed.
	Opaque,
}

impl Enum {
	/// A short name for the dev-mode request log.
	pub(crate) fn Kind(&self) -> &'static str {
		match self {
			Enum::ChatCompletion => "chat_completion",
			Enum::Json => "json",
			Enum::Opaque => "opaque",
		}
	}
}
