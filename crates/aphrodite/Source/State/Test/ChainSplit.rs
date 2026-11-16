//! The Tier 1 teaching loop: attribution counted once, the ring bound, and
//! every adaptation branch (raise, hold, clamp at both ends, minimum events).

use crate::State::{
	ChainSplit::{Const::Cap, Struct::Struct as ChainSplit, Window},
	Struct::AphroditeState,
};

#[test]
fn Once() {
	let mut s = AphroditeState::default();
	let Hashes = vec!["h1".into(), "h2".into(), "h3".into()];
	s.ChainSplit.Record(s.Turn.Counter, Hashes);
	assert_eq!(s.ChainSplit.Ledger.len(), 1);
	assert_eq!(s.ChainSplit.Ledger[0].Produced, 3);
	assert_eq!(s.ChainSplit.Ledger[0].Retrieved, 0);

	// Retrieving a segment hash attributes the consequence once.
	assert_eq!(s.ChainSplit.Retrieve("h2"), Some(1));
	assert_eq!(s.ChainSplit.Ledger[0].Retrieved, 1);
	// Re-retrieving the same hash must not double-count.
	assert_eq!(s.ChainSplit.Retrieve("h2"), None);
	assert_eq!(s.ChainSplit.Ledger[0].Retrieved, 1);
	// Unknown hashes are not attributed.
	assert_eq!(s.ChainSplit.Retrieve("nope"), None);
}

#[test]
fn Ring() {
	let mut s = AphroditeState::default();
	(0..40).for_each(|i| {
		s.ChainSplit.Record(s.Turn.Counter, vec![format!("h{i}")]);
	});
	assert!(s.ChainSplit.Ledger.len() <= Cap);
	assert_eq!(s.ChainSplit.Ledger.len(), Cap);
}

#[test]
fn Evicted() {
	// The ring is capped and the segment map used to be unbounded: an unretrieved
	// segment hash stayed addressable forever. Eviction has to remove exactly the
	// hashes of the events it drops - no more (a live event must stay attributable)
	// and no fewer (the map would grow again).
	let mut s = AphroditeState::default();
	let Recorded:usize = Cap * 2;

	(0..Recorded).for_each(|i| {
		s.ChainSplit.Record(s.Turn.Counter, vec![format!("segment-{i}")]);
	});

	assert_eq!(s.ChainSplit.Ledger.len(), Cap);
	assert_eq!(
		s.ChainSplit.Segments.len(),
		Cap,
		"the map holds one hash per live event, not one per split ever recorded"
	);

	// The newest events stay attributable...
	let Newest = Recorded - 1;
	assert_eq!(
		s.ChainSplit.Retrieve(&format!("segment-{Newest}")),
		Some(Recorded),
		"the newest event is still live and still attributes"
	);
	// ...and an evicted one no longer is.
	assert_eq!(
		s.ChainSplit.Retrieve("segment-0"),
		None,
		"an evicted event cannot be attributed"
	);
}

#[test]
fn Ignored() {
	let mut s = AphroditeState {
		ChainSplit:ChainSplit { Minimum:2, Floor:2, Maximum:6, ..Default::default() },
		..Default::default()
	};
	// 4 splits, all segments ignored: ratio 0/8 = 0.0 < 0.25 → raise.
	(0..4).for_each(|i| {
		s.ChainSplit.Record(s.Turn.Counter, vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplit.Minimum, 3);
}

#[test]
fn Retrieved() {
	let mut s = AphroditeState {
		ChainSplit:ChainSplit { Minimum:2, Floor:2, Maximum:6, ..Default::default() },
		..Default::default()
	};
	// 4 splits, all segments retrieved: ratio 8/8 = 1.0 ≥ 0.5 → floor.
	(0..4).for_each(|i| {
		let Hashes = vec![format!("a{i}"), format!("b{i}")];
		s.ChainSplit.Record(s.Turn.Counter, Hashes.clone());
		Hashes.into_iter().for_each(|h| {
			s.ChainSplit.Retrieve(&h);
		});
	});
	assert_eq!(s.ChainSplit.Minimum, 2);
}

#[test]
fn Bounds() {
	let mut s = AphroditeState {
		ChainSplit:ChainSplit { Minimum:5, Floor:5, Maximum:6, ..Default::default() },
		..Default::default()
	};
	// Ignored segments: threshold rises, but never past max.
	(0..20).for_each(|i| {
		s.ChainSplit.Record(s.Turn.Counter, vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplit.Minimum, 6);

	// Fully-retrieved segments: falls, but never below floor.
	(20..60).for_each(|i| {
		let Hashes = vec![format!("a{i}"), format!("b{i}")];
		s.ChainSplit.Record(s.Turn.Counter, Hashes.clone());
		Hashes.into_iter().for_each(|h| {
			s.ChainSplit.Retrieve(&h);
		});
	});
	assert_eq!(s.ChainSplit.Minimum, 5);
}

#[test]
fn Minimum() {
	let mut s = AphroditeState::default();
	// Fewer than `Window::Const::Minimum`: no adaptation.
	(0..(Window::Const::Minimum - 1)).for_each(|i| {
		s.ChainSplit.Record(s.Turn.Counter, vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplit.Minimum, 2);
}
