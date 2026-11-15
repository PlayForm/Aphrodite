//! What a caller asks the retrieve endpoint for.

use serde::Deserialize;

/// Request body for `/retrieve`. `Query`/`Offset`/`Limit` narrow down a large
/// stored payload instead of returning it in full.
#[derive(Debug, Deserialize)]
pub struct Struct {
	/// Marker hash to resolve. The endpoint normalises it first, so a model
	/// that echoes the whole marker body (`hash|type|size`) still resolves.
	pub hash:Option<String>,
	/// Case-insensitive substring filter applied before pagination; `None` or
	/// an empty string returns the payload unchanged.
	#[serde(rename = "query")]
	pub Query:Option<String>,
	/// Zero-based line the window starts at.
	#[serde(default)]
	#[serde(rename = "offset")]
	pub Offset:usize,
	/// Lines to return. `0` means the FULL document - that is what makes a
	/// retrieve round-trip byte-exact - and any other value is clamped to the
	/// 10,000-line server cap.
	#[serde(default)]
	#[serde(rename = "limit")]
	pub Limit:usize,
}
