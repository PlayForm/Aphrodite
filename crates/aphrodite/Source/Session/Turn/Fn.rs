//! Advance the turn counter.

use crate::State::Turn::Struct::Struct as Turn;

/// Increment turn counter and return the new value.
///
/// Takes the turn bookkeeping rather than the session state: advancing a turn is
/// a counter and a cursor, and handing it the whole state would let a future
/// edit reach the store or the rings from here.
///
/// The message scan position is reset with it: the scan index is a per-turn
/// cursor into the conversation, so leaving it set would make the next turn start
/// reading from the middle of the history.
pub fn Fn(Turn:&mut Turn) -> usize {
	Turn.Counter += 1;
	Turn.Scanned = 0;
	Turn.Counter
}
