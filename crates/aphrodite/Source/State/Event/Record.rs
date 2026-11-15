//! Recording a tool event into the bounded ring.

use crate::State::{Const::ToolEventCap, Event::Struct::ToolEvent, Struct::AphroditeState};

impl AphroditeState {
	/// Record a per-call tool event into the bounded ring (P2/T6). Caps at
	/// `ToolEventCap` (200) entries, evicting the front (oldest) - same
	/// eviction style as `RecentMarkers`.
	pub fn RecordToolEvent(&mut self, Event:ToolEvent) {
		self.ToolEvents.push_back(Event);

		loop {
			match self.ToolEvents.len() > ToolEventCap {
				true => {
					self.ToolEvents.pop_front();
				},
				false => break,
			}
		}
	}
}
