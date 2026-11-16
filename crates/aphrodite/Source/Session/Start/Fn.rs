//! Reset every per-session field to its start-of-session value.
//!
//! The state is process-global and outlives a session, so anything left behind
//! here leaks into the next one: a stale turn counter suppresses the first-turn
//! orientation, a stale marker count makes the recall catalog report "no
//! change" on a session that has never emitted, and an adapted chain-split
//! threshold carries one conversation's shape into another.

use crate::State::Struct::AphroditeState;

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
	// The consequence ledger the threshold adapts FROM is per-conversation for the
	// same reason the threshold is: it records what this conversation's agent did
	// with the markers it was shown. A ledger carried over from another session
	// would move a threshold that was just reset to its floor.
	//
	// `BgTasks` is deliberately NOT reset here. A backgrounded process outlives the
	// session boundary, and its task record is the only handle the poll worker has
	// on it; the expiry pass is what bounds that set, and clearing it would strand a
	// running command with no way to report its output.
	State.SplitEvents.clear();
	State.SplitSegmentMap.clear();
	State.SplitNextEventID = 0;

	serde_json::json!({
		"status": "ok",
		"version": env!("CARGO_PKG_VERSION"),
		"engine": "aphrodite-ffi",
		"turn": 0,
		"inline_entries": State.InlineStore.len(),
	})
}
