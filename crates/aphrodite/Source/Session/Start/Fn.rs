//! Reset every per-session field to its start-of-session value.
//!
//! The state is process-global and outlives a session, so anything left behind
//! here leaks into the next one: a stale turn counter suppresses the first-turn
//! orientation, a stale marker count makes the recall catalog report "no
//! change" on a session that has never emitted, and an adapted chain-split
//! threshold carries one conversation's shape into another.

use crate::State::AphroditeState;

/// Handle session start - reset all per-session state.
pub fn OnSessionStart(State:&mut AphroditeState) -> serde_json::Value {
	State.TurnCounter = 0;
	State.ScannedMessageIndex = 0;
	State.ConvIndex.clear();
	State.RecentMarkers.clear();
	State.ReferencedFiles.clear();
	State.ToolEvents.clear();
	State.EphemeralDirectives.clear();
	State.ManualDirectiveTurn = None;
	State.LastEmittedMarkerCount = 0;
	State.LastEmittedFileCount = 0;
	// Reset the adaptive chain-split threshold to the configured floor (the
	// initial value) so a threshold adapted in a previous session doesn't
	// leak into this one. Uses the configured floor, not a hardcoded number.
	State.ChainSplitMinimumSegments = State.ChainSplitFloor;

	serde_json::json!({
		"status": "ok",
		"version": env!("CARGO_PKG_VERSION"),
		"engine": "aphrodite-ffi",
		"turn": 0,
		"inline_entries": State.InlineStore.len(),
	})
}
