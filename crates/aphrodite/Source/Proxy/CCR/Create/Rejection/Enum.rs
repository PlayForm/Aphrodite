//! The ways `/ccr/create` refuses a request.
//!
//! Each carries what the client needs to fix it, and each maps to exactly one
//! status. The three that used to be "return early with a JSON literal" are now
//! variants, so the renderer is the only place a status code is chosen.

/// Why the request was refused.
pub(crate) enum Enum {
	/// No CCR backend on this listener (e.g. token mode with `--no-ccr-marker`).
	NotEnabled,
	/// The backend refused the write.
	StoreFailed,
	/// The body claimed to be JSON and was not.
	InvalidJson(String),
	/// The body was not valid UTF-8 and could not be stored as content.
	InvalidUtf8,
}
