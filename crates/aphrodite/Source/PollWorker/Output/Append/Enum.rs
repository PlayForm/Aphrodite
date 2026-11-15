//! What one append did to the accumulator.

/// The outcome of appending a poll payload.
///
/// Three cases and no fourth: the payload fitted whole, it was cut to the room
/// that was left, or there was no room to begin with. The caller does not need
/// to act on this - the buffer is already correct - but it is the honest answer
/// to "did the model get the whole poll", which a bare length could not give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Append {
	/// Every byte of the payload was kept.
	Complete,
	/// The payload was cut to the remaining room; the tail was dropped and the
	/// accumulator is now at its bound.
	Truncated,
	/// Nothing was kept - the accumulator was already full.
	Full,
}
