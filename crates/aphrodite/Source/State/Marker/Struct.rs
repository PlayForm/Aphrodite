//! One catalog entry - the marker as later turns see it.

use std::collections::HashMap;

/// A recorded compression marker: the wire fields a catalog render needs
/// (hash, type, size, preview, turn) plus the two optional enrichments.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarkerEntry {
	pub hash:String,
	#[serde(rename = "ccr_type")]
	pub CCRType:String,
	#[serde(rename = "size")]
	pub Size:usize,
	#[serde(rename = "preview")]
	pub Preview:String,
	#[serde(rename = "turn")]
	pub Turn:usize,
	#[serde(rename = "center")]
	pub Center:Option<String>,
	pub meta:Option<HashMap<String, String>>,
}
