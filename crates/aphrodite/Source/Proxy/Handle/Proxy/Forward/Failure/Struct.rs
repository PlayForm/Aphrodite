//! A failed upstream call.
//!
//! The old shape was `(anyhow::Result<Response>, bool)`, where the boolean said
//! whether the final failure was a timeout - so `(Ok(Response), true)` was
//! representable and meaningless, and the loop seeded the result with a
//! fabricated `"unreachable"` error because it had no way to say "no outcome
//! yet". A struct with a kind, an attempt count and the source says the same
//! thing without the illegal combination and without the placeholder.

use crate::Proxy::Handle::Proxy::Forward::Failure::Kind::Enum::Enum as FailureKind;

/// One terminal upstream failure.
pub(crate) struct Struct {
	/// Which class of failure ended the loop.
	pub(crate) Kind:FailureKind,
	/// How many attempts were made before giving up.
	pub(crate) Attempts:u32,
	/// The transport error, with the URL as context.
	pub(crate) Source:anyhow::Error,
}
