//! Session start: every per-session field returns to its configured default.

use crate::{Session::Start::Fn::Fn as Start, State::Struct::AphroditeState};

#[test]
fn Resets() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 42;
	s.RecordMarker(crate::State::Marker::Struct::Struct {
		hash:"abc".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});
	let r = Start(&mut s);
	assert_eq!(r["status"], "ok");
	assert_eq!(s.TurnCounter, 0);
	assert_eq!(s.RecentMarkers.len(), 0);
}

#[test]
fn Threshold() {
	let mut s = AphroditeState::default();
	// Simulate a previous session that adapted the threshold upward
	// (and a non-default configured floor, so the reset is proven to
	// restore the configured value, not a hardcoded constant).
	s.ChainSplitFloor = 3;
	s.ChainSplitMinimumSegments = 9;
	Start(&mut s);
	assert_eq!(s.ChainSplitMinimumSegments, s.ChainSplitFloor);
	assert_eq!(s.ChainSplitMinimumSegments, 3);
}

#[test]
fn Ledger() {
	// The ledger the threshold adapts FROM is per-conversation too: a carried-over
	// ledger would immediately move the threshold that was just reset to its floor.
	let mut s = AphroditeState::default();
	s.RecordSplit(vec!["a".into(), "b".into()]);
	assert_eq!(s.SplitEvents.len(), 1);
	assert_eq!(s.SplitSegmentMap.len(), 2);

	Start(&mut s);

	assert!(s.SplitEvents.is_empty(), "the consequence ledger is per-session");
	assert!(s.SplitSegmentMap.is_empty(), "no segment stays attributable across sessions");
	assert_eq!(s.SplitNextEventID, 0, "event ids restart with the session");
}

#[test]
fn Background() {
	// A backgrounded process outlives the session boundary, and its task record is
	// the poll worker's only handle on it - clearing it would strand a running
	// command. The expiry pass bounds the set instead.
	let mut s = AphroditeState::default();
	crate::PollWorker::Insert::Fn::Fn(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);

	Start(&mut s);

	assert_eq!(s.BgTasks.len(), 1, "a live background task must survive session start");
}

#[test]
fn Catalog() {
	let mut s = AphroditeState::default();
	s.LastEmittedMarkerCount = 10;
	s.LastEmittedFileCount = 5;
	Start(&mut s);
	assert_eq!(s.LastEmittedMarkerCount, 0);
	assert_eq!(s.LastEmittedFileCount, 0);
}
