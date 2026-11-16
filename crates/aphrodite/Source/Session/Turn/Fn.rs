//! Advance the turn counter.

use crate::State::Struct::AphroditeState;

/// Increment turn counter and return the new value.
///
/// The message scan position is reset with it: the scan index is a per-turn
/// cursor into the conversation, so leaving it set would make the next turn start
/// reading from the middle of the history.
pub fn Fn(State:&mut AphroditeState) -> usize {
	State.TurnCounter += 1;
	State.ScannedMessageIndex = 0;
	State.TurnCounter
}
