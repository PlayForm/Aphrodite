//! Recording a tool event into the bounded ring.

use crate::State::{Const::ToolEventCap, Event::Struct::ToolEvent, Struct::AphroditeState};

impl AphroditeState {
	/// Record a per-call tool event into the bounded ring (P2/T6). Caps at
	/// `ToolEventCap` (200) entries, evicting the front (oldest) - same
	/// eviction style as `RecentMarkers`.
	///
	/// The number to drop is arithmetic on a length the ring already knows
	/// (`len - cap`), not a pop loop: one append can overfill the ring by at most
	/// one, and `drain` removes exactly the excess from the front, where the
	/// oldest events sit.
	pub fn RecordToolEvent(&mut self, Event:ToolEvent) {
		self.ToolEvents.push_back(Event);

		let Excess = self.ToolEvents.len().saturating_sub(ToolEventCap);

		let _ = self.ToolEvents.drain(..Excess);
	}
}
