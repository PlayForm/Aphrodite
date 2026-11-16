//! Aggregate over the last `n` turns of recorded tool events.
//!
//! Every field is a count, not a rate, and `NewFiles` counts distinct written
//! paths inside the window - a file rewritten three times is one new file.
//! `Distinct` is the only field that depends on hashing, which is
//! why it is computed from `ErrorSig` rather than from message text.

/// Cheap aggregate over the last `n` turns of `Event::Recent` (P2/T7). Scans
/// events with `turn > turn_counter - n`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Struct {
	pub Reads:usize,
	pub Writes:usize,
	pub Searches:usize,
	pub Errors:usize,
	/// Distinct error signatures in the window; the only field that depends on
	/// hashing, which is why it is counted from `Flow::Signature::Error::Fn`
	/// rather than from message text. Zero = every error had the same signature
	/// (or there were none).
	pub Distinct:usize,
	pub NewFiles:usize,
	pub Total:usize,
}
