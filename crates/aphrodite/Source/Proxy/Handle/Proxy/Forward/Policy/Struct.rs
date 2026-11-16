//! The bounds the retry loop obeys.
//!
//! A policy value rather than three literals inside the loop: the numbers are
//! the safety contract (a non-idempotent `POST /v1/chat/completions` must not be
//! resent more than twice), and a reader deciding whether to change them should
//! find them in one place with the reason attached.

/// How many attempts, how long between them, and how much to jitter.
pub(crate) struct Struct {
	/// Total attempts, including the first. Three means at most two retries.
	pub(crate) MaximumAttempts:u32,
	/// First backoff in milliseconds, doubled per attempt.
	pub(crate) Backoff:u64,
	/// Lowest jitter multiplier - 0.75 keeps a retry from landing on the same
	/// millisecond as a peer's.
	pub(crate) JitterFloor:f64,
	/// Width of the jitter window above the floor; the multiplier is uniform in
	/// `[JitterFloor, JitterFloor + JitterSpan)`.
	pub(crate) JitterSpan:f64,
}
