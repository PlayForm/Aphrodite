//! Recording a chain-split event.
//!
//! The segment hashes must be registered in `SplitSegmentMap` before the
//! event is pushed, because attribution (`Retrieve.rs`) looks the hash up in
//! that map. Registering first and pushing second means an interruption can
//! only lose an event, never attribute a resolve to the wrong one.
//!
//! The reverse must hold too: when the ring evicts an event, that event's hashes
//! leave the map with it. The ring is capped at `SplitEventCap` and the map was
//! not, so a split whose segments were never retrieved left its hashes in the map
//! for the life of the process - the one unbounded structure in the state, growing
//! by every segment of every chain ever split.

use crate::State::{ChainSplit::Struct::SplitEvent, Const::SplitEventCap, Struct::AphroditeState};

impl AphroditeState {
	/// Record a chain-split event (Tier 1 teaching loop): a chain's output
	/// was split into `Produced` segment markers whose hashes are registered
	/// for retrieval attribution. Bounded ring (cap `SplitEventCap`), then
	/// re-adapts the split threshold from the retrieval ratio.
	///
	/// An empty batch records nothing - a plain guard rather than a combinator, because
	/// everything inside it mutates: the id, the map and the ring move together and a
	/// reader should see that as one block. It also keeps the event id from advancing for
	/// a split that produced no segments.
	///
	/// The ring bound is `drain` over the excess rather than a pop loop: the
	/// length is known, so the number to drop is arithmetic, and the eviction
	/// happens in one call at the front where the oldest events sit. Every
	/// evicted event takes its hashes out of `SplitSegmentMap`, so the map holds
	/// exactly the segment identities the ring can still explain.
	pub fn RecordChainSplit(&mut self, Hashes:Vec<String>) {
		if Hashes.is_empty() {
			return;
		}

		self.SplitNextEventID += 1;

		let ID = self.SplitNextEventID;
		let Produced = Hashes.len();

		Hashes.iter().for_each(|h| {
			self.SplitSegmentMap.insert(h.clone(), ID);
		});

		self.SplitEvents
			.push_back(SplitEvent { ID, Turn:self.TurnCounter, Produced, Retrieved:0 });

		let Excess = self.SplitEvents.len().saturating_sub(SplitEventCap);
		let Evicted:Vec<usize> = self.SplitEvents.drain(..Excess).map(|Event| Event.ID).collect();

		// An evicted event can no longer be attributed, so its hashes leave
		// with it - the ring entry that explained them is gone.
		self.SplitSegmentMap.retain(|_, EventID| !Evicted.contains(EventID));

		self.AdaptChainSplitThreshold();
	}
}
