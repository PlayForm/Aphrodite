//! Recording a chain-split event.
//!
//! The segment hashes must be registered in `ChainSplit::Segments` before the
//! event is pushed, because attribution (`Retrieve.rs`) looks the hash up in
//! that map. Registering first and pushing second means an interruption can
//! only lose an event, never attribute a resolve to the wrong one.
//!
//! The reverse must hold too: when the ring evicts an event, that event's hashes
//! leave the map with it. The ring is capped at `Const::Cap` and the map was
//! not, so a split whose segments were never retrieved left its hashes in the map
//! for the life of the process - the one unbounded structure in the state, growing
//! by every segment of every chain ever split.

use super::{Const::Cap, Entry::Struct::Struct as Entry, Struct::Struct};

impl Struct {
	/// Record a chain-split event (Tier 1 teaching loop): a chain's output
	/// was split into `Produced` segment markers whose hashes are registered
	/// for retrieval attribution. Bounded ring (cap `Const::Cap`), then
	/// re-adapts the split threshold from the retrieval ratio.
	///
	/// An empty batch records nothing, and the count carries that decision: the
	/// recording only exists for a batch with at least one segment, so the id, the
	/// map and the ring move together - and the event id never advances for a split
	/// that produced no segments.
	///
	/// The ring bound is `drain` over the excess rather than a pop loop: the
	/// length is known, so the number to drop is arithmetic, and the eviction
	/// happens in one call at the front where the oldest events sit. Every
	/// evicted event takes its hashes out of `ChainSplit::Segments`, so the map holds
	/// exactly the segment identities the ring can still explain.
	pub fn Record(&mut self, Turn:usize, Hashes:Vec<String>) {
		// A batch with no segments records nothing, and the count is the whole
		// decision: `NonZeroUsize` exists only when the split produced something,
		// so the id, the map and the ring move together for exactly those batches.
		let Some(Produced) = std::num::NonZeroUsize::new(Hashes.len()) else {
			return;
		};

		self.Next += 1;

		let ID = self.Next;

		Hashes.iter().for_each(|h| {
			self.Segments.insert(h.clone(), ID);
		});

		self.Ledger.push_back(Entry { ID, Turn, Produced:Produced.get(), Retrieved:0 });

		let Excess = self.Ledger.len().saturating_sub(Cap);
		let Evicted:Vec<usize> = self.Ledger.drain(..Excess).map(|Event| Event.ID).collect();

		// An evicted event can no longer be attributed, so its hashes leave
		// with it - the ring entry that explained them is gone.
		self.Segments.retain(|_, EventID| !Evicted.contains(EventID));

		self.Adapt();
	}
}
