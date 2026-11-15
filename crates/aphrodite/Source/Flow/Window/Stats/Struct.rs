//! Aggregate over the last `n` turns of recorded tool events.
//!
//! Every field is a count, not a rate, and `NewFiles` counts distinct written
//! paths inside the window - a file rewritten three times is one new file.
//! `DistinctErrorSigs` is the only field that depends on hashing, which is
//! why it is computed from `ErrorSig` rather than from message text.

/// Cheap aggregate over the last `n` turns of `ToolEvents` (P2/T7). Scans
/// events with `turn > turn_counter - n`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowStats {
	pub Reads:usize,
	pub Writes:usize,
	pub Searches:usize,
	pub Errors:usize,
	pub DistinctErrorSigs:usize,
	pub NewFiles:usize,
	pub TotalCalls:usize,
}
