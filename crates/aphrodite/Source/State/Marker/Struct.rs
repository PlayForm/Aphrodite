//! One catalog entry - the marker as later turns see it.

use std::collections::HashMap;

/// A recorded compression marker: the wire fields a catalog render needs
/// (hash, type, size, preview, turn) plus the two optional enrichments.
///
/// Serialized straight onto the `aphrodite_catalog` JSON, so the field names ARE
/// the wire keys (`ccr_type`, `size`, `preview`, `turn`, `center`) and each is
/// pinned with `#[serde(rename = ...)]` - a rename here cannot change the shape
/// the model reads.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Struct {
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
