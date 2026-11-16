//! Session start: every per-session field returns to its configured default.

use crate::{Session::Start::Fn::Fn as Start, State::Struct::AphroditeState};

#[test]
fn Resets() {
	let mut s = AphroditeState::default();
	s.Turn.Counter = 42;
	s.Marker.Record(crate::State::Marker::Entry::Struct::Struct {
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
	assert_eq!(s.Turn.Counter, 0);
	assert_eq!(s.Marker.Recent.len(), 0);
}

#[test]
fn Threshold() {
	let mut s = AphroditeState::default();
	// Simulate a previous session that adapted the threshold upward
	// (and a non-default configured floor, so the reset is proven to
	// restore the configured value, not a hardcoded constant).
	s.ChainSplit.Floor = 3;
	s.ChainSplit.Minimum = 9;
	Start(&mut s);
	assert_eq!(s.ChainSplit.Minimum, s.ChainSplit.Floor);
	assert_eq!(s.ChainSplit.Minimum, 3);
}

#[test]
fn Ledger() {
	// The ledger the threshold adapts FROM is per-conversation too: a carried-over
	// ledger would immediately move the threshold that was just reset to its floor.
	let mut s = AphroditeState::default();
	s.ChainSplit.Record(s.Turn.Counter, vec!["a".into(), "b".into()]);
	assert_eq!(s.ChainSplit.Ledger.len(), 1);
	assert_eq!(s.ChainSplit.Segments.len(), 2);

	Start(&mut s);

	assert!(s.ChainSplit.Ledger.is_empty(), "the consequence ledger is per-session");
	assert!(
		s.ChainSplit.Segments.is_empty(),
		"no segment stays attributable across sessions"
	);
	assert_eq!(s.ChainSplit.Next, 0, "event ids restart with the session");
}

#[test]
fn Background() {
	// A backgrounded process outlives the session boundary, and its task record is
	// the poll worker's only handle on it - clearing it would strand a running
	// command. The expiry pass bounds the set instead.
	let mut s = AphroditeState::default();
	crate::PollWorker::Insert::Fn::Fn(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);

	Start(&mut s);

	assert_eq!(s.PollWorker.Tasks.len(), 1, "a live background task must survive session start");
}

#[test]
fn Catalog() {
	let mut s = AphroditeState::default();
	s.Marker.Emitted = 10;
	s.File.Emitted = 5;
	Start(&mut s);
	assert_eq!(s.Marker.Emitted, 0);
	assert_eq!(s.File.Emitted, 0);
}
