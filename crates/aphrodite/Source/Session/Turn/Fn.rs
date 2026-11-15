//! Advance the turn counter, and the cap the conversation index is bounded by.
//!
//! `NextTurn` also resets the message scan position: the scan index is a
//! per-turn cursor into the conversation, so leaving it set would make the next
//! turn start reading from the middle of the history.

use crate::State::Struct::AphroditeState;

/// Increment turn counter and return new value.
pub fn NextTurn(State:&mut AphroditeState) -> usize {
	State.TurnCounter += 1;
	State.ScannedMessageIndex = 0; // reset scan position for new turn
	State.TurnCounter
}

/// Cap on `ConvIndex` entries - the eviction bound enforced by
/// [`ArchiveTurn`]. Doubles as the capacity reference for the context
/// engine's fill ratio: the dylib has no token counts and no context
/// length, so the conversation index's own cap is the closest available
/// "full" reference (the index holds at most `ConvIndexCap` turns).
pub(crate) const ConvIndexCap:usize = 50;
