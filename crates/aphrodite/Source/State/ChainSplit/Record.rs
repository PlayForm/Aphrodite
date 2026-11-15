//! Recording a chain-split event.
//!
//! The segment hashes must be registered in `SplitSegmentMap` before the
//! event is pushed, because attribution (`Retrieve.rs`) looks the hash up in
//! that map. Registering first and pushing second means an interruption can
//! only lose an event, never attribute a resolve to the wrong one.

use crate::State::{ChainSplit::Struct::SplitEvent, Const::SplitEventCap, Struct::AphroditeState};

impl AphroditeState {
	/// Record a chain-split event (Tier 1 teaching loop): a chain's output
	/// was split into `Produced` segment markers whose hashes are registered
	/// for retrieval attribution. Bounded ring (cap `SplitEventCap`), then
	/// re-adapts the split threshold from the retrieval ratio.
	pub fn RecordChainSplit(&mut self, Hashes:Vec<String>) {
		match Hashes.is_empty() {
			true => return,
			false => (),
		}

		self.SplitNextEventID += 1;
		let ID = self.SplitNextEventID;
		let Produced = Hashes.len();
		Hashes.iter().for_each(|h| {
			self.SplitSegmentMap.insert(h.clone(), ID);
		});
		self.SplitEvents
			.push_back(SplitEvent { ID, Turn:self.TurnCounter, Produced, Retrieved:0 });

		loop {
			match self.SplitEvents.len() > SplitEventCap {
				true => {
					self.SplitEvents.pop_front();
				},
				false => break,
			}
		}

		self.AdaptChainSplitThreshold();
	}
}
