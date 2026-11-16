//! Attributing a resolve to the split event that produced it.
//!
//! `remove` is what makes the count honest: the hash leaves the map on its
//! first attribution, so an agent that re-reads the same segment five times
//! counts once. A `get`-then-increment would let a single popular marker
//! dominate the ratio and pin the threshold at its floor.

use crate::State::Struct::AphroditeState;

impl AphroditeState {
	/// Attribute a successful resolve to a split event (consequence signal).
	/// A hash counts at most once - removed from `SplitSegmentMap` after
	/// attribution, so re-retrieving the same marker never double-counts.
	/// Returns the event id when the hash belonged to a chain split.
	pub fn NoteSplitRetrieval(&mut self, hash:&str) -> Option<usize> {
		let Eid = self.SplitSegmentMap.remove(hash)?;
		// The linear scan below is bounded by SPLIT_EVENT_CAP (16) events -
		// deliberately not restructured (fix 7): an id→event map would need
		// an extra index kept in sync on ring eviction for at most 16
		// elements, so the scan is the simpler, equivalent choice. The
		// adaptation behavior this feeds is intentionally untouched.
		// The event found is the value: no match means no event to attribute, so
		// the increment runs only for the one that carries this id.
		self.SplitEvents.iter_mut().find(|e| e.ID == Eid).into_iter().for_each(|Event| {
			Event.Retrieved = Event.Retrieved.saturating_add(1);
		});
		self.AdaptChainSplitThreshold();
		Some(Eid)
	}
}
