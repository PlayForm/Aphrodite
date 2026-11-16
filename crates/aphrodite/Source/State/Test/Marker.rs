//! The recent-marker catalog cap and its front-eviction arithmetic.

use crate::State::{
	Marker::{Const::Cap, Entry::Struct::Struct as Entry},
	Struct::AphroditeState,
};

#[test]
fn Eviction() {
	let mut s = AphroditeState::default();
	const Recorded:usize = 250;
	(0..Recorded).for_each(|i| {
		s.Marker.Record(Entry {
			hash:format!("h{}", i),
			CCRType:"text".into(),
			Size:100,
			Preview:"[text]".into(),
			Turn:i,
			Center:None,
			meta:None,
		});
	});
	assert!(s.Marker.Recent.len() <= Cap);
	// Eviction arithmetic: RECORDED - cap = 250 - 200 = 50 dropped from the
	// front; front eviction keeps index zero as the oldest survivor (h50).
	const Evicted:usize = Recorded - Cap;
	assert_eq!(s.Marker.Recent[0].hash, format!("h{Evicted}"));
}
