use std::collections::HashMap;

use super::{AphroditeState, RECENT_MARKERS_CAP};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarkerEntry {
	pub hash:String,
	pub ccr_type:String,
	pub size:usize,
	pub preview:String,
	pub turn:usize,
	pub center:Option<String>,
	pub meta:Option<HashMap<String, String>>,
}

impl AphroditeState {
	/// Record a compression marker.
	pub fn record_marker(&mut self, entry:MarkerEntry) {
		self.recent_markers.push_back(entry);
		// Keep the last RECENT_MARKERS_CAP markers; front eviction (fix 6)
		// keeps index 0 as the oldest survivor.
		while self.recent_markers.len() > RECENT_MARKERS_CAP {
			self.recent_markers.pop_front();
		}
	}
}
