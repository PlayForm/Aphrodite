//! Recording a marker into the catalog.

use crate::State::{Const::RecentMarkersCap, Marker::Struct::MarkerEntry, Struct::AphroditeState};

impl AphroditeState {
	/// Record a compression marker.
	pub fn RecordMarker(&mut self, Entry:MarkerEntry) {
		// DEV-ONLY debug metrics (crate::Debug): one atomic add when the
		// logger is on; a no-op single load when off.
		crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::MarkersCreated);
		self.RecentMarkers.push_back(Entry);

		// Keep the last RECENT_MARKERS_CAP markers; front eviction (fix 6)
		// keeps index 0 as the oldest survivor.
		loop {
			match self.RecentMarkers.len() > RecentMarkersCap {
				true => {
					self.RecentMarkers.pop_front();
				},
				false => break,
			}
		}
	}
}
