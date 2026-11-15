//! Recording a tool event into the bounded ring.

use crate::State::{Const::ToolEventCap, Event::Struct::ToolEvent, Struct::AphroditeState};

impl AphroditeState {
	/// Record a per-call tool event into the bounded ring (P2/T6). Caps at
	/// `ToolEventCap` (200) entries, evicting the front (oldest) - same
	/// eviction style as `RecentMarkers`.
	pub fn RecordToolEvent(&mut self, Event:ToolEvent) {
		self.ToolEvents.push_back(Event);
		while self.ToolEvents.len() > ToolEventCap {
			self.ToolEvents.pop_front();
		}
	}
}
