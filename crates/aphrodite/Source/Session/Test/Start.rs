//! Session start: every per-session field returns to its configured default.

use crate::{Session::OnSessionStart, State::AphroditeState};

#[test]
fn TestSessionStartResets() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 42;
	s.RecordMarker(crate::State::MarkerEntry {
		hash:"abc".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});
	let r = OnSessionStart(&mut s);
	assert_eq!(r["status"], "ok");
	assert_eq!(s.TurnCounter, 0);
	assert_eq!(s.RecentMarkers.len(), 0);
}

#[test]
fn TestSessionStartResetsChainSplitThreshold() {
	let mut s = AphroditeState::default();
	// Simulate a previous session that adapted the threshold upward
	// (and a non-default configured floor, so the reset is proven to
	// restore the configured value, not a hardcoded constant).
	s.ChainSplitFloor = 3;
	s.ChainSplitMinimumSegments = 9;
	OnSessionStart(&mut s);
	assert_eq!(s.ChainSplitMinimumSegments, s.ChainSplitFloor);
	assert_eq!(s.ChainSplitMinimumSegments, 3);
}

#[test]
fn TestCatalogSummaryResetOnSessionStart() {
	let mut s = AphroditeState::default();
	s.LastEmittedMarkerCount = 10;
	s.LastEmittedFileCount = 5;
	OnSessionStart(&mut s);
	assert_eq!(s.LastEmittedMarkerCount, 0);
	assert_eq!(s.LastEmittedFileCount, 0);
}
