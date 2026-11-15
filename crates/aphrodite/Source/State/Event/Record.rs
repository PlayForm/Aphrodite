//! Recording a tool event into the bounded ring.

use crate::State::Struct::AphroditeState;
use crate::State::Const::ToolEventCap;
use crate::State::Event::Struct::ToolEvent;

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
