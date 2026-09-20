use std::collections::HashMap;

use super::AphroditeState;

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
		self.recent_markers.push(entry);
		// Keep last 200 markers
		while self.recent_markers.len() > 200 {
			self.recent_markers.remove(0);
		}
	}
}
