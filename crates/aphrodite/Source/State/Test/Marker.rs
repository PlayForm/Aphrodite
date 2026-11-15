//! The recent-marker catalog cap and its front-eviction arithmetic.

use crate::State::{AphroditeState, Const::RecentMarkersCap, MarkerEntry};

#[test]
fn TestRecordMarkerEviction() {
	let mut s = AphroditeState::default();
	const Recorded:usize = 250;
	for i in 0..Recorded {
		s.RecordMarker(MarkerEntry {
			hash:format!("h{}", i),
			CCRType:"text".into(),
			Size:100,
			Preview:"[text]".into(),
			Turn:i,
			Center:None,
			meta:None,
		});
	}
	assert!(s.RecentMarkers.len() <= RecentMarkersCap);
	// Eviction arithmetic: RECORDED - cap = 250 - 200 = 50 dropped from the
	// front; front eviction keeps index zero as the oldest survivor (h50).
	const Evicted:usize = Recorded - RecentMarkersCap;
	assert_eq!(s.RecentMarkers[0].hash, format!("h{Evicted}"));
}
