//! One catalog entry - the marker as later turns see it.

use std::collections::HashMap;

/// A recorded compression marker: the wire fields a catalog render needs
/// (hash, type, size, preview, turn) plus the two optional enrichments.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarkerEntry {
	pub hash:String,
	pub CCRType:String,
	pub Size:usize,
	pub Preview:String,
	pub Turn:usize,
	pub Center:Option<String>,
	pub meta:Option<HashMap<String, String>>,
}
