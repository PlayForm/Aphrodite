//! The Tier 1 teaching loop: attribution counted once, the ring bound, and
//! every adaptation branch (raise, hold, clamp at both ends, minimum events).

use crate::State::{
	Const::{SplitAdaptMinimumEvents, SplitEventCap},
	Struct::AphroditeState,
};

#[test]
fn TestRecordChainSplitRegistersHashesAndCountsRetrievalOnce() {
	let mut s = AphroditeState::default();
	let Hashes = vec!["h1".into(), "h2".into(), "h3".into()];
	s.RecordChainSplit(Hashes);
	assert_eq!(s.SplitEvents.len(), 1);
	assert_eq!(s.SplitEvents[0].Produced, 3);
	assert_eq!(s.SplitEvents[0].Retrieved, 0);

	// Retrieving a segment hash attributes the consequence once.
	assert_eq!(s.NoteSplitRetrieval("h2"), Some(1));
	assert_eq!(s.SplitEvents[0].Retrieved, 1);
	// Re-retrieving the same hash must not double-count.
	assert_eq!(s.NoteSplitRetrieval("h2"), None);
	assert_eq!(s.SplitEvents[0].Retrieved, 1);
	// Unknown hashes are not attributed.
	assert_eq!(s.NoteSplitRetrieval("nope"), None);
}

#[test]
fn TestSplitEventRingIsBounded() {
	let mut s = AphroditeState::default();
	(0..40).for_each(|i| {
		s.RecordChainSplit(vec![format!("h{i}")]);
	});
	assert!(s.SplitEvents.len() <= SplitEventCap);
	assert_eq!(s.SplitEvents.len(), SplitEventCap);
}

#[test]
fn TestAdaptRaisesThresholdWhenSegmentsIgnored() {
	let mut s = AphroditeState {
		ChainSplitMinimumSegments:2,
		ChainSplitFloor:2,
		ChainSplitMaximumSegments:6,
		..Default::default()
	};
	// 4 splits, all segments ignored: ratio 0/8 = 0.0 < 0.25 → raise.
	(0..4).for_each(|i| {
		s.RecordChainSplit(vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplitMinimumSegments, 3);
}

#[test]
fn TestAdaptKeepsThresholdWhenSegmentsRetrieved() {
	let mut s = AphroditeState {
		ChainSplitMinimumSegments:2,
		ChainSplitFloor:2,
		ChainSplitMaximumSegments:6,
		..Default::default()
	};
	// 4 splits, all segments retrieved: ratio 8/8 = 1.0 ≥ 0.5 → floor.
	(0..4).for_each(|i| {
		let Hashes = vec![format!("a{i}"), format!("b{i}")];
		s.RecordChainSplit(Hashes.clone());
		Hashes.into_iter().for_each(|h| {
			s.NoteSplitRetrieval(&h);
		});
	});
	assert_eq!(s.ChainSplitMinimumSegments, 2);
}

#[test]
fn TestAdaptRespectsBounds() {
	let mut s = AphroditeState {
		ChainSplitMinimumSegments:5,
		ChainSplitFloor:5,
		ChainSplitMaximumSegments:6,
		..Default::default()
	};
	// Ignored segments: threshold rises, but never past max.
	(0..20).for_each(|i| {
		s.RecordChainSplit(vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplitMinimumSegments, 6);

	// Fully-retrieved segments: falls, but never below floor.
	(20..60).for_each(|i| {
		let Hashes = vec![format!("a{i}"), format!("b{i}")];
		s.RecordChainSplit(Hashes.clone());
		Hashes.into_iter().for_each(|h| {
			s.NoteSplitRetrieval(&h);
		});
	});
	assert_eq!(s.ChainSplitMinimumSegments, 5);
}

#[test]
fn TestAdaptRequiresMinimumEvents() {
	let mut s = AphroditeState::default();
	// Fewer than SPLIT_ADAPT_MIN_EVENTS: no adaptation.
	(0..(SplitAdaptMinimumEvents - 1)).for_each(|i| {
		s.RecordChainSplit(vec![format!("a{i}"), format!("b{i}")]);
	});
	assert_eq!(s.ChainSplitMinimumSegments, 2);
}
